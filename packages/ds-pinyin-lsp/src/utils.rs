use std::error::Error;

use dashmap::mapref::one::Ref;
use lsp_document::{IndexedText, TextAdapter, TextMap};
use regex::Regex;
use rusqlite::Connection;
use tower_lsp::lsp_types::{
    CompletionItem, CompletionItemKind, CompletionTextEdit, Position, Range, TextEdit,
};

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
            "SELECT pinyin, hanzi, priority FROM words WHERE pinyin = '{}' ORDER BY priority DESC",
            pinyin
        )
    } else {
        format!(
            "SELECT pinyin, hanzi, priority FROM words WHERE pinyin BETWEEN '{}' AND '{}{{' ORDER BY priority DESC limit 50",
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
        "SELECT pinyin, hanzi, priority FROM dict WHERE pinyin BETWEEN '{}' AND '{}{{' ORDER BY priority DESC limit 50",
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

pub fn query_the_longest_match<'a>(
    conn: &Connection,
    pinyin: &'a str,
) -> Result<Option<(&'a str, Suggest)>, Box<dyn Error>> {
    for i in 1..=pinyin.len() {
        let sub_pinyin = &pinyin[0..=pinyin.len() - i];
        let stmt =
            format!( "SELECT pinyin, hanzi, priority FROM dict WHERE pinyin = '{}' ORDER BY priority DESC limit 1", sub_pinyin);
        let mut stmt = conn.prepare(&stmt)?;

        let row_iter = stmt.query_map([], |row| {
            Ok(Suggest::new(row.get(0)?, row.get(1)?, row.get(2)?))
        })?;

        for row in row_iter {
            if let Ok(row) = row {
                return Ok(Some((sub_pinyin, row)));
            }
        }
    }

    for i in 1..=pinyin.len() {
        let sub_pinyin = &pinyin[0..=pinyin.len() - i];
        let stmt =
            format!( "SELECT pinyin, hanzi, priority FROM words WHERE pinyin = '{}' ORDER BY priority DESC limit 1", sub_pinyin);
        let mut stmt = conn.prepare(&stmt)?;

        let row_iter = stmt.query_map([], |row| {
            Ok(Suggest::new(row.get(0)?, row.get(1)?, row.get(2)?))
        })?;

        for row in row_iter {
            if let Ok(row) = row {
                return Ok(Some((sub_pinyin, row)));
            }
        }
    }

    for i in 1..=pinyin.len() {
        let sub_pinyin = &pinyin[0..=pinyin.len() - i];
        let stmt =
            format!(
                "SELECT pinyin, hanzi, priority FROM dict WHERE pinyin BETWEEN '{}' AND '{}{{' ORDER BY priority DESC limit 1",
                sub_pinyin, sub_pinyin
            );
        let mut stmt = conn.prepare(&stmt)?;

        let row_iter = stmt.query_map([], |row| {
            Ok(Suggest::new(row.get(0)?, row.get(1)?, row.get(2)?))
        })?;

        for row in row_iter {
            if let Ok(row) = row {
                return Ok(Some((sub_pinyin, row)));
            }
        }
    }

    Ok(None)
}

pub fn query_long_sentence(
    conn: &Connection,
    pinyin: &str,
) -> Result<Option<Vec<Suggest>>, Box<dyn Error>> {
    let mut res = vec![];

    let mut remain = pinyin.to_string();

    while remain.len() > 0 {
        if let Ok(Some((match_pinyin, suggest))) = query_the_longest_match(conn, &remain) {
            res.push(suggest);
            remain = Regex::new(&format!("^{}", match_pinyin))
                .unwrap()
                .replace(&remain, "")
                .to_string();
        } else {
            return Ok(None);
        }
    }

    Ok(Some(res))
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

pub fn long_suggest_to_completion_item(suggest: Vec<Suggest>, range: Range) -> Vec<CompletionItem> {
    let hanzi = suggest
        .iter()
        .map(|s| s.hanzi.clone())
        .collect::<Vec<String>>()
        .join("");

    let pinyin = suggest
        .iter()
        .map(|s| s.pinyin.clone())
        .collect::<Vec<String>>()
        .join("");

    let sentence = vec![CompletionItem {
        label: hanzi.to_string(),
        kind: Some(CompletionItemKind::TEXT),
        filter_text: Some(pinyin.clone()),
        // use text_edit here to avoid client's replace mode
        // it's no need to replace words behind cursor
        text_edit: Some(CompletionTextEdit::Edit(TextEdit::new(range, hanzi))),
        ..Default::default()
    }];

    if suggest.len() > 1 {
        return vec![
            sentence,
            suggest
                .into_iter()
                .map(|s| CompletionItem {
                    label: s.hanzi.to_string(),
                    kind: Some(CompletionItemKind::TEXT),
                    filter_text: Some(pinyin.clone()),
                    // use text_edit here to avoid client's replace mode
                    // it's no need to replace words behind cursor
                    text_edit: Some(CompletionTextEdit::Edit(TextEdit::new(
                        range.clone(),
                        s.hanzi,
                    ))),
                    ..Default::default()
                })
                .collect::<Vec<CompletionItem>>(),
        ]
        .concat();
    }

    sentence
}

pub fn suggest_to_completion_item(suggest: Vec<Suggest>, range: Range) -> Vec<CompletionItem> {
    suggest
        .into_iter()
        .map(|s| CompletionItem {
            label: s.hanzi.to_string(),
            kind: Some(CompletionItemKind::TEXT),
            filter_text: Some(s.pinyin),
            // use text_edit here to avoid client's replace mode
            // it's no need to replace words behind cursor
            text_edit: Some(CompletionTextEdit::Edit(TextEdit::new(
                range.clone(),
                s.hanzi,
            ))),
            ..Default::default()
        })
        .collect::<Vec<CompletionItem>>()
}

#[cfg(test)]
pub mod test_utils {
    use rusqlite::Connection;

    use crate::utils::query_words;

    use super::{get_pinyin, query_long_sentence};

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
