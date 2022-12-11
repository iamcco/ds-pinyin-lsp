/*
 * https://github.com/fannheyward/coc-rust-analyzer/blob/master/src/downloader.ts
 */
import { exec } from 'child_process';
import { ExtensionContext, window } from 'coc.nvim';
import { randomBytes } from 'crypto';
import { createWriteStream, PathLike, promises as fs } from 'fs';
import { HttpsProxyAgent } from 'https-proxy-agent';
import fetch from 'node-fetch';
import path from 'path';
import stream from 'stream';
import util from 'util';
import extractZip from 'extract-zip';
import { dbName, extensionName } from './constant';

const pipeline = util.promisify(stream.pipeline);
const agent = process.env.https_proxy ? new HttpsProxyAgent(process.env.https_proxy as string) : undefined;

async function patchelf(dest: PathLike): Promise<void> {
  const expression = `
{src, pkgs ? import <nixpkgs> {}}:
    pkgs.stdenv.mkDerivation {
        name = "${extensionName}";
        inherit src;
        phases = [ "installPhase" "fixupPhase" ];
        installPhase = "cp $src $out";
        fixupPhase = ''
        chmod 755 $out
        patchelf --set-interpreter "$(cat $NIX_CC/nix-support/dynamic-linker)" $out
        '';
    }
`;
  const origFile = dest + '-orig';
  await fs.rename(dest, origFile);

  await new Promise((resolve, reject) => {
    const handle = exec(`nix-build -E - --arg src '${origFile}' -o ${dest}`, (err, stdout, stderr) => {
      // lgtm[js/shell-command-constructed-from-input]
      if (err != null) {
        reject(Error(stderr));
      } else {
        resolve(stdout);
      }
    });
    handle.stdin?.write(expression);
    handle.stdin?.end();
  });

  await fs.unlink(origFile);
}

interface Asset {
  name: string;
  browser_download_url: string;
}

interface GithubRelease {
  tag_name: string;
  published_at: string;
  assets: Array<Asset>;
}

export interface ReleaseTag {
  tag: string;
  url: string;
  name: string;
  asset?: Asset;
}

export function getPlatform(): string | undefined {
  const platforms: { [key: string]: string } = {
    'ia32 win32': 'x86_64-pc-windows-gnu',
    'x64 win32': 'x86_64-pc-windows-gnu',
    'x64 linux': 'x86_64-unknown-linux-musl',
    'x64 darwin': 'x86_64-apple-darwin',
    'arm64 darwin': 'aarch64-apple-darwin',
  };

  return platforms[`${process.arch} ${process.platform}`];
}

export async function getLatestRelease(
  type: 'db' | typeof extensionName = extensionName,
  releaseTag?: string,
): Promise<ReleaseTag | undefined> {
  const releaseURL = !releaseTag
    ? `https://api.github.com/repos/iamcco/${extensionName}/releases/latest`
    : `https://api.github.com/repos/iamcco/${extensionName}/releases/tags/${releaseTag}`;
  const response = await fetch(releaseURL, { agent });
  if (!response.ok) {
    console.error(await response.text());
    return;
  }

  const release = (await response.json()) as GithubRelease;
  const platform = getPlatform();
  if (type === extensionName && !platform) {
    console.error(`Unfortunately we don't ship binaries for your platform yet.`);
    return;
  }
  const asset = release.assets.find((val) =>
    val.browser_download_url.endsWith(`${type === 'db' ? dbName : platform}.zip`),
  );
  if (!asset) {
    console.error(`getLatestRelease failed: ${release}`);
    return;
  }

  const tag = release.tag_name;
  const name = type === 'db' ? dbName : process.platform === 'win32' ? `${extensionName}.exe` : extensionName;

  return { asset, tag, url: asset.browser_download_url, name: name };
}

export async function downloadServer(context: ExtensionContext, release: ReleaseTag, isDb = true): Promise<void> {
  const statusItem = window.createStatusBarItem(0, { progress: true });
  statusItem.text = `Downloading ${extensionName} ${release.tag}`;
  statusItem.show();

  const resp = await fetch(release.url, { agent });

  if (!resp.ok || !resp.body) {
    statusItem.hide();
    throw new Error('Download failed');
  }

  let cur = 0;
  const len = Number(resp.headers.get('content-length'));

  resp.body.on('data', (chunk: Buffer) => {
    cur += chunk.length;
    const p = ((cur / len) * 100).toFixed(2);
    statusItem.text = `${p}% Downloading ${extensionName} ${release.tag}`;
  });

  const _path = path.join(context.storagePath, release.name); // lgtm[js/shell-command-constructed-from-input]
  const randomHex = randomBytes(5).toString('hex');
  const tempFile = path.join(context.storagePath, `${release.name}${randomHex}`);

  const destFileStream = createWriteStream(tempFile, { mode: 0o755 });

  await pipeline(resp.body, destFileStream);

  await fs.unlink(_path).catch((err) => {
    if (err.code !== 'ENOENT') throw err;
  });

  await extractZip(tempFile, {
    dir: context.storagePath,
  });

  await fs.unlink(tempFile).catch((err) => {
    if (err.code !== 'ENOENT') throw err;
  });

  await new Promise<void>((resolve) => {
    destFileStream.on('close', resolve);
    destFileStream.destroy();
    setTimeout(resolve, 1000);
  });

  await context.globalState.update(isDb ? 'release-db' : 'release', release.tag);

  if (isDb) {
    statusItem.hide();
    return;
  }

  try {
    if (await fs.stat('/etc/nixos')) {
      statusItem.text = `Patching ${extensionName} executable...`;
      await patchelf(_path);
    }
  } catch (e) {}

  statusItem.hide();
}
