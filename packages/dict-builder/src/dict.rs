use std::{error, fs::read_to_string};

use pinyin::ToPinyin;

pub fn get_pinyin_from_hanzi(hanzi: &str) -> String {
    hanzi
        .to_pinyin()
        .map(|py| {
            if let Some(py) = py {
                return py.plain();
            }
            ""
        })
        .collect::<Vec<&str>>()
        .join("")
}

pub fn get_format_dict(
    dict_path: &str,
) -> Result<Vec<(String, String, u32)>, Box<dyn error::Error>> {
    let mut is_valid_line = false;

    let res = read_to_string(dict_path)?
        .lines()
        .map(|line| {
            // dict meta data end flag
            if line.eq("...") {
                is_valid_line = true;
                return (String::new(), String::new(), 0);
            }

            // ignore meta data line
            // ignore empty line
            // ignore comment line
            if !is_valid_line || line.is_empty() || line.starts_with("#") {
                return (String::new(), String::new(), 0);
            }

            let mut sep = line.split_whitespace();
            // hanzi at rist column
            let hanzi = sep.next().unwrap_or("");
            let pinyin = get_pinyin_from_hanzi(hanzi);
            // priority at last column and maybe missing
            let priority = sep.last().unwrap_or("").parse::<u32>().unwrap_or(1);

            (pinyin, hanzi.to_string(), priority)
        })
        // filter out empty line
        .filter(|line| !line.0.is_empty())
        .collect();

    Ok(res)
}
