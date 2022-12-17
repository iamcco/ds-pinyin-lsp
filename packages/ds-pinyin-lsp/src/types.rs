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
            match_as_same_as_input: false,
            match_long_input: true,
            db_path: None,
            max_suggest: 50,
        }
    }
}
