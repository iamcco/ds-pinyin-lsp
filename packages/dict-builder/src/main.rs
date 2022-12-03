use std::error::Error;

use dict_builder::{
    dict::get_format_dict,
    sqlite::{batch_insert_records, create_dict_index, create_dict_table},
};
use rusqlite::Connection;

fn main() -> Result<(), Box<dyn Error>> {
    let dict_paths = [
        "./dicts/8105.dict.yaml",
        "./dicts/base.dict.yaml",
        "./dicts/ext.dict.yaml",
        "./dicts/others.dict.yaml",
        "./dicts/sogou.dict.yaml",
        "./dicts/tencent.dict.yaml",
    ];

    println!("Resolve dict list");

    let dicts = dict_paths.map(|dict| {
        get_format_dict(dict).unwrap_or_else(|err| {
            println!("Resolve dict [{}] error: {}", dict, err);
            vec![]
        })
    });

    println!("Open database connection");

    // open databases connection
    let conn = Connection::open("./dicts/dict.db3")?;

    println!("Create dict table");

    // create dict table
    create_dict_table(&conn)?;

    println!("Batch insert records");

    // batch insert records
    batch_insert_records(&conn, &dicts)?;

    println!("Create dict index");

    // create dict index
    create_dict_index(&conn)?;

    println!("Done");

    Ok(())
}
