/**
  * Try to get the content of a result, returning a new error if there was one.
  * If an error occurs, it is returned as result of the calling function.
  *
  * Params:
  *   $res: Result<T,E>
  *   $err: String
  *
  * Returns: T or Err(String)
  */
#[macro_export]
macro_rules! try_result {
    ($res:expr, $err:expr) => {
        match $res {
            Ok(val) => val,
            Err(orig) => {
                error!("{} ({})", $err, orig.to_string());
                return Err(String::from($err));
            }
        }
    }
}

#[macro_export]
macro_rules! try_result_debug {
    ($res:expr, $err:expr) => {
        match $res {
            Ok(val) => val,
            Err(orig) => {
                error!("{} ({:?})", $err, orig);
                return Err(String::from($err));
            }
        }
    }
}

/**
  * Try to get the content of an option, returning an error if there is no content
  * If an error occurs, it is returned as result of the calling function.
  *
  * Params:
  *   $opt: Option<T>
  *   $err: String
  *
  * Returns: T or Err(String)
  */
#[macro_export]
macro_rules! try_option {
    ($opt:expr, $err:expr) => {
        match $opt {
            Some(val) => val,
            None => {
                error!("{}", $err);
                return Err(String::from($err));
            }
        }
    }
}

/**
  * Create a result of a boolean value. If the bool is true, return the Ok value,
  * if it is false return an error created from the error message.
  *
  * Params:
  *   $cond: bool
  *   $ok: T
  *   $err: String or similar
  *
  * Returns Result<T,String>
  */
#[macro_export]
macro_rules! bool_result {
    ($cond:expr, $ok:expr, $err:expr) => {
        if $cond {
            Ok($ok)
        } else {
            error!("{}", $err);
            return Err(String::from($err));
        }
    }
}

/**
  * Rewrap a result object, injecting a custom error message and returning the new Result
  *
  * Params:
  *   $res: Result<T,E>
  *   $err: String
  *
  * Returns: Result<T,String>
  */
#[macro_export]
macro_rules! change_error {
    ($res:expr, $err:expr) => {
        match $res {
            Ok(val) => Ok(val),
            Err(_) => {
                error!("{}", $err);
                return Err($err.to_string());
            }
        }
    }
}

#[macro_export]
macro_rules! dry_run {
    ($output:expr) => {
        println!("DRY-RUN: {}", $output);
    }
}

#[macro_export]
macro_rules! log_error {
    ($result:expr) => {
        if let Err(err) = $result {
            error!("{}", err);
        }
    }
}
