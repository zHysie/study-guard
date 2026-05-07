use std::{fs, io, path::PathBuf, process::Command};

const EXTENSION_ID: &str = "ehmcjemdmobgpaljappopgnmmlkjjcgo";
const HOST_NAME: &str = "com.local.study_guardian";

pub fn ensure_registered(config_dir: &std::path::Path) {
    if is_registered() {
        return;
    }

    let manifest_path = config_dir.join("native-messaging-host.json");
    if let Err(err) = write_manifest(&manifest_path) {
        eprintln!("failed to write native host manifest: {err}");
        return;
    }

    register_for("Chrome", &manifest_path);
    register_for("Edge", &manifest_path);
}

fn is_registered() -> bool {
    let output = Command::new("reg.exe")
        .args([
            "query",
            r"HKCU\Software\Google\Chrome\NativeMessagingHosts\com.local.study_guardian",
        ])
        .output();
    matches!(output, Ok(o) if o.status.success())
}

fn write_manifest(path: &std::path::Path) -> io::Result<()> {
    let exe = std::env::current_exe()?;
    let exe_path = exe.to_string_lossy().replace('\\', "\\\\");

    let manifest = format!(
        r#"{{
  "name": "{host}",
  "description": "Study Guardian Native Messaging Host",
  "path": "{exe}",
  "type": "stdio",
  "allowed_origins": [
    "chrome-extension://{id}/"
  ]
}}"#,
        host = HOST_NAME,
        exe = exe_path,
        id = EXTENSION_ID,
    );

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, manifest)
}

fn register_for(browser: &str, manifest_path: &PathBuf) {
    let reg_key = format!(
        r"HKCU\Software\{}\NativeMessagingHosts\{}",
        if browser == "Chrome" {
            r"Google\Chrome"
        } else {
            r"Microsoft\Edge"
        },
        HOST_NAME
    );

    let _ = Command::new("reg.exe")
        .args([
            "add",
            &reg_key,
            "/ve",
            "/d",
            &manifest_path.to_string_lossy(),
            "/f",
        ])
        .output();
}
