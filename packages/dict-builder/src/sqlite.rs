use std::error::Error;

use rusqlite::Connection;

pub fn create_dict_table(conn: &Connection) -> Result<(), Box<dyn Error>> {
    conn.execute(
        "CREATE TABLE dict (
            id INTEGER PRIMARY KEY,
            pinyin TEXT NOT NULL,
            hanzi TEXT NOT NULL,
            priority INTEGER
        )",
        (),
    )?;

    Ok(())
}

pub fn create_dict_index(conn: &Connection) -> Result<(), Box<dyn Error>> {
    conn.execute(
        "CREATE INDEX dict_index ON dict (
            pinyin
        )",
        (),
    )?;

    Ok(())
}

pub fn batch_insert_records(
    conn: &Connection,
    dicts: &[Vec<(String, String, u32)>],
) -> Result<(), Box<dyn Error>> {
    // begin transaction
    conn.execute("BEGIN TRANSACTION", ())?;

    // insert records
    for dict in dicts {
        for (pinyin, hanzi, priority) in dict {
            if let Err(err) = conn.execute(
                "INSERT INTO dict (pinyin, hanzi, priority) VALUES (?1, ?2, ?3)",
                (pinyin, hanzi, priority),
            ) {
                println!(
                    "Insert record [{}, {}, {}] error: {:?}",
                    pinyin, hanzi, priority, err
                );
            }
        }
    }

    // commit
    conn.execute("COMMIT", ())?;

    Ok(())
}
