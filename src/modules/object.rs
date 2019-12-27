pub struct Paths {
    pub save_path: String,
    pub timeframes_file: String,
    pub tmp_dir: String,
    pub auth_data_file: String,
    pub module_data_dir: String
}

impl Paths {
    pub fn copy(&self) -> Self {
        return Paths {
            save_path: String::from(&self.save_path),
            timeframes_file: String::from(&self.timeframes_file),
            tmp_dir: String::from(&self.tmp_dir),
            auth_data_file: String::from(&self.auth_data_file),
            module_data_dir: String::from(&self.module_data_dir)
        }
    }
}