use std::error::Error;

use rusqlite::Connection;

use crate::types::{QueryResult, Suggest};

/// query suggest
fn query_suggests(conn: &Connection, query: &str) -> QueryResult {
    let mut stmt = conn.prepare(query)?;

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

/// query dict
pub fn query_dict(
    conn: &Connection,
    pinyin: &str,
    size: u64,
    match_as_same_as_input: bool,
) -> QueryResult {
    let mut suggests = query_match_dict(conn, pinyin, size)?;

    let len = suggests.len() as u64;
    if !match_as_same_as_input && len < size {
        let mut res = query_start_match_dict(conn, pinyin, size - len)?;
        suggests.append(&mut res);
    }

    Ok(suggests)
}

/// query match in dict table
pub fn query_match_dict(conn: &Connection, pinyin: &str, size: u64) -> QueryResult {
    query_suggests(
        conn,
        &format!(
            "SELECT pinyin, hanzi, priority FROM dict WHERE pinyin = '{}' ORDER BY priority DESC limit {}",
            pinyin, size
        )
     )
}

/// query start match in dict table
pub fn query_start_match_dict(conn: &Connection, pinyin: &str, size: u64) -> QueryResult {
    query_suggests(
        conn,
        &format!(
            "SELECT pinyin, hanzi, priority FROM dict WHERE pinyin != '{}' and pinyin BETWEEN '{}' AND '{}{{' ORDER BY priority DESC limit {}",
            pinyin, pinyin, pinyin, size
        )
     )
}

/// query the longest match of the pinyin
pub fn query_the_longest_match<'a>(
    conn: &Connection,
    pinyin: &'a str,
    match_as_same_as_input: bool,
) -> Result<Option<(&'a str, Suggest)>, Box<dyn Error>> {
    for i in 1..=pinyin.len() {
        let sub_pinyin = &pinyin[0..=pinyin.len() - i];

        let suggests = query_suggests(
            conn,
            &format!( "SELECT pinyin, hanzi, priority FROM dict WHERE pinyin = '{}' ORDER BY priority DESC limit 1", sub_pinyin)
        )?;

        for suggest in suggests {
            return Ok(Some((sub_pinyin, suggest)));
        }
    }

    if !match_as_same_as_input {
        for i in 1..=pinyin.len() {
            let sub_pinyin = &pinyin[0..=pinyin.len() - i];
            let suggests = query_suggests(
                conn,
                &format!(
                    "SELECT pinyin, hanzi, priority FROM dict WHERE pinyin BETWEEN '{}' AND '{}{{' ORDER BY priority DESC limit 1",
                    sub_pinyin, sub_pinyin
                    )
                )?;

            for suggest in suggests {
                return Ok(Some((sub_pinyin, suggest)));
            }
        }
    }

    Ok(None)
}

#[cfg(test)]
pub mod test_sqlite {
    use rusqlite::Connection;

    use super::query_start_match_dict;

    #[test]
    fn test_query_dict() {
        let conn = Connection::open("../dict-builder/dicts/dict.db3").expect("Open Connection");
        if let Ok(suggests) = query_start_match_dict(&conn, "ni", 10) {
            assert!(suggests.len() > 0);
        }
    }
}
