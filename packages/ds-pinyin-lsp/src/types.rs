pub struct Suggest {
    pub pinyin: String,
    pub hanzi: String,
    pub priority: u32,
}

impl Suggest {
    pub fn new(pinyin: String, hanzi: String, priority: u32) -> Suggest {
        Suggest {
            pinyin,
            hanzi,
            priority,
        }
    }
}
