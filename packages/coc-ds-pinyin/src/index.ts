/*
 * https://github.com/fannheyward/coc-rust-analyzer/blob/master/src/index.ts
 */
import { ExtensionContext, window, workspace } from 'coc.nvim';
import { existsSync, mkdirSync } from 'fs';
import { join } from 'path';
import { dbName, dbTag, extensionName } from './constant';
import { Ctx } from './ctx';
import { downloadServer, getLatestRelease } from './downloader';

export async function activate(context: ExtensionContext): Promise<void> {
  const config = workspace.getConfiguration(extensionName);
  const isEnabled = config.get<boolean>('enabled', true);

  // if not enabled then return
  if (!isEnabled) {
    return;
  }

  const serverRoot = context.storagePath;
  if (!existsSync(serverRoot)) {
    mkdirSync(serverRoot);
  }

  const ctx = new Ctx(context);

  const bin = ctx.resolveBin();

  if (!bin) {
    let msg = `${extensionName} is not found, download from GitHub release?`;
    let ret = -1;
    if (config.get('prompt', true)) {
      ret = await window.showQuickpick(['Yes', 'Cancel'], msg);
    }
    if (ret === 0) {
      try {
        const latest = await getLatestRelease(extensionName);
        if (!latest) throw new Error('Failed to get latest release');
        await downloadServer(context, latest);
      } catch (e) {
        console.error(e);
        msg = `Download ${extensionName} failed, you can get it from https://github.com/iamcco/${extensionName}`;
        window.showErrorMessage(msg);
        return;
      }
    } else {
      return;
    }
  }

  if (!config.get('db_path') && !existsSync(join(context.storagePath, dbName))) {
    let msg = `${dbName} is not found, download from GitHub release?`;
    let ret = -1;
    if (config.get('prompt', true)) {
      ret = await window.showQuickpick(['Yes', 'Cancel'], msg);
    }
    if (ret === 0) {
      try {
        const latest = await getLatestRelease('db', dbTag);
        if (!latest) throw new Error('Failed to get latest release');
        await downloadServer(context, latest, true);
      } catch (e) {
        console.error(e);
        msg = `Download ${dbName} failed, you can get it from https://github.com/iamcco/${extensionName}`;
        window.showErrorMessage(msg);
        return;
      }
    } else {
      return;
    }
  }

  await ctx.startServer();
  if (bin) await ctx.checkUpdate();
  if (existsSync(join(context.storagePath, dbName))) await ctx.checkUpdate('db');
}
