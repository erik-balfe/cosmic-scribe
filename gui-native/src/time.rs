use chrono::{DateTime, Local, NaiveDateTime, TimeZone, Utc};

/// Parse recording timestamp.
///
/// Filenames use **local** wall time (`YYYY-MM-DD_HH-MM-SS`). API human form is
/// `YYYY-MM-DD HH:MM:SS` (same local components). Interpreting those as UTC made
/// relative labels negative for non-UTC zones → everything showed as "just now".
pub fn parse_recording_ts(ts: &str) -> Option<DateTime<Utc>> {
    if ts.is_empty() {
        return None;
    }
    let s = ts.trim();

    // RFC3339 with offset (if we ever store that)
    if let Ok(d) = DateTime::parse_from_rfc3339(s) {
        return Some(d.with_timezone(&Utc));
    }

    let normalized = s.replacen(' ', "T", 1);
    let naive = NaiveDateTime::parse_from_str(&normalized, "%Y-%m-%dT%H:%M:%S")
        .or_else(|_| NaiveDateTime::parse_from_str(&normalized, "%Y-%m-%dT%H:%M:%S%.f"))
        .or_else(|_| {
            // Filename-style still sometimes leaked through: 2026-05-24_20-44-19
            let with_colons = normalized.replace('_', "T");
            // 2026-05-24T20-44-19
            let fixed = {
                let mut parts = with_colons.split('T');
                let date = parts.next().unwrap_or("");
                let time = parts.next().unwrap_or("").replace('-', ":");
                format!("{date}T{time}")
            };
            NaiveDateTime::parse_from_str(&fixed, "%Y-%m-%dT%H:%M:%S")
        })
        .ok()?;

    // Treat naive components as **local** wall time, then convert to UTC.
    Local
        .from_local_datetime(&naive)
        .single()
        .map(|l| l.with_timezone(&Utc))
        .or_else(|| {
            // Ambiguous/skipped local time (DST): fall back to UTC interpretation
            Some(naive.and_utc())
        })
}

pub fn format_recording_time(ts: &str) -> String {
    parse_recording_ts(ts)
        .map(|d| {
            d.with_timezone(&Local)
                .format("%b %-d, %-I:%M %p")
                .to_string()
        })
        .unwrap_or_else(|| ts.to_string())
}

pub fn relative_recording_time(ts: &str) -> String {
    let Some(d) = parse_recording_ts(ts) else {
        return ts.to_string();
    };
    let diff = Utc::now().signed_duration_since(d);
    let mins = diff.num_minutes();
    // Future / timezone skew: show absolute rather than "just now"
    if mins < 0 {
        return format_recording_time(ts);
    }
    if mins < 1 {
        return "just now".into();
    }
    if mins < 60 {
        return format!("{mins}m ago");
    }
    let hours = diff.num_hours();
    if hours < 24 {
        return format!("{hours}h ago");
    }
    let days = diff.num_days();
    if days < 7 {
        return format!("{days}d ago");
    }
    if days < 30 {
        return format!("{days}d ago");
    }
    let weeks = days / 7;
    if weeks < 5 {
        return format!("{weeks}w ago");
    }
    d.with_timezone(&Local).format("%b %-d, %Y").to_string()
}

pub fn parse_duration_secs(value: &str) -> u64 {
    value
        .strip_suffix('s')
        .and_then(|s| s.parse().ok())
        .unwrap_or(0)
}

pub fn format_duration(value: &str) -> String {
    let secs = parse_duration_secs(value);
    if secs < 1 {
        return "<1s".into();
    }
    if secs < 60 {
        return format!("{secs}s");
    }
    let m = secs / 60;
    let s = secs % 60;
    if m < 60 {
        return if s == 0 {
            format!("{m}m")
        } else {
            format!("{m}m {s}s")
        };
    }
    let h = m / 60;
    let rm = m % 60;
    if rm > 0 && s > 0 {
        format!("{h}h {rm}m {s}s")
    } else if rm > 0 {
        format!("{h}h {rm}m")
    } else {
        format!("{h}h")
    }
}

pub struct TimeLabels {
    pub primary: String,
    pub tooltip: String,
}

pub fn recording_time_labels(ts: &str, absolute: bool) -> TimeLabels {
    let absolute_label = format_recording_time(ts);
    let relative_label = relative_recording_time(ts);
    if absolute {
        TimeLabels {
            primary: absolute_label,
            tooltip: relative_label,
        }
    } else {
        TimeLabels {
            primary: relative_label,
            tooltip: absolute_label,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_human_local_ts() {
        let d = parse_recording_ts("2026-05-24 20:44:19").expect("parse");
        // Should not be "just now" months later
        let mins = Utc::now().signed_duration_since(d).num_minutes();
        assert!(mins > 60, "expected hours/days ago, got mins={mins}");
        let rel = relative_recording_time("2026-05-24 20:44:19");
        assert_ne!(rel, "just now");
    }

    #[test]
    fn old_recordings_not_just_now() {
        // Same shape as api::ts_to_human from filename 2026-05-24_16-58-46_…
        let rel = relative_recording_time("2026-05-24 16:58:46");
        assert_ne!(rel, "just now");
        assert!(
            rel.contains("ago") || rel.contains("2026") || rel.contains("May"),
            "unexpected relative label: {rel}"
        );
    }
}
