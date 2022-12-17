pub struct Suggest {
    pub pinyin: String,
    pub hanzi: String,
    pub priority: u64,
}

impl Suggest {
    pub fn new(pinyin: String, hanzi: String, priority: u64) -> Suggest {
        Suggest {
            pinyin,
            hanzi,
            priority,
        }
    }
}

#[derive(Debug)]
pub struct Setting {
    /// 是否开启自动补全
    pub completion_on: bool,
    /// 是否补全中文字符
    pub show_symbols: bool,
    /// 是否只有在汉字后面才显示中文字符，只有 show_symbols 为 true 才生效
    pub show_symbols_only_follow_by_hanzi: bool,
    /// 是否在输入 n 遍的时候才显示中文字符，只有 show_symbols 为 true 才生效
    /// 设置为 0 则不生效
    pub show_symbols_by_n_times: u64,
    /// 自动补全是否只显示完全匹配结果
    pub match_as_same_as_input: bool,
    /// 是否自动补全长句
    pub match_long_input: bool,
    /// dict.db3 路径
    pub db_path: Option<String>,
    /// 最多显示多少补全结果
    pub max_suggest: u64,
}

impl Setting {
    pub fn new() -> Setting {
        Setting {
            completion_on: true,
            show_symbols: true,
            show_symbols_only_follow_by_hanzi: false,
            show_symbols_by_n_times: 0,
            match_as_same_as_input: false,
            match_long_input: true,
            db_path: None,
            max_suggest: 50,
        }
    }
}
