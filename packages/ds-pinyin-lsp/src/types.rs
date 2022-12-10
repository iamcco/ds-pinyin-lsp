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
    pub db_path: String,
}

impl Setting {
    pub fn new(db_path: String) -> Setting {
        Setting { db_path }
    }
}
