use std::error::Error;

use dict_builder::{
    dict::format_dict,
    sqlite::{batch_insert_records, create_dict_index, create_dict_table},
};
use rusqlite::Connection;

fn main() -> Result<(), Box<dyn Error>> {
    let tables = ["dict"];
    let dict_paths = [
        ("./dicts/8105.dict.yaml", tables[0]),
        ("./dicts/base.dict.yaml", tables[0]),
        ("./dicts/ext.dict.yaml", tables[0]),
        ("./dicts/others.dict.yaml", tables[0]),
        ("./dicts/sogou.dict.yaml", tables[0]),
        ("./dicts/tencent.dict.yaml", tables[0]),
    ];

    println!("Resolve dict list");

    let dicts = dict_paths.map(|(dict, table)| {
        (
            table,
            format_dict(dict).unwrap_or_else(|err| {
                println!("Resolve dict [{}] error: {}", dict, err);
                vec![]
            }),
        )
    });

    println!("Open database connection");

    // open databases connection
    let conn = Connection::open("./dicts/dict.db3")?;

    println!("Create dict table");

    // create dict table
    create_dict_table(&conn, &tables)?;

    println!("Batch insert records");

    // batch insert records
    batch_insert_records(&conn, &dicts)?;

    println!("Create dict index");

    // create dict index
    create_dict_index(&conn, &tables)?;

    println!("Done");

    Ok(())
}
