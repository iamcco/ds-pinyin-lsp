pub mod types;
pub mod utils;

#[cfg(test)]
pub mod test_utils {
    use rusqlite::Connection;

    use crate::utils::{get_pinyin, query_long_sentence, query_words};

    #[test]
    fn test_query_words() {
        let conn = Connection::open("../dict-builder/dicts/dict.db3").expect("Open Connection");
        if let Ok(suggest) = query_words(&conn, "ni", true) {
            assert!(suggest.len() > 0);
        }
    }

    #[test]
    fn test_get_pinyin() {
        assert_eq!(
            get_pinyin("hello world nihao").expect("get pinyin nihao"),
            "nihao"
        );
    }

    #[test]
    fn test_query_long_sentence() {
        let conn = Connection::open("../dict-builder/dicts/dict.db3").expect("Open Connection");
        if let Ok(Some(suggest)) = query_long_sentence(&conn, "nihaonishishui") {
            assert_eq!(
                suggest
                    .into_iter()
                    .map(|s| s.hanzi)
                    .collect::<Vec<String>>()
                    .join(""),
                "你好你是谁"
            );
        } else {
            panic!("query_long_sentence should match words");
        }
    }
}
