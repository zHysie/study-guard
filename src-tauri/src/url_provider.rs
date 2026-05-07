use serde::{Deserialize, Serialize};
use std::{
    fs, io,
    path::{Path, PathBuf},
    sync::Mutex,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

pub trait UrlProvider: Send + Sync {
    fn current_url(&self) -> Option<String>;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NativeUrlSnapshot {
    pub url: Option<String>,
    pub updated_at_ms: u128,
}

impl NativeUrlSnapshot {
    pub fn new(url: Option<String>, updated_at: SystemTime) -> Self {
        Self {
            url,
            updated_at_ms: updated_at
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis(),
        }
    }

    pub fn write(path: &Path, url: Option<String>) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let snapshot = Self::new(url, SystemTime::now());
        let content = serde_json::to_string_pretty(&snapshot)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
        fs::write(path, content)
    }

    fn is_fresh(&self, max_age: Duration, now: SystemTime) -> bool {
        let Ok(now_ms) = now.duration_since(UNIX_EPOCH) else {
            return false;
        };
        now_ms.as_millis().saturating_sub(self.updated_at_ms) <= max_age.as_millis()
    }
}

#[derive(Debug)]
pub struct UrlProviderState {
    mock_url: Mutex<Option<String>>,
    native_url_path: PathBuf,
    native_max_age: Duration,
}

impl UrlProviderState {
    pub fn new(native_url_path: PathBuf) -> Self {
        Self {
            mock_url: Mutex::new(None),
            native_url_path,
            native_max_age: Duration::from_secs(10 * 60),
        }
    }

    pub fn set_url(&self, url: Option<String>) {
        let sanitized = url.and_then(|value| {
            let trimmed = value.trim().to_string();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed)
            }
        });
        *self.mock_url.lock().expect("mock url lock poisoned") = sanitized;
    }
}

impl UrlProvider for UrlProviderState {
    fn current_url(&self) -> Option<String> {
        let mock_url = self
            .mock_url
            .lock()
            .expect("mock url lock poisoned")
            .clone();
        if mock_url.is_some() {
            return mock_url;
        }

        read_native_url(
            &self.native_url_path,
            self.native_max_age,
            SystemTime::now(),
        )
    }
}

fn read_native_url(path: &Path, max_age: Duration, now: SystemTime) -> Option<String> {
    let content = fs::read_to_string(path).ok()?;
    let snapshot = serde_json::from_str::<NativeUrlSnapshot>(&content).ok()?;
    if snapshot.is_fresh(max_age, now) {
        snapshot.url
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_url_snapshot_expires() {
        let snapshot = NativeUrlSnapshot::new(
            Some("https://github.com".into()),
            UNIX_EPOCH + Duration::from_secs(10),
        );

        assert!(snapshot.is_fresh(
            Duration::from_secs(10 * 60),
            UNIX_EPOCH + Duration::from_secs(609)
        ));
        assert!(!snapshot.is_fresh(
            Duration::from_secs(10 * 60),
            UNIX_EPOCH + Duration::from_secs(611)
        ));
    }
}
