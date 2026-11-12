//! Human-friendly byte sizes and durations.

use std::time::Duration;

const UNITS: [&str; 6] = ["B", "KB", "MB", "GB", "TB", "PB"];

/// Render a byte count the way a human would say it: `2.31 GB`, `812 MB`, `9 B`.
pub fn human(bytes: u64) -> String {
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    let mut value = bytes as f64;
    let mut unit = 0usize;
    while value >= 1024.0 && unit + 1 < UNITS.len() {
        value /= 1024.0;
        unit += 1;
    }
    // Keep the column narrow: no decimals once the number has three digits,
    // and no trailing `.00` on a value that happens to be exact.
    let exact = (value.round() - value).abs() < 0.005;
    let rendered = if value >= 100.0 || exact {
        format!("{value:.0}")
    } else if value >= 10.0 {
        format!("{value:.1}")
    } else {
        format!("{value:.2}")
    };
    format!("{rendered} {}", UNITS[unit])
}

/// Parse `--min-size` input: `512kb`, `1.5GB`, `800`, `2g`.
pub fn parse_size(input: &str) -> Result<u64, String> {
    let raw = input.trim().to_ascii_lowercase();
    if raw.is_empty() {
        return Err("empty size".into());
    }

    let (number, multiplier) = if let Some(rest) = raw.strip_suffix("tb") {
        (rest, 1u64 << 40)
    } else if let Some(rest) = raw.strip_suffix("gb") {
        (rest, 1u64 << 30)
    } else if let Some(rest) = raw.strip_suffix("mb") {
        (rest, 1u64 << 20)
    } else if let Some(rest) = raw.strip_suffix("kb") {
        (rest, 1u64 << 10)
    } else if let Some(rest) = raw.strip_suffix('b') {
        (rest, 1)
    } else if let Some(rest) = raw.strip_suffix('t') {
        (rest, 1u64 << 40)
    } else if let Some(rest) = raw.strip_suffix('g') {
        (rest, 1u64 << 30)
    } else if let Some(rest) = raw.strip_suffix('m') {
        (rest, 1u64 << 20)
    } else if let Some(rest) = raw.strip_suffix('k') {
        (rest, 1u64 << 10)
    } else {
        (raw.as_str(), 1)
    };

    let value: f64 = number
        .trim()
        .parse()
        .map_err(|_| format!("`{input}` is not a size (try `500mb` or `2gb`)"))?;
    if !value.is_finite() || value < 0.0 {
        return Err(format!("`{input}` is not a valid size"));
    }
    Ok((value * multiplier as f64).round() as u64)
}

/// Parse `--older-than` input: `12h`, `30d`, `6w`, bare numbers mean days.
pub fn parse_age(input: &str) -> Result<Duration, String> {
    let raw = input.trim().to_ascii_lowercase();
    if raw.is_empty() {
        return Err("empty duration".into());
    }

    let (number, seconds_per_unit) = if let Some(rest) = raw.strip_suffix('d') {
        (rest, 86_400f64)
    } else if let Some(rest) = raw.strip_suffix('w') {
        (rest, 7.0 * 86_400.0)
    } else if let Some(rest) = raw.strip_suffix('h') {
        (rest, 3_600f64)
    } else if let Some(rest) = raw.strip_suffix('m') {
        (rest, 60f64)
    } else if let Some(rest) = raw.strip_suffix('s') {
        (rest, 1f64)
    } else {
        (raw.as_str(), 86_400f64)
    };

    let value: f64 = number
        .trim()
        .parse()
        .map_err(|_| format!("`{input}` is not a duration (try `30d` or `12h`)"))?;
    if !value.is_finite() || value < 0.0 {
        return Err(format!("`{input}` is not a valid duration"));
    }
    Ok(Duration::from_secs(
        (value * seconds_per_unit).round() as u64
    ))
}

/// Short, scannable age label such as `3d ago` or `just now`.
pub fn describe_age(seconds: u64) -> String {
    if seconds < 60 {
        return "just now".into();
    }
    let minutes = seconds / 60;
    if minutes < 60 {
        return format!("{minutes}m ago");
    }
    let hours = minutes / 60;
    if hours < 24 {
        return format!("{hours}h ago");
    }
    let days = hours / 24;
    if days < 30 {
        return format!("{days}d ago");
    }
    let months = days / 30;
    if months < 12 {
        return format!("{months}mo ago");
    }
    format!("{}y ago", months / 12)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_values_lose_the_trailing_zeros() {
        assert_eq!(human(1024), "1 KB");
        assert_eq!(human(1024 * 1024), "1 MB");
    }

    #[test]
    fn formats_bytes() {
        assert_eq!(human(0), "0 B");
        assert_eq!(human(999), "999 B");
        assert_eq!(human(1024), "1 KB");
        assert_eq!(human(1536), "1.50 KB");
        assert_eq!(human(1024 * 1024 * 1024 + 512 * 1024 * 1024), "1.50 GB");
        assert_eq!(human(2 * 1024 * 1024 * 1024 * 1024), "2 TB");
    }

    #[test]
    fn parses_sizes() {
        assert_eq!(parse_size("512").unwrap(), 512);
        assert_eq!(parse_size("1kb").unwrap(), 1024);
        assert_eq!(parse_size("1.5 MB").unwrap(), 1_572_864);
        assert_eq!(parse_size("2g").unwrap(), 2 * (1 << 30));
        assert!(parse_size("huge").is_err());
    }

    #[test]
    fn parses_ages() {
        assert_eq!(parse_age("30").unwrap().as_secs(), 30 * 86_400);
        assert_eq!(parse_age("12h").unwrap().as_secs(), 12 * 3_600);
        assert_eq!(parse_age("2w").unwrap().as_secs(), 14 * 86_400);
        assert!(parse_age("soon").is_err());
    }

    #[test]
    fn describes_age() {
        assert_eq!(describe_age(10), "just now");
        assert_eq!(describe_age(90), "1m ago");
        assert_eq!(describe_age(3 * 86_400), "3d ago");
    }
}
