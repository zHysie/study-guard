use crate::config::AppConfig;
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Classification {
    Studying,
    Distracting,
    Waiting,
}

pub fn classify_url(current_url: Option<&str>, config: &AppConfig) -> Classification {
    let Some(raw_url) = current_url.map(str::trim).filter(|url| !url.is_empty()) else {
        return Classification::Waiting;
    };

    let Ok(parsed) = Url::parse(raw_url) else {
        return Classification::Waiting;
    };

    let host = parsed.host_str().unwrap_or("").to_ascii_lowercase();
    if host.is_empty() {
        return Classification::Waiting;
    }

    if matches_blacklist_domain(&host, &config.domain_blacklist) {
        return Classification::Distracting;
    }

    if is_bilibili_host(&host) {
        if matches_any(raw_url, &config.video_whitelist)
            || matches_any(raw_url, &config.up_whitelist)
        {
            Classification::Studying
        } else {
            Classification::Distracting
        }
    } else {
        Classification::Waiting
    }
}

fn is_bilibili_host(host: &str) -> bool {
    host == "bilibili.com" || host.ends_with(".bilibili.com")
}

fn matches_blacklist_domain(host: &str, blacklist: &[String]) -> bool {
    blacklist.iter().any(|domain| {
        let domain = domain.trim().trim_start_matches('.').to_ascii_lowercase();
        !domain.is_empty() && (host == domain || host.ends_with(&format!(".{domain}")))
    })
}

fn matches_any(haystack: &str, needles: &[String]) -> bool {
    let haystack = haystack.to_ascii_lowercase();
    needles
        .iter()
        .map(|needle| needle.trim().to_ascii_lowercase())
        .any(|needle| !needle.is_empty() && haystack.contains(&needle))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bilibili_video_whitelist_is_studying() {
        let config = AppConfig {
            video_whitelist: vec!["BV1abc".into()],
            ..AppConfig::default()
        };

        let result = classify_url(Some("https://www.bilibili.com/video/BV1abc/?p=1"), &config);

        assert_eq!(result, Classification::Studying);
    }

    #[test]
    fn bilibili_up_whitelist_is_studying() {
        let config = AppConfig {
            up_whitelist: vec!["space.bilibili.com/12345".into()],
            ..AppConfig::default()
        };

        let result = classify_url(Some("https://space.bilibili.com/12345/video"), &config);

        assert_eq!(result, Classification::Studying);
    }

    #[test]
    fn bilibili_non_whitelisted_video_is_distracting() {
        let result = classify_url(
            Some("https://www.bilibili.com/video/BV9noise"),
            &AppConfig::default(),
        );

        assert_eq!(result, Classification::Distracting);
    }

    #[test]
    fn bilibili_home_and_search_are_distracting() {
        assert_eq!(
            classify_url(Some("https://www.bilibili.com/"), &AppConfig::default()),
            Classification::Distracting
        );
        assert_eq!(
            classify_url(
                Some("https://search.bilibili.com/all?keyword=rust"),
                &AppConfig::default()
            ),
            Classification::Distracting
        );
    }

    #[test]
    fn blacklist_domain_and_subdomain_are_distracting() {
        assert_eq!(
            classify_url(
                Some("https://www.xiaohongshu.com/explore"),
                &AppConfig::default()
            ),
            Classification::Distracting
        );
        assert_eq!(
            classify_url(Some("https://live.douyin.com/123"), &AppConfig::default()),
            Classification::Distracting
        );
    }

    #[test]
    fn neutral_or_empty_url_is_waiting() {
        assert_eq!(
            classify_url(
                Some("https://github.com/tauri-apps/tauri"),
                &AppConfig::default()
            ),
            Classification::Waiting
        );
        assert_eq!(
            classify_url(None, &AppConfig::default()),
            Classification::Waiting
        );
    }
}
