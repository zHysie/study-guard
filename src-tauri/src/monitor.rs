use crate::{
    audio,
    classifier::{classify_url, Classification},
    config::AppConfig,
    idle, paths,
    reminder_policy::{ActiveReminder, PolicyInput, ReminderPolicy},
    url_provider::{UrlProvider, UrlProviderState},
    windows,
};
use serde::{Deserialize, Serialize};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AudioAction {
    None,
    StartOverlay,
    StopOverlay,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReminderEffect {
    StartOverlayAudio,
    ApplyReminder,
    StopOverlayAudio,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MonitorStatus {
    pub classification: Classification,
    pub current_url: Option<String>,
    pub active_reminder: ActiveReminder,
    pub idle_seconds: u64,
    pub distracting_seconds: u64,
    pub paused: bool,
    pub url_source: String,
}

impl Default for MonitorStatus {
    fn default() -> Self {
        Self {
            classification: Classification::Waiting,
            current_url: None,
            active_reminder: ActiveReminder::None,
            idle_seconds: 0,
            distracting_seconds: 0,
            paused: false,
            url_source: "none".into(),
        }
    }
}

pub struct AppState {
    pub config_path: PathBuf,
    pub config: Mutex<AppConfig>,
    pub url_provider: UrlProviderState,
    pub policy: Mutex<ReminderPolicy>,
    pub status: Mutex<MonitorStatus>,
    pub paused: Mutex<bool>,
    started_at: Instant,
}

impl AppState {
    pub fn new(config_path: PathBuf, config: AppConfig, native_url_path: PathBuf) -> Self {
        Self {
            config_path,
            config: Mutex::new(config),
            url_provider: UrlProviderState::new(native_url_path),
            policy: Mutex::new(ReminderPolicy::default()),
            status: Mutex::new(MonitorStatus::default()),
            paused: Mutex::new(false),
            started_at: Instant::now(),
        }
    }

    fn now_seconds(&self) -> u64 {
        self.started_at.elapsed().as_secs()
    }
}

pub fn start_monitor(app: AppHandle, state: Arc<AppState>) {
    thread::spawn(move || loop {
        evaluate_once(&app, &state);
        thread::sleep(Duration::from_secs(1));
    });
}

pub fn evaluate_once(app: &AppHandle, state: &Arc<AppState>) -> MonitorStatus {
    let config = state.config.lock().expect("config lock poisoned").clone();
    let paused = *state.paused.lock().expect("paused lock poisoned");
    let current_url = state.url_provider.current_url();
    let url_source = if current_url.is_some() {
        "browser-or-mock".to_string()
    } else {
        "none".to_string()
    };
    let classification = classify_url(current_url.as_deref(), &config);
    let idle_seconds = idle::system_idle_seconds().unwrap_or(0);
    let now_seconds = state.now_seconds();

    let active_reminder = if paused {
        state.policy.lock().expect("policy lock poisoned").reset();
        ActiveReminder::None
    } else {
        let mut policy = state.policy.lock().expect("policy lock poisoned");
        let output = policy.update(PolicyInput {
            classification,
            now_seconds,
            idle_seconds,
            idle_threshold_seconds: config.idle_minutes as u64 * 60,
            overlay_distracting_seconds: config.overlay_distracting_minutes as u64 * 60,
            banner_delay_seconds: config.banner_delay_seconds as u64,
        });
        output.active_reminder
    };

    let distracting_seconds = state
        .policy
        .lock()
        .expect("policy lock poisoned")
        .distracting_seconds(now_seconds);

    let next_status = MonitorStatus {
        classification,
        current_url,
        active_reminder,
        idle_seconds,
        distracting_seconds,
        paused,
        url_source,
    };

    let mut previous = state.status.lock().expect("status lock poisoned");
    if *previous != next_status {
        let effects = reminder_effects(previous.active_reminder, active_reminder);
        *previous = next_status.clone();
        let _ = app.emit("monitor-status-changed", &next_status);
        for effect in effects {
            match effect {
                ReminderEffect::StartOverlayAudio => audio::start_overlay_audio(&config),
                ReminderEffect::ApplyReminder => {
                    windows::apply_reminder(app, active_reminder, &config)
                }
                ReminderEffect::StopOverlayAudio => audio::stop_overlay_audio(),
            }
        }
    }

    next_status
}

fn audio_action(previous: ActiveReminder, next: ActiveReminder) -> AudioAction {
    match (previous, next) {
        (ActiveReminder::Overlay, ActiveReminder::Overlay) => AudioAction::None,
        (_, ActiveReminder::Overlay) => AudioAction::StartOverlay,
        (ActiveReminder::Overlay, _) => AudioAction::StopOverlay,
        _ => AudioAction::None,
    }
}

fn reminder_effects(previous: ActiveReminder, next: ActiveReminder) -> Vec<ReminderEffect> {
    match audio_action(previous, next) {
        AudioAction::StartOverlay => vec![
            ReminderEffect::StartOverlayAudio,
            ReminderEffect::ApplyReminder,
        ],
        AudioAction::StopOverlay => vec![
            ReminderEffect::StopOverlayAudio,
            ReminderEffect::ApplyReminder,
        ],
        AudioAction::None => vec![ReminderEffect::ApplyReminder],
    }
}

pub fn default_native_url_path(config_path: &std::path::Path) -> PathBuf {
    let config_dir = config_path
        .parent()
        .map(PathBuf::from)
        .unwrap_or_else(paths::fallback_config_dir);
    paths::native_url_path_from_config_dir(config_dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audio_starts_only_when_entering_overlay() {
        assert_eq!(
            audio_action(ActiveReminder::Banner, ActiveReminder::Overlay),
            AudioAction::StartOverlay
        );
        assert_eq!(
            audio_action(ActiveReminder::None, ActiveReminder::Overlay),
            AudioAction::StartOverlay
        );
    }

    #[test]
    fn audio_does_not_restart_while_overlay_status_updates() {
        assert_eq!(
            audio_action(ActiveReminder::Overlay, ActiveReminder::Overlay),
            AudioAction::None
        );
    }

    #[test]
    fn audio_stops_when_leaving_overlay() {
        assert_eq!(
            audio_action(ActiveReminder::Overlay, ActiveReminder::Banner),
            AudioAction::StopOverlay
        );
        assert_eq!(
            audio_action(ActiveReminder::Overlay, ActiveReminder::None),
            AudioAction::StopOverlay
        );
    }

    #[test]
    fn entering_overlay_starts_audio_before_showing_window() {
        assert_eq!(
            reminder_effects(ActiveReminder::Banner, ActiveReminder::Overlay),
            vec![
                ReminderEffect::StartOverlayAudio,
                ReminderEffect::ApplyReminder
            ]
        );
    }
}
