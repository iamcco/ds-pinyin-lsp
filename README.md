# 超，超简单的拼音输入法

![](https://user-images.githubusercontent.com/5492542/206855944-7be15fa8-e2eb-4325-97f9-e2a33c07f6c7.png)

中文 [English](./README-En.md)

## 介绍

通过 LSP 实现的超简单拼音输入法，其主要的用途是在 (neo)vim 编辑器中不需要切换输入法也能输入中文。
避免忘记切换输入法而导致在 Normal 模式下弹出输入法的蛋疼问题。

**注意**

- 非专业输入法，不是输入法的代替品，只适合少量需要输入中文的场景。
- 只支持**全拼**， 需要配合 LSP 客户端使用，比如 coc.nvim / VS Code 等。

## 配合 coc.nvim 使用

需要在 `coc-settings.json` 配置中启用 `"suggest.asciiCharactersOnly": true,` 设置。

> 如果不启用这个设置，那么在中文字符后面输入拼音会得不到建议选项。

使用扩展 [coc-ds-pinyin-lsp](./packages/coc-ds-pinyin)

```
:CocInstall coc-ds-pinyin-lsp
```

或者可以添加以下配置到 `coc-settings.json`

``` jsonc
  "languageserver": {
    "ds-pinyin": {
      "command": "path to ds-pinyin-lsp command",
      "filetypes": ["*"],
      "initializationOptions": {
        "db_path": "path to dict.db3",
        "completion_on": true
      }
    }
  }
```

> `dict.db3` 可以从 [Release](https://github.com/iamcco/ds-pinyin-lsp/releases/tag/v0.1.0) 下载。

可以通过向服务端发送通知（Notification）来关闭/开启/切换自动补全

- `$/turn/completion`: 参数: `{ completion_on?: boolean }`

## Packages

- [dict-builder](./packages/dict-builder) 用来构建 `dict.db3`
- [ds-pinyin-lsp](./packages/ds-pinyin-lsp) lsp 实现
- [coc-ds-pinyin-lsp](./packages/coc-ds-pinyin) coc.nvim 扩展

## 关于使用的字典

所使用的字典来自 [rime-ice](https://github.com/iDvel/rime-ice) 项目

### 请我吃个煎饼馃子 🤟

![btc](https://img.shields.io/keybase/btc/iamcco.svg?style=popout-square)

![image](https://user-images.githubusercontent.com/5492542/42771079-962216b0-8958-11e8-81c0-520363ce1059.png)
