use serde::{Deserialize, Serialize};
use std::{fs, io, path::Path};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    #[serde(default)]
    pub extension_id: String,
    pub video_whitelist: Vec<String>,
    pub up_whitelist: Vec<String>,
    pub domain_blacklist: Vec<String>,
    pub idle_minutes: u32,
    pub overlay_distracting_minutes: u32,
    pub banner_delay_seconds: u32,
    pub check_interval_seconds: u32,
    pub banner_text: String,
    pub overlay_text: String,
    pub overlay_image_path: String,
    pub overlay_sound_enabled: bool,
    pub overlay_sound_path: String,
    pub overlay_voice_text: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            extension_id: String::new(),
            video_whitelist: Vec::new(),
            up_whitelist: Vec::new(),
            domain_blacklist: vec!["xiaohongshu.com".into(), "douyin.com".into()],
            idle_minutes: 1,
            overlay_distracting_minutes: 5,
            banner_delay_seconds: 5,
            check_interval_seconds: 2,
            banner_text: "快去学习".into(),
            overlay_text: "别刷了，回到教程".into(),
            overlay_image_path: String::new(),
            overlay_sound_enabled: true,
            overlay_sound_path: String::new(),
            overlay_voice_text: "快点学习！".into(),
        }
    }
}

impl AppConfig {
    pub fn sanitized(mut self) -> Self {
        self.video_whitelist = sanitize_list(self.video_whitelist);
        self.up_whitelist = sanitize_list(self.up_whitelist);
        self.domain_blacklist = sanitize_list(self.domain_blacklist)
            .into_iter()
            .map(|domain| domain.trim_start_matches('.').to_ascii_lowercase())
            .collect();
        self.extension_id = sanitize_extension_id(&self.extension_id);

        if !self.domain_blacklist.iter().any(|d| d == "xiaohongshu.com") {
            self.domain_blacklist.push("xiaohongshu.com".into());
        }
        if !self.domain_blacklist.iter().any(|d| d == "douyin.com") {
            self.domain_blacklist.push("douyin.com".into());
        }

        self.idle_minutes = clamp_minutes(self.idle_minutes);
        self.overlay_distracting_minutes = clamp_minutes(self.overlay_distracting_minutes);
        self.banner_delay_seconds = self.banner_delay_seconds.clamp(1, 600);
        self.check_interval_seconds = 2;
        self
    }
}

pub fn load_or_default(path: &Path) -> io::Result<AppConfig> {
    if !path.exists() {
        let config = AppConfig::default();
        save_to_path(path, &config)?;
        return Ok(config);
    }

    let content = fs::read_to_string(path)?;
    match serde_json::from_str::<AppConfig>(&content) {
        Ok(config) => Ok(config.sanitized()),
        Err(_) => save_to_path(path, &AppConfig::default()),
    }
}

pub fn save_to_path(path: &Path, config: &AppConfig) -> io::Result<AppConfig> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let sanitized = config.clone().sanitized();
    let content = serde_json::to_string_pretty(&sanitized)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
    fs::write(path, content)?;
    Ok(sanitized)
}

fn sanitize_list(values: Vec<String>) -> Vec<String> {
    values
        .into_iter()
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
        .collect()
}

fn sanitize_extension_id(value: &str) -> String {
    let mut id = value.trim().to_ascii_lowercase();
    if let Some(rest) = id.strip_prefix("chrome-extension://") {
        id = rest.trim_end_matches('/').to_string();
    }

    if id.len() == 32 && id.chars().all(|ch| ('a'..='p').contains(&ch)) {
        id
    } else {
        String::new()
    }
}

fn clamp_minutes(value: u32) -> u32 {
    value.clamp(1, 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_blacklist_contains_required_domains() {
        let config = AppConfig::default();

        assert!(config.domain_blacklist.contains(&"xiaohongshu.com".into()));
        assert!(config.domain_blacklist.contains(&"douyin.com".into()));
    }

    #[test]
    fn minute_fields_are_clamped_to_one_to_sixty_and_interval_is_fixed() {
        let config = AppConfig {
            idle_minutes: 0,
            overlay_distracting_minutes: 99,
            banner_delay_seconds: 0,
            check_interval_seconds: 30,
            ..AppConfig::default()
        }
        .sanitized();

        assert_eq!(config.idle_minutes, 1);
        assert_eq!(config.overlay_distracting_minutes, 60);
        assert_eq!(config.banner_delay_seconds, 1);
        assert_eq!(config.check_interval_seconds, 2);
    }

    #[test]
    fn extension_id_is_trimmed_and_normalized() {
        let config = AppConfig {
            extension_id: " chrome-extension://NIILFGNHLFENPEGLELBDJMKBACNLOGLJ/ ".into(),
            ..AppConfig::default()
        }
        .sanitized();

        assert_eq!(config.extension_id, "niilfgnhlfenpeglelbdjmkbacnloglj");
    }

    #[test]
    fn invalid_config_file_is_replaced_with_valid_defaults() {
        let path = std::env::temp_dir().join(format!(
            "study-guardian-invalid-config-{}.json",
            std::process::id()
        ));
        fs::write(&path, r#"{"overlayVoiceText":"broken"#).expect("write invalid config");

        let config = load_or_default(&path).expect("load repaired config");
        let repaired = fs::read_to_string(&path).expect("read repaired config");
        let parsed = serde_json::from_str::<AppConfig>(&repaired);

        assert_eq!(config.banner_text, "快去学习");
        assert!(parsed.is_ok());
        let _ = fs::remove_file(path);
    }
}
