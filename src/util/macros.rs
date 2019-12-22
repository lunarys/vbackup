// Macro for unwrapping a result or returning a custom error
#[macro_export]
macro_rules! try_result {
    ($res:expr, $err:expr) => {
        match $res {
            Ok(val) => val,
            Err(orig) => {
                error!("{} ({})", $err, orig.to_string());
                return Err($err.to_string());
            }
        }
    }
}

#[macro_export]
macro_rules! try_option {
    ($res:expr, $err:expr) => {
        match $res {
            Some(val) => val,
            None => {
                error!("{}", $err);
                return Err($err.to_string());
            }
        }
    }
}

#[macro_export]
macro_rules! rewrap {
    ($res:expr, $err:expr) => {
        match $res {
            Ok(val) => return Ok(val),
            Err(_) => return Err($err.to_string())
        }
    }
}