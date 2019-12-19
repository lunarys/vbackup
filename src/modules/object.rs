use serde_json::Value;

pub struct Arguments {
    name: String,
    config: Value,
    timeframes: Value,
    paths: Paths,
    dry_run: bool,
    no_docker: bool
}

pub struct Paths {
    save_path: String,
    timeframes_file: String,
    tmp_dir: String,
    auth_data_file: String,
    module_data_dir: String
}