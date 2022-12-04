use std::error::Error;

use dashmap::mapref::one::Ref;
use lsp_document::{IndexedText, TextAdapter, TextMap};
use regex::Regex;
use rusqlite::Connection;
use tower_lsp::lsp_types::{Position, Range};

use crate::types::Suggest;

pub fn get_pre_line<'a>(
    document: &'a Option<Ref<String, IndexedText<String>>>,
    position: &Position,
) -> Option<&'a str> {
    if let Some(ref document) = document {
        if let Some(range) = document.lsp_range_to_range(&Range {
            start: Position {
                line: position.line,
                character: 0,
            },
            end: Position {
                line: position.line,
                character: position.character,
            },
        }) {
            return document.substr(range);
        }
    }

    None
}

pub fn query_words(
    conn: &Connection,
    pinyin: &str,
    eq: bool,
) -> Result<Vec<Suggest>, Box<dyn Error>> {
    let stmt = if eq {
        format!(
            "SELECT pinyin, hanzi, priority FROM words WHERE pinyin = '{}'",
            pinyin
        )
    } else {
        format!(
            "SELECT pinyin, hanzi, priority FROM words WHERE pinyin BETWEEN '{}' AND '{}{{' limit 50",
            pinyin, pinyin
        )
    };
    let mut stmt = conn.prepare(&stmt)?;

    let row_iter = stmt.query_map([], |row| {
        Ok(Suggest::new(row.get(0)?, row.get(1)?, row.get(2)?))
    })?;

    let mut res = vec![];

    for row in row_iter {
        if let Ok(row) = row {
            res.push(row);
        }
    }

    Ok(res)
}

pub fn query_dict(conn: &Connection, pinyin: &str) -> Result<Vec<Suggest>, Box<dyn Error>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT pinyin, hanzi, priority FROM dict WHERE pinyin BETWEEN '{}' AND '{}{{' limit 50",
        pinyin, pinyin
    ))?;

    let row_iter = stmt.query_map([], |row| {
        Ok(Suggest::new(row.get(0)?, row.get(1)?, row.get(2)?))
    })?;

    let mut res = vec![];

    for row in row_iter {
        if let Ok(row) = row {
            res.push(row);
        }
    }

    Ok(res)
}

pub fn get_pinyin<'a>(pre_line: &'a str) -> Option<String> {
    if pre_line.is_empty() {
        return None;
    }
    let regex = Regex::new(r"(?P<pinyin>[a-zA-Z]+)$").unwrap();
    if let Some(m) = regex.captures(pre_line) {
        return Some(m["pinyin"].to_string());
    }
    None
}

#[cfg(test)]
pub mod test_utils {
    use rusqlite::Connection;

    use crate::utils::query_words;

    use super::get_pinyin;

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
}
