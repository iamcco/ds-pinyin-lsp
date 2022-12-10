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
    pub db_path: Option<String>,
}

impl Setting {
    pub fn new() -> Setting {
        Setting {
            completion_on: true,
            db_path: None,
        }
    }
}
