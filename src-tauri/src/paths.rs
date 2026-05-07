use std::{env, path::PathBuf};

pub const APP_IDENTIFIER: &str = "com.local.study-guardian";
pub const NATIVE_HOST_NAME: &str = "com.local.study_guardian";

pub fn fallback_config_dir() -> PathBuf {
    if let Some(appdata) = env::var_os("APPDATA") {
        PathBuf::from(appdata).join(APP_IDENTIFIER)
    } else {
        env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(APP_IDENTIFIER)
    }
}

pub fn native_url_path_from_config_dir(config_dir: PathBuf) -> PathBuf {
    config_dir.join("native-url.json")
}
