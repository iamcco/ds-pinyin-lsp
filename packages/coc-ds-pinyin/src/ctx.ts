/*
 * https://github.com/fannheyward/coc-rust-analyzer/blob/master/src/ctx.ts
 */
import { commands, Disposable, ExtensionContext, LanguageClient, window, workspace } from 'coc.nvim';
import { existsSync } from 'fs';
import { homedir } from 'os';
import { join } from 'path';
import which from 'which';
import { extensionName } from './constant';
import { downloadServer, getLatestRelease } from './downloader';

export class Ctx {
  client!: LanguageClient;

  constructor(private readonly extCtx: ExtensionContext) {
    if (workspace.getConfiguration(extensionName).get('show-status-bar')) {
      const statusBar = window.createStatusBarItem(0);
      statusBar.text = 'Pinyin';
      statusBar.show();
      this.extCtx.subscriptions.push(statusBar);
    }
  }

  async startServer() {
    const bin = this.resolveBin();
    if (!bin) {
      return;
    }

    const client = new LanguageClient(
      extensionName,
      `${extensionName} server`,
      {
        command: bin,
      },
      {
        documentSelector: ['*'],
        initializationOptions: {
          'db-path': workspace.getConfiguration(extensionName).get('db-path', ''),
        },
      },
    );

    this.client = client;

    this.extCtx.subscriptions.push({
      dispose: () => {
        if (this.client) {
          this.client.stop();
        }
      },
    });

    this.client.start();
  }

  async stopServer() {
    if (this.client) {
      await this.client.stop();
    }
  }

  get subscriptions(): Disposable[] {
    return this.extCtx.subscriptions;
  }

  resolveBin(): string | undefined {
    // 1. from config, custom server path
    // 2. bundled
    let bin = join(this.extCtx.storagePath, process.platform === 'win32' ? `${extensionName}.exe` : extensionName);
    if (!existsSync(bin)) {
      bin = workspace.getConfiguration(extensionName).get<string>('server-path', '');

      if (bin) {
        if (bin?.startsWith('~/')) {
          bin = bin.replace('~', homedir());
        }

        bin = which.sync(bin, { nothrow: true }) || bin;
      }
    }

    if (!bin) {
      return;
    }

    return bin;
  }

  async checkUpdate(auto = true) {
    const config = workspace.getConfiguration(extensionName);
    if (config.get('server-path')) {
      // no update checking if using custom server
      return;
    }

    if (auto && !config.get('checkOnStartup')) {
      return;
    }

    const latest = await getLatestRelease();
    if (!latest) {
      return;
    }

    const old = this.extCtx.globalState.get('release') || 'unknown release';
    if (old === latest.tag) {
      if (!auto) {
        window.showInformationMessage(`Your ${extensionName} release is updated`);
      }
      return;
    }

    const msg = `${extensionName} has a new release: ${latest.tag}, you're using ${old}. Would you like to download from GitHub`;
    let ret = 0;
    if (config.get('prompt', true)) {
      ret = await window.showQuickpick(
        ['Yes, download the latest ${extensionName}', 'Check GitHub releases', 'Cancel'],
        msg,
      );
    }
    if (ret === 0) {
      if (process.platform === 'win32') {
        await this.client.stop();
      }
      try {
        await downloadServer(this.extCtx, latest);
      } catch (e) {
        console.error(e);
        let msg = `Upgrade ${extensionName} failed, please try again`;
        const err = e as any;
        if (err.code === 'EBUSY' || err.code === 'ETXTBSY' || err.code === 'EPERM') {
          msg = `Upgrade ${extensionName} failed, other Vim instances might be using it, you should close them and try again`;
        }
        window.showInformationMessage(msg, 'error');
        return;
      }
      await this.client.stop();
      this.client.start();

      this.extCtx.globalState.update('release', latest.tag);
    } else if (ret === 1) {
      await commands.executeCommand('vscode.open', `https://github.com/iamcco/${extensionName}/releases`).catch(() => {
        //
      });
    }
  }
}
