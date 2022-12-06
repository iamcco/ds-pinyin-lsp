# Dead Simple Pinyin Language Server

![](https://user-images.githubusercontent.com/5492542/205504265-0875046a-aab4-4672-9467-048cd43460a9.png)

[中文](./README.md) English

## Introduction

Dead simple Pinyin language server for input Chinese without IME. (Main for (neo)vim environment)

> Current only support **全拼(Quanpin)**

### Packages

- [dict-builder](./packages/dict-builder) script to build `dict.db3`
- [ds-pinyin-lsp](./packages/ds-pinyin-lsp) the pinyin language server
- [coc-ds-pinyin-lsp](./packages/coc-ds-pinyin) extension for coc.nvim

## Using with coc.nvim

Add `"suggest.asciiCharactersOnly": true,` option to `coc-settings.json`

Using extension:

```
:CocInstall coc-ds-pinyin-lsp
```

Or add config to coc-settings.json

``` jsonc
  "languageserver": {
    "ds-pinyin": {
      "command": "path to ds-pinyin-lsp command",
      "filetypes": ["*"],
      "initializationOptions": {
        "db-path": "path to dict.db3"
      }
    }
  }
```

## Dict data

All dict data from [rime-ice](https://github.com/iDvel/rime-ice)

### Buy Me A Coffee ☕️

![btc](https://img.shields.io/keybase/btc/iamcco.svg?style=popout-square)

![image](https://user-images.githubusercontent.com/5492542/42771079-962216b0-8958-11e8-81c0-520363ce1059.png)
