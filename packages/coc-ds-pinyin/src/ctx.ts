/*
 * https://github.com/fannheyward/coc-rust-analyzer/blob/master/src/ctx.ts
 */
import { commands, Disposable, ExtensionContext, LanguageClient, window, workspace } from 'coc.nvim';
import { existsSync } from 'fs';
import { homedir } from 'os';
import { join } from 'path';
import which from 'which';
import { dbName, dbTag, extensionName } from './constant';
import { downloadServer, getLatestRelease } from './downloader';

export class Ctx {
  client!: LanguageClient;

  constructor(private readonly extCtx: ExtensionContext) {
    if (workspace.getConfiguration(extensionName).get('show_status_bar')) {
      const statusBar = window.createStatusBarItem(0);
      statusBar.text = workspace.getConfiguration(extensionName).get('status_bar_flag', 'Pinyin');
      statusBar.show();
      this.extCtx.subscriptions.push(statusBar);
    }

    this.subscriptions.push(
      commands.registerCommand('ds-pinyin-lsp.turn-on-completion', () => {
        if (!this.client) {
          return;
        }
        this.client.sendNotification('$/turn/completion', { completion_on: true });
      }),
      commands.registerCommand('ds-pinyin-lsp.turn-off-completion', () => {
        if (!this.client) {
          return;
        }
        this.client.sendNotification('$/turn/completion', { completion_on: false });
      }),
      commands.registerCommand('ds-pinyin-lsp.toggle-completion', () => {
        if (!this.client) {
          return;
        }
        this.client.sendNotification('$/turn/completion', {});
      }),
    );
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
        initializationOptions: this.configuration,
      },
    );

    this.client = client;

    this.extCtx.subscriptions.push(
      workspace.onDidChangeConfiguration((evt) => {
        if (!evt.affectsConfiguration(extensionName) || !this.client?.started) {
          return;
        }
        this.client.sendNotification('workspace/didChangeConfiguration', {
          settings: this.configuration,
        });
      }),
      {
        dispose: () => {
          if (this.client) {
            this.client.stop();
          }
        },
      },
    );

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

  get configuration() {
    const config = workspace.getConfiguration(extensionName);
    const db_path = join(this.extCtx.storagePath, dbName);
    return {
      db_path: config.get<string>('db_path') || (existsSync(db_path) ? db_path : ''),
      completion_on: config.get<boolean>('completion_on', true),
      completion_around_mode: config.get('completion_around_mode', false),
      completion_trigger_characters: config.get('completion_trigger_characters', ''),
      show_symbols: config.get<boolean>('show_symbols', true),
      show_symbols_only_follow_by_hanzi: config.get<boolean>('show_symbols_only_follow_by_hanzi', false),
      show_symbols_by_n_times: config.get<number>('show_symbols_by_n_times', 0),
      match_as_same_as_input: config.get<boolean>('match_as_same_as_input', false),
      match_long_input: config.get<boolean>('match_long_input', true),
      max_suggest: config.get<number>('max_suggest', 50),
    };
  }

  resolveBin(): string | undefined {
    // 1. from config, custom server path
    // 2. bundled
    let bin = workspace.getConfiguration(extensionName).get<string>('server_path', '');

    if (bin) {
      if (bin?.startsWith('~/')) {
        bin = bin.replace('~', homedir());
      }

      bin = which.sync(bin, { nothrow: true }) || bin;
    }

    if (!bin) {
      bin = join(this.extCtx.storagePath, process.platform === 'win32' ? `${extensionName}.exe` : extensionName);
    }

    if (!existsSync(bin)) {
      return;
    }

    return bin;
  }

  async checkUpdate(type: 'db' | typeof extensionName = extensionName, auto = true) {
    const config = workspace.getConfiguration(extensionName);
    if (config.get('server_path')) {
      // no update checking if using custom server
      return;
    }

    if (auto && !config.get('check_on_startup')) {
      return;
    }

    const latest = await getLatestRelease(type, type === 'db' ? dbTag : undefined);
    if (!latest) {
      return;
    }

    const old = this.extCtx.globalState.get(type === 'db' ? 'release-db' : 'release') || 'unknown release';
    if (old === latest.tag) {
      if (!auto) {
        window.showInformationMessage(`Your ${type} release is updated`);
      }
      return;
    }

    const msg = `${type} has a new release: ${latest.tag}, you're using ${old}. Would you like to download from GitHub`;
    let ret = 0;
    if (config.get('prompt', true)) {
      ret = await window.showQuickpick([`Yes, download the latest ${type}`, 'Check GitHub releases', 'Cancel'], msg);
    }
    if (ret === 0) {
      if (process.platform === 'win32') {
        await this.client.stop();
      }
      try {
        await downloadServer(this.extCtx, latest, type === 'db');
      } catch (e) {
        console.error(e);
        let msg = `Upgrade ${type} failed, please try again`;
        const err = e as any;
        if (err.code === 'EBUSY' || err.code === 'ETXTBSY' || err.code === 'EPERM') {
          msg = `Upgrade ${type} failed, other Vim instances might be using it, you should close them and try again`;
        }
        window.showInformationMessage(msg, 'error');
        return;
      }
      if (type === extensionName) {
        await this.client.stop();
        this.client.start();
      }

      this.extCtx.globalState.update(type === 'db' ? 'release-db' : 'release', latest.tag);
    } else if (ret === 1) {
      await commands.executeCommand('vscode.open', `https://github.com/iamcco/${extensionName}/releases`).catch(() => {
        //
      });
    }
  }
}
