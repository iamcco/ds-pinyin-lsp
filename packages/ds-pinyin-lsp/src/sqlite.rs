use std::error::Error;

use rusqlite::Connection;

use crate::types::Suggest;

/// query suggest
fn query_suggests(conn: &Connection, query: &str) -> Result<Vec<Suggest>, Box<dyn Error>> {
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

/// query in words table
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

    query_suggests(conn, &stmt)
}

/// query in dict table
pub fn query_dict(conn: &Connection, pinyin: &str) -> Result<Vec<Suggest>, Box<dyn Error>> {
    query_suggests(
        conn,
        &format!(
            "SELECT pinyin, hanzi, priority FROM dict WHERE pinyin BETWEEN '{}' AND '{}{{' ORDER BY priority DESC limit 50",
            pinyin, pinyin
        )
     )
}

/// query the longest match of the pinyin
pub fn query_the_longest_match<'a>(
    conn: &Connection,
    pinyin: &'a str,
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

    for i in 1..=pinyin.len() {
        let sub_pinyin = &pinyin[0..=pinyin.len() - i];
        let suggests = query_suggests(
            conn,
            &format!("SELECT pinyin, hanzi, priority FROM words WHERE pinyin = '{}' ORDER BY priority DESC limit 1", sub_pinyin)
        )?;

        for suggest in suggests {
            return Ok(Some((sub_pinyin, suggest)));
        }
    }

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

    Ok(None)
}

#[cfg(test)]
pub mod test_sqlite {
    use rusqlite::Connection;

    use super::query_words;

    #[test]
    fn test_query_words() {
        let conn = Connection::open("../dict-builder/dicts/dict.db3").expect("Open Connection");
        if let Ok(suggests) = query_words(&conn, "ni", true) {
            assert!(suggests.len() > 0);
        }
    }
}
