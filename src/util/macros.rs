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
                return Err($err.to_string());
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
                return Err($err.to_string());
            }
        }
    }
}

/**
  * Rewrap a result object, injecting a custom error message and retuning the new Result immediately
  *
  * Params:
  *   $res: Result<T,E>
  *   $err: String
  *
  * Returns: Result<T,String>
  */
#[macro_export]
macro_rules! rewrap {
    ($res:expr, $err:expr) => {
        match $res {
            Ok(val) => return Ok(val),
            Err(_) => return Err($err.to_string())
        }
    }
}

/**
  * Resolve the authentication configuration,
  * which is either a reference to the shared authentication store,
  * or directly provided.
  * If an error occurs, it is returned as result of the calling function.
  *
  * Params:
  *   $reference : &Option<String> -> If is some, use the reference to find the shared authentication
  *   $obj: &Option<Value> -> If the reference is none, use this authentication object
  *   $paths: &Paths -> Paths object to resolve the shared authentication data store
  */
#[macro_export]
macro_rules! auth_resolve {
    ($reference:expr, $obj:expr, $paths:expr) => {
        match auth_data::load_if_reference($reference, $paths) {
            Ok(Some(value)) => {
                try_result!(serde_json::from_value(value), "Failed parsing shared authentication config")
            },
            Ok(None) => {
                let value = try_option!($obj.clone(), "Expected provided authentication, got none");
                try_result!(serde_json::from_value(value), "Failed parsing provided authentication")
            },
            Err(err) => return Err(err)
        };
    }
}

/**
  * Resolve the configuration by parsing the json object or returning an error if not possible,
  * the result being returned as a deserialized struct
  * If an error occurs, it is returned as result of the calling function.
  *
  * Params:
  *   $obj: &Value -> The already parsed json value
  */
#[macro_export]
macro_rules! conf_resolve {
    ($obj:expr) => {
        try_result!(serde_json::from_value($obj.clone()), "Could not parse configuration");
    }
}