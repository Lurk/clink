use crate::{runtime, stats};

fn format_row(label: &str, c: &stats::Counters) -> String {
    format!(
        "{:<16} {:>12}  {:>14}  {:>15}  {:>16}",
        label, c.urls_cleaned, c.params_removed, c.exits_unwrapped, c.clipboard_checks
    )
}

fn format_stats_table(stats: &stats::Stats, is_running: bool) -> String {
    let header = format!(
        "{:<16} {:>12}  {:>14}  {:>15}  {:>16}",
        "", "URLs cleaned", "Params removed", "Exits unwrapped", "Clipboard checks"
    );
    let session = stats.session_or_zero(is_running);
    let rows = [
        format_row("Since restart", &session),
        format_row("Today", &stats.today.counters),
        format_row("This month", &stats.month.counters),
        format_row("This year", &stats.year.counters),
        format_row("Total", &stats.total),
    ];
    format!("{header}\n{}", rows.join("\n"))
}

#[allow(clippy::unnecessary_wraps)]
pub fn execute() -> Result<(), String> {
    let pid = runtime::read_pid();
    let is_running = pid.is_some_and(runtime::is_running);

    match pid {
        Some(pid) if runtime::is_running(pid) => {
            println!("clink is running (PID {pid})");
        }
        Some(pid) => {
            println!("clink is not running (stale PID file for PID {pid})");
            runtime::remove_pid_file();
        }
        None => {
            println!("clink is not running");
        }
    }

    let stats_path = runtime::stats_file_path();
    let stats = stats::load(&stats_path);
    println!("\nStatistics:\n{}", format_stats_table(&stats, is_running));

    let log_path = runtime::log_file_path();
    println!("\nLog file: {}", log_path.display());

    let lines = runtime::read_last_log_lines(20);
    if lines.is_empty() {
        println!("(no log entries)");
    } else {
        println!("\nLast log entries:");
        for line in &lines {
            println!("  {line}");
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_stats_table_output() {
        let stats = stats::Stats {
            session: stats::SessionBucket {
                started_at: "2026-04-09T10:00:00".to_string(),
                counters: stats::Counters {
                    urls_cleaned: 3,
                    params_removed: 12,
                    exits_unwrapped: 1,
                    clipboard_checks: 847,
                },
            },
            today: stats::DayBucket {
                date: "2026-04-09".to_string(),
                counters: stats::Counters {
                    urls_cleaned: 5,
                    params_removed: 20,
                    exits_unwrapped: 2,
                    clipboard_checks: 1200,
                },
            },
            month: stats::MonthBucket {
                month: "2026-04".to_string(),
                counters: stats::Counters {
                    urls_cleaned: 50,
                    params_removed: 180,
                    exits_unwrapped: 10,
                    clipboard_checks: 28000,
                },
            },
            year: stats::YearBucket {
                year: "2026".to_string(),
                counters: stats::Counters {
                    urls_cleaned: 200,
                    params_removed: 800,
                    exits_unwrapped: 40,
                    clipboard_checks: 100000,
                },
            },
            total: stats::Counters {
                urls_cleaned: 500,
                params_removed: 2000,
                exits_unwrapped: 100,
                clipboard_checks: 300000,
            },
        };

        let output = format_stats_table(&stats, true);
        assert!(output.contains("Since restart"));
        assert!(output.contains("Today"));
        assert!(output.contains("This month"));
        assert!(output.contains("This year"));
        assert!(output.contains("Total"));
        assert!(output.contains("847"));
        assert!(output.contains("300000"));
    }

    #[test]
    fn format_stats_table_not_running_zeros_session() {
        let mut stats = stats::Stats::default();
        stats.increment(5, 10, 2, 20);
        let output = format_stats_table(&stats, false);
        let lines: Vec<&str> = output.lines().collect();
        let session_line = lines.iter().find(|l| l.contains("Since restart")).unwrap();
        let numbers: Vec<u32> = session_line
            .split_whitespace()
            .filter_map(|w| w.parse().ok())
            .collect();
        assert_eq!(numbers, vec![0, 0, 0, 0]);
    }
}
