use crate::classifier::Classification;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ActiveReminder {
    None,
    Banner,
    Overlay,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PolicyInput {
    pub classification: Classification,
    pub now_seconds: u64,
    pub idle_seconds: u64,
    pub idle_threshold_seconds: u64,
    pub overlay_distracting_seconds: u64,
    pub banner_delay_seconds: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PolicyOutput {
    pub active_reminder: ActiveReminder,
}

#[derive(Debug, Default)]
pub struct ReminderPolicy {
    distracting_started_at: Option<u64>,
    idle_overlay_active: bool,
}

impl ReminderPolicy {
    pub fn update(&mut self, input: PolicyInput) -> PolicyOutput {
        if self.idle_overlay_active && input.idle_seconds < input.idle_threshold_seconds {
            self.reset();
            return none();
        }

        if input.idle_seconds >= input.idle_threshold_seconds {
            self.idle_overlay_active = true;
            return overlay();
        }

        if input.classification == Classification::Studying {
            self.reset();
            return none();
        }

        if input.classification != Classification::Distracting {
            self.distracting_started_at = None;
            return none();
        }

        let started_at = *self.distracting_started_at.get_or_insert(input.now_seconds);
        let distracting_seconds = input.now_seconds.saturating_sub(started_at);

        if distracting_seconds >= input.overlay_distracting_seconds {
            overlay()
        } else if distracting_seconds >= input.banner_delay_seconds {
            banner()
        } else {
            none()
        }
    }

    pub fn reset(&mut self) {
        self.distracting_started_at = None;
        self.idle_overlay_active = false;
    }

    pub fn distracting_seconds(&self, now_seconds: u64) -> u64 {
        self.distracting_started_at
            .map(|started_at| now_seconds.saturating_sub(started_at))
            .unwrap_or(0)
    }
}

fn none() -> PolicyOutput {
    PolicyOutput {
        active_reminder: ActiveReminder::None,
    }
}

fn banner() -> PolicyOutput {
    PolicyOutput {
        active_reminder: ActiveReminder::Banner,
    }
}

fn overlay() -> PolicyOutput {
    PolicyOutput {
        active_reminder: ActiveReminder::Overlay,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(classification: Classification, now_seconds: u64) -> PolicyInput {
        PolicyInput {
            classification,
            now_seconds,
            idle_seconds: 0,
            idle_threshold_seconds: 60,
            overlay_distracting_seconds: 300,
            banner_delay_seconds: 5,
        }
    }

    #[test]
    fn neutral_site_while_active_does_not_remind() {
        let mut policy = ReminderPolicy::default();

        let result = policy.update(input(Classification::Waiting, 20));

        assert_eq!(result.active_reminder, ActiveReminder::None);
    }

    #[test]
    fn idle_threshold_triggers_overlay_for_any_url() {
        let mut policy = ReminderPolicy::default();
        let mut event = input(Classification::Studying, 60);
        event.idle_seconds = 60;

        let result = policy.update(event);

        assert_eq!(result.active_reminder, ActiveReminder::Overlay);
    }

    #[test]
    fn distracting_url_triggers_banner_after_five_seconds() {
        let mut policy = ReminderPolicy::default();

        assert_eq!(
            policy
                .update(input(Classification::Distracting, 10))
                .active_reminder,
            ActiveReminder::None
        );
        assert_eq!(
            policy
                .update(input(Classification::Distracting, 15))
                .active_reminder,
            ActiveReminder::Banner
        );
    }

    #[test]
    fn distracting_url_triggers_overlay_at_configured_minutes() {
        let mut policy = ReminderPolicy::default();
        let mut first = input(Classification::Distracting, 0);
        first.overlay_distracting_seconds = 60;
        let mut later = first;
        later.now_seconds = 60;

        policy.update(first);
        let result = policy.update(later);

        assert_eq!(result.active_reminder, ActiveReminder::Overlay);
    }

    #[test]
    fn overlay_takes_priority_over_banner() {
        let mut policy = ReminderPolicy::default();
        let mut first = input(Classification::Distracting, 0);
        first.overlay_distracting_seconds = 5;
        let mut later = first;
        later.now_seconds = 5;

        policy.update(first);
        let result = policy.update(later);

        assert_eq!(result.active_reminder, ActiveReminder::Overlay);
    }

    #[test]
    fn idle_overlay_closes_after_user_input_and_resets_distracting_timer() {
        let mut policy = ReminderPolicy::default();
        let mut idle = input(Classification::Distracting, 100);
        idle.idle_seconds = 60;
        assert_eq!(policy.update(idle).active_reminder, ActiveReminder::Overlay);

        let mut active = input(Classification::Distracting, 101);
        active.idle_seconds = 0;
        assert_eq!(policy.update(active).active_reminder, ActiveReminder::None);

        let later = input(Classification::Distracting, 105);
        assert_eq!(policy.update(later).active_reminder, ActiveReminder::None);
    }

    #[test]
    fn studying_closes_all_reminders() {
        let mut policy = ReminderPolicy::default();

        policy.update(input(Classification::Distracting, 0));
        assert_eq!(
            policy
                .update(input(Classification::Distracting, 5))
                .active_reminder,
            ActiveReminder::Banner
        );
        assert_eq!(
            policy
                .update(input(Classification::Studying, 6))
                .active_reminder,
            ActiveReminder::None
        );
    }
}
