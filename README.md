# Dead Simple Pinyin Language Server

> Dead simple pinyin language server implement for input Chinese with no IME enable

## LSP client setting

### coc.nvim

``` jsonc
  "languageserver": {
    "ds-pinyin": {
      "command": "path to ds-pinyin-lsp command",
      "filetypes": ["*"],
      "settings": {
        "db_path": "path to dict.db3"
      }
    }
  }
```

## Dict data

- [rime-ice](https://github.com/iDvel/rime-ice)
