// Macro for unwrapping a result or returning a custom error
#[macro_export]
macro_rules! try_else {
    ($res:expr, $err:expr) => {
        match $res {
            Ok(val) => val,
            Err(_) => return Err($err.to_string())
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