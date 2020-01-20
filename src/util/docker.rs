use crate::modules::object::Paths;
use crate::util::command::CommandWrapper;

pub fn image_exists(image_name: &str) -> Result<bool,String> {
    let mut cmd = CommandWrapper::new("sh");
    cmd.arg_str("-c").arg_string(format!("docker images -q {} 2> /dev/null", image_name));
    let output = cmd.run_get_output()?;
    return Ok(!output.is_empty());
}

pub fn build_image(paths: &Paths, image_file: &str, image_name: &str) -> Result<(),String> {
    let mut cmd = CommandWrapper::new("docker");

    // Build image
    cmd.arg_str("build")
        // Set tag
        .arg_str("-t")
        .arg_string(String::from(image_name))
        // Set dockerfile
        .arg_str("-f")
        .arg_string(format!("{}/{}", &paths.docker_images, image_file))
        // Set context for build (just tmp dir for now as none is really required...)
        .arg_string(paths.tmp_dir.clone());

    let status = cmd.run_get_status()?;
    if status.success() {
        return Ok(());
    } else {
        return Err(format!("Failed building the docker image (Code: {})", status.to_string()));
    }
}

pub fn build_image_if_missing(paths: &Paths, image_file: &str, image_name: &str) -> Result<bool,String> {
    if image_exists(image_name)? {
        return Ok(false);
    } else {
        return build_image(paths, image_file, image_name).map(|_| true);
    }
}