use std::sync::Mutex;
use std::time::{Duration, Instant};
use vigilon_core::events::{RiskLevel, SecurityEvent};

pub struct Notifier {
    enabled: bool,
    min_risk: String,
    last_notify: Mutex<Option<Instant>>,
}

impl Notifier {
    pub fn new(enabled: bool, min_risk: String) -> Self {
        Self {
            enabled,
            min_risk,
            last_notify: Mutex::new(None),
        }
    }

    pub fn notify(&self, event: &SecurityEvent) {
        if !self.enabled {
            return;
        }

        let should_alert = match self.min_risk.to_uppercase().as_str() {
            "CRITICAL" => event.risk == RiskLevel::Critical,
            "HIGH" => event.risk == RiskLevel::High || event.risk == RiskLevel::Critical,
            "MEDIUM" => event.risk != RiskLevel::Info && event.risk != RiskLevel::Low,
            _ => true,
        };

        if !should_alert {
            return;
        }

        let now = Instant::now();
        if let Ok(last) = self.last_notify.lock()
            && let Some(prev) = *last
            && now.duration_since(prev) < Duration::from_secs(10)
        {
            return;
        }
        if let Ok(mut last) = self.last_notify.lock() {
            *last = Some(now);
        }

        let title = format!("Vigilon Security Alert [{}]", event.risk.as_str());
        let summary = event.why_summary();
        let body = format!("{}: {}", event.rule.as_str(), summary);

        #[cfg(windows)]
        {
            let _ = winrt_notification::Toast::new(winrt_notification::Toast::POWERSHELL_APP_ID)
                .title(&title)
                .text1(&body)
                .duration(winrt_notification::Duration::Short)
                .show();
        }

        #[cfg(not(windows))]
        {
            let _ = (title, body);
        }
    }
}
