use crate::config::AppConfig;
use std::{
    collections::HashSet,
    fs, io,
    path::{Path, PathBuf},
    process::Command,
};

const HOST_NAME: &str = "com.local.study_guardian";
const EXTENSION_NAME: &str = "学习守门员";
const NATIVE_HOST_EXE: &str = "study_guardian_native_host.exe";

pub fn ensure_registered(config_dir: &Path, config: &AppConfig) {
    let ids = if config.extension_id.is_empty() {
        find_extension_ids()
    } else {
        extension_infos_for_manual_id(&config.extension_id)
    };
    if ids.is_empty() {
        // Extension not yet installed in any browser; skip registration.
        // Restart the app after loading the extension to trigger registration.
        return;
    }

    let manifest_path = config_dir.join("native-messaging-host.json");
    let host_exe = match native_host_exe_path() {
        Ok(path) => path,
        Err(err) => {
            eprintln!("failed to resolve native host executable: {err}");
            return;
        }
    };

    if let Err(err) = write_manifest(&manifest_path, &host_exe, &ids) {
        eprintln!("failed to write native host manifest: {err}");
        return;
    }

    let mut registered = HashSet::new();
    for browser in ids.iter().map(|info| info.browser.as_str()) {
        if registered.insert(browser) {
            register_for(browser, &manifest_path);
        }
    }
}

#[derive(Debug)]
struct ExtensionInfo {
    browser: String,
    id: String,
}

fn find_extension_ids() -> Vec<ExtensionInfo> {
    let localapp = std::env::var_os("LOCALAPPDATA");
    let Some(localapp) = localapp else {
        return Vec::new();
    };
    find_extension_ids_in(&PathBuf::from(localapp))
}

fn extension_infos_for_manual_id(id: &str) -> Vec<ExtensionInfo> {
    ["Chrome", "Edge"]
        .into_iter()
        .map(|browser| ExtensionInfo {
            browser: browser.to_string(),
            id: id.to_string(),
        })
        .collect()
}

fn find_extension_ids_in(localapp: &Path) -> Vec<ExtensionInfo> {
    let mut results = Vec::new();
    let mut seen = HashSet::new();

    for (browser_key, prefs_dir) in [
        ("Chrome", "Google\\Chrome\\User Data"),
        ("Edge", "Microsoft\\Edge\\User Data"),
    ] {
        let user_data_dir = localapp.join(prefs_dir);
        let Ok(entries) = fs::read_dir(user_data_dir) else {
            continue;
        };

        for entry in entries.flatten() {
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if !file_type.is_dir() {
                continue;
            }

            let prefs_file = entry.path().join("Preferences");
            let Ok(content) = fs::read_to_string(&prefs_file) else {
                continue;
            };
            let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) else {
                continue;
            };

            let Some(settings) = json.get("extensions").and_then(|e| e.get("settings")) else {
                continue;
            };

            let Some(settings) = settings.as_object() else {
                continue;
            };

            for (ext_id, ext_data) in settings {
                if let Some(name) = ext_data
                    .get("manifest")
                    .and_then(|m| m.get("name"))
                    .and_then(|n| n.as_str())
                {
                    if name.contains(EXTENSION_NAME)
                        && seen.insert((browser_key.to_string(), ext_id.clone()))
                    {
                        results.push(ExtensionInfo {
                            browser: browser_key.to_string(),
                            id: ext_id.clone(),
                        });
                    }
                }
            }
        }
    }

    results
}

fn native_host_exe_path() -> io::Result<PathBuf> {
    Ok(native_host_exe_path_for_current_exe(
        &std::env::current_exe()?,
    ))
}

fn native_host_exe_path_for_current_exe(current_exe: &Path) -> PathBuf {
    if current_exe
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.eq_ignore_ascii_case(NATIVE_HOST_EXE))
    {
        return current_exe.to_path_buf();
    }

    let sidecar = current_exe.with_file_name(NATIVE_HOST_EXE);
    if sidecar.exists() {
        sidecar
    } else {
        current_exe.to_path_buf()
    }
}

fn write_manifest(path: &Path, exe: &Path, ids: &[ExtensionInfo]) -> io::Result<()> {
    let manifest = manifest_contents(exe, ids);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, manifest)
}

fn manifest_contents(exe: &Path, ids: &[ExtensionInfo]) -> String {
    let exe_path = exe.to_string_lossy().replace('\\', "\\\\");
    let origins: Vec<String> = ids
        .iter()
        .map(|info| format!("    \"chrome-extension://{}/\"", info.id))
        .collect();
    let origins_json = origins.join(",\n");

    format!(
        r#"{{
  "name": "{host}",
  "description": "Study Guardian Native Messaging Host",
  "path": "{exe}",
  "type": "stdio",
  "allowed_origins": [
{origins}
  ]
}}"#,
        host = HOST_NAME,
        exe = exe_path,
        origins = origins_json,
    )
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_dir(name: &str) -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock before epoch")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "study_guardian_native_host_{name}_{}_{}",
            std::process::id(),
            suffix
        ))
    }

    fn write_preferences(path: &std::path::Path, extension_id: &str, extension_name: &str) {
        fs::create_dir_all(path.parent().expect("preferences parent")).expect("create profile dir");
        let content = format!(
            r#"{{
  "extensions": {{
    "settings": {{
      "{extension_id}": {{
        "manifest": {{
          "name": "{extension_name}"
        }}
      }}
    }}
  }}
}}"#
        );
        fs::write(path, content).expect("write preferences");
    }

    #[test]
    fn scans_all_browser_profiles_for_extension_ids() {
        let localapp = unique_temp_dir("profiles");
        let chrome_prefs = localapp
            .join("Google")
            .join("Chrome")
            .join("User Data")
            .join("Profile 1")
            .join("Preferences");
        let edge_prefs = localapp
            .join("Microsoft")
            .join("Edge")
            .join("User Data")
            .join("Default")
            .join("Preferences");

        write_preferences(
            &chrome_prefs,
            "chrome_extension_id",
            "学习守门员 URL Provider",
        );
        write_preferences(&edge_prefs, "edge_extension_id", "学习守门员 URL Provider");

        let ids = find_extension_ids_in(&localapp);

        assert!(ids
            .iter()
            .any(|info| info.browser == "Chrome" && info.id == "chrome_extension_id"));
        assert!(ids
            .iter()
            .any(|info| info.browser == "Edge" && info.id == "edge_extension_id"));

        let _ = fs::remove_dir_all(localapp);
    }

    #[test]
    fn manifest_uses_native_host_exe_when_sidecar_exists() {
        let dir = unique_temp_dir("exe");
        fs::create_dir_all(&dir).expect("create exe dir");
        let main_exe = dir.join("study_guardian.exe");
        let host_exe = dir.join("study_guardian_native_host.exe");
        fs::write(&host_exe, []).expect("write host exe");

        let selected = native_host_exe_path_for_current_exe(&main_exe);

        assert_eq!(selected, host_exe);

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn manifest_includes_scanned_extension_origins() {
        let ids = vec![
            ExtensionInfo {
                browser: "Chrome".into(),
                id: "chrome_extension_id".into(),
            },
            ExtensionInfo {
                browser: "Edge".into(),
                id: "edge_extension_id".into(),
            },
        ];

        let manifest = manifest_contents(std::path::Path::new(r"C:\Apps\study_guardian.exe"), &ids);

        assert!(manifest.contains(r#""chrome-extension://chrome_extension_id/""#));
        assert!(manifest.contains(r#""chrome-extension://edge_extension_id/""#));
        assert!(manifest.contains(r#""path": "C:\\Apps\\study_guardian.exe""#));
    }

    #[test]
    fn manual_extension_id_registers_both_browsers_without_browser_scan() {
        let ids = extension_infos_for_manual_id("niilfgnhlfenpeglelbdjmkbacnloglj");

        assert!(ids
            .iter()
            .any(|info| info.browser == "Chrome" && info.id == "niilfgnhlfenpeglelbdjmkbacnloglj"));
        assert!(ids
            .iter()
            .any(|info| info.browser == "Edge" && info.id == "niilfgnhlfenpeglelbdjmkbacnloglj"));
    }
}
