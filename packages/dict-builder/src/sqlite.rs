use std::error::Error;

use rusqlite::Connection;

pub fn create_dict_table(conn: &Connection, tables: &[&str]) -> Result<(), Box<dyn Error>> {
    for table in tables {
        conn.execute(
            &format!(
                "CREATE TABLE {} (
                    id INTEGER PRIMARY KEY,
                    pinyin TEXT NOT NULL,
                    hanzi TEXT NOT NULL,
                    priority INTEGER
                )",
                table
            ),
            (),
        )?;
    }

    Ok(())
}

pub fn create_dict_index(conn: &Connection, tables: &[&str]) -> Result<(), Box<dyn Error>> {
    for table in tables {
        conn.execute(
            &format!(
                "CREATE INDEX {}_index ON {}(pinyin, priority)",
                table, table
            ),
            (),
        )?;
    }

    Ok(())
}

pub fn batch_insert_records(
    conn: &Connection,
    dicts: &[(&str, Vec<(String, String, u32)>)],
) -> Result<(), Box<dyn Error>> {
    // begin transaction
    conn.execute("BEGIN TRANSACTION", ())?;

    // insert records
    for (table, dict) in dicts {
        for (pinyin, hanzi, priority) in dict {
            if let Err(err) = conn.execute(
                &format!(
                    "INSERT INTO {} (pinyin, hanzi, priority) VALUES (?1, ?2, ?3)",
                    table
                ),
                (pinyin, hanzi, priority),
            ) {
                println!(
                    "Insert record [{}, {}, {}] for {} error: {:?}",
                    pinyin, hanzi, priority, table, err
                );
            }
        }
    }

    // commit
    conn.execute("COMMIT", ())?;

    Ok(())
}
