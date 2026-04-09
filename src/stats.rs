use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Counters {
    pub urls_cleaned: u32,
    pub params_removed: u32,
    pub exits_unwrapped: u32,
    pub clipboard_checks: u32,
}

impl Counters {
    fn increment(
        &mut self,
        urls_cleaned: u32,
        params_removed: u32,
        exits_unwrapped: u32,
        clipboard_checks: u32,
    ) {
        self.urls_cleaned += urls_cleaned;
        self.params_removed += params_removed;
        self.exits_unwrapped += exits_unwrapped;
        self.clipboard_checks += clipboard_checks;
    }

    fn reset(&mut self) {
        *self = Self::default();
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SessionBucket {
    pub started_at: String,
    #[serde(flatten)]
    pub counters: Counters,
}

impl Default for SessionBucket {
    fn default() -> Self {
        Self {
            started_at: chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string(),
            counters: Counters::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DayBucket {
    pub date: String,
    #[serde(flatten)]
    pub counters: Counters,
}

impl Default for DayBucket {
    fn default() -> Self {
        Self {
            date: chrono::Local::now().format("%Y-%m-%d").to_string(),
            counters: Counters::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MonthBucket {
    pub month: String,
    #[serde(flatten)]
    pub counters: Counters,
}

impl Default for MonthBucket {
    fn default() -> Self {
        Self {
            month: chrono::Local::now().format("%Y-%m").to_string(),
            counters: Counters::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct YearBucket {
    pub year: String,
    #[serde(flatten)]
    pub counters: Counters,
}

impl Default for YearBucket {
    fn default() -> Self {
        Self {
            year: chrono::Local::now().format("%Y").to_string(),
            counters: Counters::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Stats {
    pub session: SessionBucket,
    pub today: DayBucket,
    pub month: MonthBucket,
    pub year: YearBucket,
    pub total: Counters,
}

impl Stats {
    pub fn increment(
        &mut self,
        urls_cleaned: u32,
        params_removed: u32,
        exits_unwrapped: u32,
        clipboard_checks: u32,
    ) {
        self.session.counters.increment(
            urls_cleaned,
            params_removed,
            exits_unwrapped,
            clipboard_checks,
        );
        self.today.counters.increment(
            urls_cleaned,
            params_removed,
            exits_unwrapped,
            clipboard_checks,
        );
        self.month.counters.increment(
            urls_cleaned,
            params_removed,
            exits_unwrapped,
            clipboard_checks,
        );
        self.year.counters.increment(
            urls_cleaned,
            params_removed,
            exits_unwrapped,
            clipboard_checks,
        );
        self.total.increment(
            urls_cleaned,
            params_removed,
            exits_unwrapped,
            clipboard_checks,
        );
    }

    pub fn check_rollovers(&mut self) {
        let now = chrono::Local::now();
        let current_date = now.format("%Y-%m-%d").to_string();
        let current_month = now.format("%Y-%m").to_string();
        let current_year = now.format("%Y").to_string();

        if self.year.year != current_year {
            self.year.counters.reset();
            self.year.year = current_year;
            self.month.counters.reset();
            self.month.month = current_month;
            self.today.counters.reset();
            self.today.date = current_date;
        } else if self.month.month != current_month {
            self.month.counters.reset();
            self.month.month = current_month;
            self.today.counters.reset();
            self.today.date = current_date;
        } else if self.today.date != current_date {
            self.today.counters.reset();
            self.today.date = current_date;
        }
    }

    pub fn reset_session(&mut self) {
        self.session = SessionBucket::default();
    }

    pub fn session_or_zero(&self, is_running: bool) -> Counters {
        if is_running {
            self.session.counters.clone()
        } else {
            Counters::default()
        }
    }
}

pub fn load(path: &Path) -> Stats {
    fs::read_to_string(path)
        .ok()
        .and_then(|content| toml::from_str(&content).ok())
        .unwrap_or_default()
}

pub fn save(stats: &Stats, path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Failed to create stats directory: {e}"))?;
    }
    let content = toml::to_string(stats).map_err(|e| format!("Failed to serialize stats: {e}"))?;
    fs::write(path, content).map_err(|e| format!("Failed to write stats file: {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_stats_has_zero_counters() {
        let stats = Stats::default();
        assert_eq!(stats.session.counters.urls_cleaned, 0);
        assert_eq!(stats.today.counters.urls_cleaned, 0);
        assert_eq!(stats.month.counters.urls_cleaned, 0);
        assert_eq!(stats.year.counters.urls_cleaned, 0);
        assert_eq!(stats.total.urls_cleaned, 0);
    }

    #[test]
    fn increment_adds_to_all_buckets() {
        let mut stats = Stats::default();
        stats.increment(1, 3, 0, 1);
        assert_eq!(stats.session.counters.urls_cleaned, 1);
        assert_eq!(stats.session.counters.params_removed, 3);
        assert_eq!(stats.session.counters.exits_unwrapped, 0);
        assert_eq!(stats.session.counters.clipboard_checks, 1);
        assert_eq!(stats.today.counters.urls_cleaned, 1);
        assert_eq!(stats.month.counters.urls_cleaned, 1);
        assert_eq!(stats.year.counters.urls_cleaned, 1);
        assert_eq!(stats.total.urls_cleaned, 1);
    }

    #[test]
    fn serialize_deserialize_roundtrip() {
        let mut stats = Stats::default();
        stats.increment(2, 5, 1, 3);
        let toml_str = toml::to_string(&stats).unwrap();
        let loaded: Stats = toml::from_str(&toml_str).unwrap();
        assert_eq!(loaded.total.urls_cleaned, 2);
        assert_eq!(loaded.total.params_removed, 5);
        assert_eq!(loaded.total.exits_unwrapped, 1);
        assert_eq!(loaded.total.clipboard_checks, 3);
        assert_eq!(loaded.session.counters.urls_cleaned, 2);
    }

    #[test]
    fn day_rollover_resets_today_only() {
        let mut stats = Stats::default();
        stats.increment(2, 5, 1, 10);
        stats.today.date = "2026-04-08".to_string();
        stats.check_rollovers();
        assert_eq!(
            stats.today.date,
            chrono::Local::now().format("%Y-%m-%d").to_string()
        );
        assert_eq!(stats.today.counters.urls_cleaned, 0);
        assert_eq!(stats.month.counters.urls_cleaned, 2);
        assert_eq!(stats.year.counters.urls_cleaned, 2);
        assert_eq!(stats.total.urls_cleaned, 2);
    }

    #[test]
    fn month_rollover_resets_today_and_month() {
        let mut stats = Stats::default();
        stats.increment(2, 5, 1, 10);
        stats.today.date = "2026-03-31".to_string();
        stats.month.month = "2026-03".to_string();
        stats.check_rollovers();
        assert_eq!(stats.today.counters.urls_cleaned, 0);
        assert_eq!(stats.month.counters.urls_cleaned, 0);
        assert_eq!(stats.year.counters.urls_cleaned, 2);
        assert_eq!(stats.total.urls_cleaned, 2);
    }

    #[test]
    fn year_rollover_resets_today_month_and_year() {
        let mut stats = Stats::default();
        stats.increment(2, 5, 1, 10);
        stats.today.date = "2025-12-31".to_string();
        stats.month.month = "2025-12".to_string();
        stats.year.year = "2025".to_string();
        stats.check_rollovers();
        assert_eq!(stats.today.counters.urls_cleaned, 0);
        assert_eq!(stats.month.counters.urls_cleaned, 0);
        assert_eq!(stats.year.counters.urls_cleaned, 0);
        assert_eq!(stats.total.urls_cleaned, 2);
    }

    #[test]
    fn reset_session_zeroes_session_only() {
        let mut stats = Stats::default();
        stats.increment(2, 5, 1, 10);
        stats.reset_session();
        assert_eq!(stats.session.counters.urls_cleaned, 0);
        assert_eq!(stats.session.counters.clipboard_checks, 0);
        assert_eq!(stats.total.urls_cleaned, 2);
    }

    #[test]
    fn session_zeroed_when_not_running() {
        let mut stats = Stats::default();
        stats.increment(5, 10, 2, 20);
        let zeroed = stats.session_or_zero(false);
        assert_eq!(zeroed.urls_cleaned, 0);
        assert_eq!(zeroed.params_removed, 0);

        let live = stats.session_or_zero(true);
        assert_eq!(live.urls_cleaned, 5);
    }

    #[test]
    fn save_and_load_roundtrip() {
        let dir = std::env::temp_dir().join("clink_test_stats_io");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("stats.toml");

        let mut stats = Stats::default();
        stats.increment(3, 7, 1, 15);
        save(&stats, &path).unwrap();

        let loaded = load(&path);
        assert_eq!(loaded.total.urls_cleaned, 3);
        assert_eq!(loaded.total.params_removed, 7);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_missing_file_returns_default() {
        let path = std::env::temp_dir().join("clink_test_stats_missing.toml");
        let _ = std::fs::remove_file(&path);
        let stats = load(&path);
        assert_eq!(stats.total.urls_cleaned, 0);
    }

    #[test]
    fn load_corrupt_file_returns_default() {
        let dir = std::env::temp_dir().join("clink_test_stats_corrupt");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("stats.toml");
        std::fs::write(&path, "this is not valid [[[ toml").unwrap();
        let stats = load(&path);
        assert_eq!(stats.total.urls_cleaned, 0);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
