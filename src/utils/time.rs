//! Time utilities for IPPAN
//! 
//! This module provides time-related functions used throughout the IPPAN codebase.

use std::time::{SystemTime, UNIX_EPOCH, Duration, Instant};
use chrono::{DateTime, Utc, TimeZone};


/// IPPAN Time precision in microseconds
pub const IPPAN_TIME_PRECISION: u64 = 100; // 0.1 microseconds

/// Get current system time in microseconds since epoch
pub fn current_time_micros() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_micros() as u64
}

/// Get current system time in nanoseconds since epoch
pub fn current_time_nanos() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64
}

/// Get current system time in seconds since epoch
pub fn current_time_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Convert microseconds to IPPAN Time units
pub fn micros_to_ippan_time(micros: u64) -> u64 {
    micros * 10 // 0.1 microsecond precision
}

/// Convert IPPAN Time units to microseconds
pub fn ippan_time_to_micros(ippan_time: u64) -> u64 {
    ippan_time / 10
}

/// Get current IPPAN Time
pub fn current_ippan_time() -> u64 {
    micros_to_ippan_time(current_time_micros())
}

/// Convert timestamp to DateTime
pub fn timestamp_to_datetime(timestamp: u64) -> chrono::DateTime<chrono::Utc> {
    match Utc.timestamp_opt(timestamp as i64, 0) {
        chrono::LocalResult::Single(dt) => dt,
        _ => Utc::now(),
    }
}

/// Convert DateTime to timestamp
pub fn datetime_to_timestamp(datetime: &DateTime<Utc>) -> u64 {
    datetime.timestamp() as u64
}

/// Format timestamp as ISO 8601 string
pub fn format_timestamp_iso(timestamp: u64) -> String {
    let datetime = timestamp_to_datetime(timestamp);
    datetime.to_rfc3339()
}

/// Parse ISO 8601 string to timestamp
pub fn parse_timestamp_iso(iso_string: &str) -> Result<u64, Box<dyn std::error::Error>> {
    let datetime = DateTime::parse_from_rfc3339(iso_string)?;
    Ok(datetime_to_timestamp(&datetime.with_timezone(&Utc)))
}

/// Get time difference in seconds
pub fn time_diff_secs(timestamp1: u64, timestamp2: u64) -> i64 {
    timestamp1 as i64 - timestamp2 as i64
}

/// Get time difference in microseconds
pub fn time_diff_micros(timestamp1: u64, timestamp2: u64) -> i64 {
    timestamp1 as i64 - timestamp2 as i64
}

/// Check if a timestamp is within a time window
pub fn is_within_time_window(timestamp: u64, window_start: u64, window_duration: Duration) -> bool {
    let window_end = window_start + window_duration.as_secs();
    timestamp >= window_start && timestamp <= window_end
}

/// Get time until a future timestamp
pub fn time_until(timestamp: u64) -> Duration {
    let now = current_time_secs();
    if timestamp > now {
        Duration::from_secs(timestamp - now)
    } else {
        Duration::from_secs(0)
    }
}

/// Get time since a past timestamp
pub fn time_since(timestamp: u64) -> Duration {
    let now = current_time_secs();
    if now > timestamp {
        Duration::from_secs(now - timestamp)
    } else {
        Duration::from_secs(0)
    }
}

/// Sleep for a duration
pub async fn sleep(duration: Duration) {
    tokio::time::sleep(duration).await;
}

/// Sleep until a specific timestamp
pub async fn sleep_until(timestamp: u64) {
    let now = current_time_secs();
    if timestamp > now {
        let duration = Duration::from_secs(timestamp - now);
        tokio::time::sleep(duration).await;
    }
}

/// Get a timer that measures elapsed time
pub fn start_timer() -> Instant {
    Instant::now()
}

/// Get elapsed time from a timer
pub fn elapsed_time(timer: &Instant) -> Duration {
    timer.elapsed()
}

/// Get elapsed time in milliseconds
pub fn elapsed_millis(timer: &Instant) -> u64 {
    timer.elapsed().as_millis() as u64
}

/// Get elapsed time in microseconds
pub fn elapsed_micros(timer: &Instant) -> u64 {
    timer.elapsed().as_micros() as u64
}

/// Get elapsed time in nanoseconds
pub fn elapsed_nanos(timer: &Instant) -> u64 {
    timer.elapsed().as_nanos() as u64
}

/// Calculate median time from a list of timestamps
pub fn median_time(timestamps: &[u64]) -> Option<u64> {
    if timestamps.is_empty() {
        return None;
    }
    
    let mut sorted = timestamps.to_vec();
    sorted.sort_unstable();
    
    let len = sorted.len();
    if len % 2 == 0 {
        // Even number of elements, take average of middle two
        let mid1 = sorted[len / 2 - 1];
        let mid2 = sorted[len / 2];
        Some((mid1 + mid2) / 2)
    } else {
        // Odd number of elements, take middle element
        Some(sorted[len / 2])
    }
}

/// Calculate average time from a list of timestamps
pub fn average_time(timestamps: &[u64]) -> Option<u64> {
    if timestamps.is_empty() {
        return None;
    }
    
    let sum: u64 = timestamps.iter().sum();
    Some(sum / timestamps.len() as u64)
}

/// Remove outliers from a list of timestamps using IQR method
pub fn remove_outliers(timestamps: &[u64]) -> Vec<u64> {
    if timestamps.len() < 4 {
        return timestamps.to_vec();
    }
    
    let mut sorted = timestamps.to_vec();
    sorted.sort_unstable();
    
    let q1_index = sorted.len() / 4;
    let q3_index = 3 * sorted.len() / 4;
    
    let q1 = sorted[q1_index];
    let q3 = sorted[q3_index];
    let iqr = q3 - q1;
    
    let lower_bound = q1.saturating_sub(iqr * 3 / 2);
    let upper_bound = q3 + iqr * 3 / 2;
    
    sorted.into_iter()
        .filter(|&t| t >= lower_bound && t <= upper_bound)
        .collect()
}

/// Calculate time drift between local time and reference time
pub fn calculate_time_drift(local_time: u64, reference_time: u64) -> i64 {
    local_time as i64 - reference_time as i64
}

/// Adjust local time based on drift
pub fn adjust_time_for_drift(local_time: u64, drift: i64) -> u64 {
    if drift > 0 {
        local_time.saturating_sub(drift as u64)
    } else {
        local_time.saturating_add((-drift) as u64)
    }
}

/// Get time zone offset in seconds
pub fn get_timezone_offset() -> i32 {
    // This is a simplified implementation
    // In a real implementation, you would use a timezone library
    0
}

/// Convert local time to UTC
pub fn local_to_utc(local_time: u64) -> u64 {
    local_time.saturating_sub(get_timezone_offset() as u64)
}

/// Convert UTC to local time
pub fn utc_to_local(utc_time: u64) -> u64 {
    utc_time.saturating_add(get_timezone_offset() as u64)
}

/// Format duration as human readable string
pub fn format_duration(duration: Duration) -> String {
    let secs = duration.as_secs();
    let millis = duration.subsec_millis();
    
    if secs > 0 {
        format!("{}.{:03}s", secs, millis)
    } else {
        format!("{}ms", millis)
    }
}

/// Parse duration from string
pub fn parse_duration(duration_str: &str) -> Result<Duration, Box<dyn std::error::Error>> {
    // Simple parser for formats like "1s", "500ms", "1.5s"
    let s = duration_str.trim();
    
    if s.ends_with("ms") {
        let millis: u64 = s[..s.len()-2].parse()?;
        Ok(Duration::from_millis(millis))
    } else if s.ends_with('s') {
        let secs: f64 = s[..s.len()-1].parse()?;
        let millis = (secs * 1000.0) as u64;
        Ok(Duration::from_millis(millis))
    } else {
        // Assume seconds
        let secs: u64 = s.parse()?;
        Ok(Duration::from_secs(secs))
    }
}

/// Get current time as a formatted string
pub fn current_time_string() -> String {
    let now = Utc::now();
    now.format("%Y-%m-%d %H:%M:%S UTC").to_string()
}

/// Get current date as a formatted string
pub fn current_date_string() -> String {
    let now = Utc::now();
    now.format("%Y-%m-%d").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_conversions() {
        let now = current_time_secs();
        assert!(now > 0);
        
        let datetime = timestamp_to_datetime(now);
        let converted = datetime_to_timestamp(&datetime);
        assert_eq!(now, converted);
    }

    #[test]
    fn test_ippan_time_conversions() {
        let micros = 1000;
        let ippan_time = micros_to_ippan_time(micros);
        let converted = ippan_time_to_micros(ippan_time);
        assert_eq!(micros, converted);
    }

    #[test]
    fn test_median_time() {
        let timestamps = vec![1, 2, 3, 4, 5];
        assert_eq!(median_time(&timestamps), Some(3));
        
        let timestamps = vec![1, 2, 3, 4];
        assert_eq!(median_time(&timestamps), Some(2)); // Average of 2 and 3
    }

    #[test]
    fn test_average_time() {
        let timestamps = vec![1, 2, 3, 4, 5];
        assert_eq!(average_time(&timestamps), Some(3));
    }

    #[test]
    fn test_remove_outliers() {
        let timestamps = vec![1, 2, 3, 100, 4, 5];
        let filtered = remove_outliers(&timestamps);
        assert!(!filtered.contains(&100));
    }

    #[test]
    fn test_duration_formatting() {
        let duration = Duration::from_millis(1500);
        assert_eq!(format_duration(duration), "1.500s");
        
        let duration = Duration::from_millis(500);
        assert_eq!(format_duration(duration), "500ms");
    }

    #[test]
    fn test_duration_parsing() {
        assert_eq!(parse_duration("1s").unwrap(), Duration::from_secs(1));
        assert_eq!(parse_duration("500ms").unwrap(), Duration::from_millis(500));
        assert_eq!(parse_duration("1.5s").unwrap(), Duration::from_millis(1500));
    }

    #[tokio::test]
    async fn test_sleep() {
        let start = start_timer();
        sleep(Duration::from_millis(10)).await;
        let elapsed = elapsed_millis(&start);
        assert!(elapsed >= 10);
    }
}
