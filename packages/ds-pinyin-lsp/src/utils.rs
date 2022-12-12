use std::error::Error;

use dashmap::mapref::one::Ref;
use lsp_document::{IndexedText, TextAdapter, TextMap};
use regex::Regex;
use rusqlite::Connection;
use tower_lsp::lsp_types::{
    CompletionItem, CompletionItemKind, CompletionTextEdit, Position, Range, TextEdit,
};

use crate::{sqlite::query_the_longest_match, types::Suggest};

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

pub fn query_long_sentence(
    conn: &Connection,
    pinyin: &str,
    match_as_same_as_input: bool,
) -> Result<Option<Vec<Suggest>>, Box<dyn Error>> {
    let mut res = vec![];

    let mut remain = pinyin.to_string();

    while remain.len() > 0 {
        if let Ok(Some((match_pinyin, suggests))) =
            query_the_longest_match(conn, &remain, match_as_same_as_input)
        {
            res.push(suggests);
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

pub fn long_suggests_to_completion_item(
    suggests: Vec<Suggest>,
    range: Range,
) -> Vec<CompletionItem> {
    let hanzi = suggests
        .iter()
        .map(|s| s.hanzi.clone())
        .collect::<Vec<String>>()
        .join("");

    let pinyin = suggests
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

    if suggests.len() > 1 {
        return vec![
            sentence,
            suggests
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

pub fn suggests_to_completion_item(suggests: Vec<Suggest>, range: Range) -> Vec<CompletionItem> {
    suggests
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

pub fn symbols_to_completion_item(
    symbol: char,
    symbols: Ref<char, Vec<String>>,
    position: Position,
) -> Vec<CompletionItem> {
    symbols
        .iter()
        .map(|s| CompletionItem {
            label: s.clone(),
            kind: Some(CompletionItemKind::OPERATOR),
            filter_text: Some(symbol.to_string()),
            // use text_edit here to avoid client's replace mode
            // it's no need to replace words behind cursor
            text_edit: Some(CompletionTextEdit::Edit(TextEdit::new(
                Range::new(
                    Position {
                        line: position.line,
                        character: position.character - 1,
                    },
                    position,
                ),
                s.clone(),
            ))),
            ..Default::default()
        })
        .collect::<Vec<CompletionItem>>()
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

    use super::{get_pinyin, query_long_sentence};

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
        if let Ok(Some(suggests)) = query_long_sentence(&conn, "nihaonishishui", true) {
            assert_eq!(
                suggests
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
