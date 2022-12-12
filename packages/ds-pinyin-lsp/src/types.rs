pub struct Suggest {
    pub pinyin: String,
    pub hanzi: String,
    pub priority: usize,
}

impl Suggest {
    pub fn new(pinyin: String, hanzi: String, priority: usize) -> Suggest {
        Suggest {
            pinyin,
            hanzi,
            priority,
        }
    }
}

#[derive(Debug)]
pub struct Setting {
    pub completion_on: bool,
    pub show_symbols: bool,
    pub match_as_same_as_input: bool,
    pub match_long_input: bool,
    pub db_path: Option<String>,
}

impl Setting {
    pub fn new() -> Setting {
        Setting {
            completion_on: true,
            show_symbols: true,
            match_as_same_as_input: false,
            match_long_input: true,
            db_path: None,
        }
    }
}
