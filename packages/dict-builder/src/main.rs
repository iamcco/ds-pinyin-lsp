use std::error::Error;

use dict_builder::{
    dict::{format_dict, format_other_dict},
    sqlite::{batch_insert_records, create_dict_index, create_dict_table},
};
use rusqlite::Connection;

enum DictTypes {
    CN,
    CC,
}

fn main() -> Result<(), Box<dyn Error>> {
    let tables = ["dict"];
    let dict_paths = [
        ("./dicts/8105.dict.yaml", tables[0], DictTypes::CN),
        ("./dicts/base.dict.yaml", tables[0], DictTypes::CN),
        ("./dicts/ext.dict.yaml", tables[0], DictTypes::CN),
        ("./dicts/others.dict.yaml", tables[0], DictTypes::CN),
        ("./dicts/sogou.dict.yaml", tables[0], DictTypes::CN),
        ("./dicts/tencent.dict.yaml", tables[0], DictTypes::CN),
        ("./dicts/others.txt", tables[0], DictTypes::CC),
        ("./dicts/emoji.txt", tables[0], DictTypes::CC),
    ];

    println!("Resolve dict list");

    let dicts = dict_paths.map(|(dict, table, dict_type)| {
        (
            table,
            match dict_type {
                DictTypes::CN => format_dict(dict),
                DictTypes::CC => format_other_dict(dict),
            }
            .unwrap_or_else(|err| {
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
