use std::io;
use std::str::FromStr;
use crate::try_result;

fn get_user_input_line() -> Result<String,String> {
    let mut input = String::new();
    let result = io::stdin().read_line(&mut input);

    // input is confirmed with enter and that is appended to the input string... remove it
    let len_without_newline = input.trim_end().len();
    input.truncate(len_without_newline);

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

pub fn ask_user_option_list_index<T>(
    string_before_list: Option<&str>,
    string_after_list: Option<&str>,
    input_list: &Vec<T>,
    list_mapper: &dyn Fn(&T) -> &str,
    default_index: usize
) -> Result<usize, String> {

    if let Some(print) = string_before_list {
        println!("{}", print);
    }

    input_list.iter().enumerate().for_each(|(index,entry)| {
        println!("[{}] {}", index, apply_mapper(list_mapper, entry));
    });

    let user_input_result = if let Some(print) = string_after_list {
        ask_user_default(print, default_index.to_string().as_str())
    } else {
        ask_user_default("Please select an option from the list above.", default_index.to_string().as_str())
    };

    let user_input = try_result!(user_input_result, "Could not get user input");

    return if let Ok(index) = usize::from_str(user_input.as_str()) {
        Ok(index)
    } else {
        let err = "Could not parse user input, expected a number";
        error!("{}", err);
        Err(String::from(err))
    };
}

pub fn ask_user_option_list<'a,T>(
    string_before_list: Option<&str>,
    string_after_list: Option<&str>,
    input_list: &'a Vec<T>,
    list_mapper: &'a dyn Fn(&T) -> &str,
    default_index: usize
) -> Result<&'a T, String> {
    let index = ask_user_option_list_index(
        string_before_list,
        string_after_list,
        input_list,
        list_mapper,
        default_index
    )?;

    return input_list
        .get(index)
        .ok_or(String::from("Selected index is out of bounds"));
}

fn apply_mapper<'a,T>(fun: &'a dyn Fn(&T) -> &str, input: &'a T) -> &'a str {
    fun(input)
}

pub fn ask_user_abort(text: Option<&str>) -> Result<(),String> {
    let confirm = ask_user_boolean(text.unwrap_or("Continue?"), true)?;
    return if !confirm {
        Err(String::from("Aborted by user"))
    } else {
        Ok(())
    }
}