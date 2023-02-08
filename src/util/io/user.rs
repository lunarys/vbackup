use std::io;
use crate::try_result;

pub fn ask_user(question: &str, default: Option<&str>) -> Result<Option<String>, String> {
    let default_hint = if let Some(default) = default {
        format!("[{}]", default)
    } else {
        String::new()
    };

    println!("{} {}", question, default_hint);

    let mut input = String::new();
    let result = io::stdin().read_line(&mut input);

    try_result!(result, "Could not read user input");

    Ok(if input.len() == 0 {
        default.map(String::from)
    } else {
        Some(input)
    })
}

pub fn ask_user_boolean(question: &str, default: bool) -> Result<bool, String> {
    let result = ask_user(format!("{} {}", question, if default {"(Y/n)"} else {"(y/N)"}).as_str(), None)?;

    Ok(result.map_or(default, |input| {
        if input.to_lowercase() == "y" {
            true
        } else {
            false
        }
    }))
}