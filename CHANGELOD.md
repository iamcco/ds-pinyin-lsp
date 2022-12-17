# 2022-12-17 v0.3.0

LSP 新增设置项

- `show_symbols_only_follow_by_hanzi`: 是否只在中文后面补全字符
- `show_symbols_by_n_times`: 是否在输入 `n` 次字符后才显示字符补全选项
- `max_suggest`: 中文补全列表最大显示个数

COC.nvim 插件新增设置项

- `ds-pinyin-lsp.show_symbols_only_follow_by_hanzi`: 是否只在中文后面补全字符
- `ds-pinyin-lsp.show_symbols_by_n_times`: 是否在输入 `n` 次字符后才显示字符补全选项
- `ds-pinyin-lsp.max_suggest`: 中文补全列表最大显示个数

# 2022-12-15 v0.2.0

LSP 新增设置项

- `show_symbols`: 是否补全中文标点符号
- `match_as_same_as_input`: 是否只显示完全匹配结果，比如: 输入 `pinyin` 会只显示 `拼音` 选项，不会显示 `拼音输入法` 等选项
- `match_long_input`: 是否显示长句匹配，比如：输入 `nihaonishishei` 在没有补全项的时候会把 `你好` `你是谁` 两个选项拼起来作为补全选项

COC.nvim 插件新增设置项

- `ds-pinyin-lsp.show_symbols`: 是否补全中文标点符号
- `ds-pinyin-lsp.match_as_same_as_input`: 是否只显示完全匹配结果，比如: 输入 `pinyin` 会只显示 `拼音` 选项，不会显示 `拼音输入法` 等选项
- `ds-pinyin-lsp.match_long_input`: 是否显示长句匹配，比如：输入 `nihaonishishei` 在没有补全项的时候会把 `你好` `你是谁` 两个选项拼起来作为补全选项
