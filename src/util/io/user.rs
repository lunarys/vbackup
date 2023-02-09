use std::io;

fn get_user_input_line() -> Result<String,String> {
    let mut input = String::new();
    let result = io::stdin().read_line(&mut input);

    return if let Err(err) = result {
        Err(format!("Could not read user input: {}", err))
    } else {
        Ok(input)
    };
}

pub fn ask_user(question: &str) -> Result<Option<String>, String> {
    println!("{}", question);

    let input = get_user_input_line()?;

    Ok(if input.len() == 0 {
        None
    } else {
        Some(input)
    })
}

pub fn ask_user_default(question: &str, default: &str) -> Result<String, String> {
    println!("{} [{}]", question, default);

    let input = get_user_input_line()?;

    Ok(if input.len() == 0 {
        String::from(default)
    } else {
        input
    })
}

pub fn ask_user_boolean(question: &str, default: bool) -> Result<bool, String> {
    let result = ask_user(format!("{} {}", question, if default {"(Y/n)"} else {"(y/N)"}).as_str())?;

    Ok(result.map_or(default, |input| {
        if input.to_lowercase() == "y" {
            true
        } else {
            false
        }
    }))
}