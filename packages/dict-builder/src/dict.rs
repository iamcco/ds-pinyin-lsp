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

pub fn format_dict(dict_path: &str) -> Result<Vec<(String, String, u32)>, Box<dyn error::Error>> {
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

            // split by whitespace
            let seps = line.split_whitespace().into_iter().collect::<Vec<&str>>();

            // invalid line
            if seps.len() < 2 {
                return (String::new(), String::new(), 0);
            }

            // hanzi at rist column
            let hanzi = seps[0];

            // invalid hanzi
            if hanzi.eq("") {
                return (String::new(), String::new(), 0);
            }

            // the pinyin of hanzi
            // use dict pinyin first
            let pinyin = if seps.len() > 2 {
                seps[1..=seps.len() - 2].join("")
            } else {
                get_pinyin_from_hanzi(hanzi)
            };

            // invalid pinyin
            if pinyin.eq("") {
                return (String::new(), String::new(), 0);
            }

            // priority at last column and maybe missing
            let priority = seps[seps.len() - 1].parse::<u32>().unwrap_or(1);

            (pinyin, hanzi.to_string(), priority)
        })
        // filter out empty line
        .filter(|line| !line.0.is_empty())
        .collect();

    Ok(res)
}
