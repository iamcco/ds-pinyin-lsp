pub mod types;
pub mod utils;

#[cfg(test)]
pub mod test {
    use rusqlite::Connection;

    use crate::utils::query_words;

    #[test]
    fn test_query_words() {
        let conn = Connection::open(
            "/Users/aioiyuuko/develop/pinyin-lsp/packages/dict-builder/dicts/dict.db3",
        )
        .expect("Open Connection");
        if let Ok(suggest) = query_words(&conn, "ni", true) {
            assert!(suggest.len() > 0);
        }
    }
}
