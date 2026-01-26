//! Type definitions for Anthropic usage API responses.
//!
//! This module defines the structures that map to the JSON response
//! from the Anthropic OAuth usage API.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Main usage data returned by [`get_usage()`](crate::get_usage).
///
/// Contains utilization data for different time periods.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UsageData {
    /// 5-hour rolling window usage.
    pub five_hour: UsagePeriod,

    /// 7-day rolling window usage.
    pub seven_day: UsagePeriod,

    /// 7-day Sonnet-specific usage (if applicable).
    #[serde(default)]
    pub seven_day_sonnet: Option<UsagePeriod>,

    /// Extra usage billing information (if enabled).
    #[serde(default)]
    pub extra_usage: Option<ExtraUsage>,
}

/// Usage data for a specific time period.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UsagePeriod {
    /// Percentage of quota used (0.0 - 100.0+).
    ///
    /// Values over 100.0 indicate quota exceeded.
    pub utilization: f64,

    /// When this period's quota resets.
    ///
    /// May be `None` if the reset time is not available from the API.
    #[serde(default)]
    pub resets_at: Option<DateTime<Utc>>,
}

/// Extra usage billing information.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExtraUsage {
    /// Whether extra usage billing is enabled.
    pub is_enabled: bool,

    /// Amount of extra usage consumed (in dollars, if enabled).
    #[serde(default)]
    pub amount_used: Option<f64>,

    /// Extra usage spending limit (in dollars, if set).
    #[serde(default)]
    pub limit: Option<f64>,
}

impl UsagePeriod {
    /// Calculate time remaining until this period resets.
    ///
    /// Returns `None` if reset time is not available from the API.
    /// Returns a negative duration if the reset time has passed.
    pub fn time_until_reset(&self) -> Option<chrono::TimeDelta> {
        self.resets_at.map(|reset| reset - Utc::now())
    }

    /// Calculate percentage of time period elapsed.
    ///
    /// # Arguments
    ///
    /// * `period_hours` - Total duration of the period in hours
    ///
    /// # Returns
    ///
    /// `None` if reset time is not available.
    /// Otherwise, percentage (0.0 - 100.0) of the period that has elapsed.
    /// Clamped to valid range even if reset time is in the past.
    pub fn time_elapsed_percent(&self, period_hours: u32) -> Option<f64> {
        self.time_until_reset().map(|remaining| {
            let total_seconds = period_hours as f64 * 3600.0;
            let remaining_seconds = remaining.num_seconds() as f64;
            let elapsed_seconds = total_seconds - remaining_seconds;
            (elapsed_seconds / total_seconds * 100.0).clamp(0.0, 100.0)
        })
    }

    /// Check if usage is on pace with time elapsed.
    ///
    /// Returns `None` if reset time is not available.
    /// Returns `true` if utilization percentage is less than or equal to
    /// the percentage of time elapsed. This indicates sustainable usage
    /// that won't exceed quota before reset.
    ///
    /// # Arguments
    ///
    /// * `period_hours` - Total duration of the period in hours
    pub fn is_on_pace(&self, period_hours: u32) -> Option<bool> {
        self.time_elapsed_percent(period_hours)
            .map(|elapsed| self.utilization <= elapsed)
    }
}

impl UsageData {
    /// Check if 5-hour usage is on pace.
    ///
    /// Returns `None` if reset time is not available.
    /// Returns `true` if current 5-hour utilization is sustainable.
    pub fn five_hour_on_pace(&self) -> Option<bool> {
        self.five_hour.is_on_pace(5)
    }

    /// Check if 7-day usage is on pace.
    ///
    /// Returns `None` if reset time is not available.
    /// Returns `true` if current 7-day utilization is sustainable.
    pub fn seven_day_on_pace(&self) -> Option<bool> {
        self.seven_day.is_on_pace(7 * 24)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn sample_usage_period(utilization: f64, hours_until_reset: i64) -> UsagePeriod {
        UsagePeriod {
            utilization,
            resets_at: Some(Utc::now() + Duration::hours(hours_until_reset)),
        }
    }

    #[test]
    fn test_parse_full_response() {
        let json = r#"{
            "five_hour": {
                "utilization": 8.0,
                "resets_at": "2026-01-22T09:00:00Z"
            },
            "seven_day": {
                "utilization": 77.0,
                "resets_at": "2026-01-22T19:00:00Z"
            },
            "seven_day_sonnet": {
                "utilization": 0.0,
                "resets_at": "2026-01-25T00:00:00Z"
            },
            "extra_usage": {
                "is_enabled": false
            }
        }"#;

        let usage: UsageData = serde_json::from_str(json).expect("should parse");
        assert!((usage.five_hour.utilization - 8.0).abs() < f64::EPSILON);
        assert!((usage.seven_day.utilization - 77.0).abs() < f64::EPSILON);
        assert!(usage.seven_day_sonnet.is_some());
        assert!(usage.extra_usage.is_some());
        assert!(!usage.extra_usage.expect("extra_usage present").is_enabled);
    }

    #[test]
    fn test_parse_minimal_response() {
        let json = r#"{
            "five_hour": {
                "utilization": 50.0,
                "resets_at": "2026-01-22T09:00:00Z"
            },
            "seven_day": {
                "utilization": 25.0,
                "resets_at": "2026-01-22T19:00:00Z"
            }
        }"#;

        let usage: UsageData = serde_json::from_str(json).expect("should parse");
        assert!((usage.five_hour.utilization - 50.0).abs() < f64::EPSILON);
        assert!((usage.seven_day.utilization - 25.0).abs() < f64::EPSILON);
        assert!(usage.seven_day_sonnet.is_none());
        assert!(usage.extra_usage.is_none());
    }

    #[test]
    fn test_extra_usage_with_amounts() {
        let json = r#"{
            "five_hour": { "utilization": 0.0, "resets_at": "2026-01-22T09:00:00Z" },
            "seven_day": { "utilization": 0.0, "resets_at": "2026-01-22T19:00:00Z" },
            "extra_usage": {
                "is_enabled": true,
                "amount_used": 5.50,
                "limit": 100.0
            }
        }"#;

        let usage: UsageData = serde_json::from_str(json).expect("should parse");
        let extra = usage.extra_usage.expect("extra_usage should be present");
        assert!(extra.is_enabled);
        assert!((extra.amount_used.expect("amount") - 5.50).abs() < f64::EPSILON);
        assert!((extra.limit.expect("limit") - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_time_elapsed_percent_at_start() {
        // 5 hours remaining out of 5 = 0% elapsed
        let period = sample_usage_period(0.0, 5);
        let elapsed = period
            .time_elapsed_percent(5)
            .expect("reset time available");
        // Allow small margin for test execution time
        assert!(elapsed < 1.0, "elapsed should be near 0%: {}", elapsed);
    }

    #[test]
    fn test_time_elapsed_percent_at_half() {
        // 2.5 hours remaining out of 5 = 50% elapsed
        let period = UsagePeriod {
            utilization: 50.0,
            resets_at: Some(Utc::now() + Duration::minutes(150)), // 2.5 hours
        };
        let elapsed = period
            .time_elapsed_percent(5)
            .expect("reset time available");
        assert!(
            (elapsed - 50.0).abs() < 1.0,
            "elapsed should be near 50%: {}",
            elapsed
        );
    }

    #[test]
    fn test_time_elapsed_percent_no_reset() {
        let period = UsagePeriod {
            utilization: 50.0,
            resets_at: None,
        };
        assert!(period.time_elapsed_percent(5).is_none());
    }

    #[test]
    fn test_is_on_pace_when_behind() {
        // 50% time elapsed but only 30% usage = on pace
        let period = UsagePeriod {
            utilization: 30.0,
            resets_at: Some(Utc::now() + Duration::minutes(150)), // 50% remaining
        };
        assert!(
            period.is_on_pace(5).expect("reset time available"),
            "30% usage at 50% time should be on pace"
        );
    }

    #[test]
    fn test_is_on_pace_when_ahead() {
        // 50% time elapsed but 70% usage = not on pace
        let period = UsagePeriod {
            utilization: 70.0,
            resets_at: Some(Utc::now() + Duration::minutes(150)), // 50% remaining
        };
        assert!(
            !period.is_on_pace(5).expect("reset time available"),
            "70% usage at 50% time should not be on pace"
        );
    }

    #[test]
    fn test_five_hour_on_pace() {
        let usage = UsageData {
            five_hour: sample_usage_period(10.0, 4), // 10% used, ~20% time elapsed
            seven_day: sample_usage_period(50.0, 84), // 50% used, 50% time elapsed
            seven_day_sonnet: None,
            extra_usage: None,
        };
        assert!(usage.five_hour_on_pace().expect("reset time available"));
    }

    #[test]
    fn test_seven_day_on_pace() {
        let usage = UsageData {
            five_hour: sample_usage_period(80.0, 1),
            seven_day: sample_usage_period(40.0, 84), // ~50% time remaining, 40% used
            seven_day_sonnet: None,
            extra_usage: None,
        };
        assert!(usage.seven_day_on_pace().expect("reset time available"));
    }

    #[test]
    fn test_serialize_round_trip() {
        let now = Utc::now();
        let usage = UsageData {
            five_hour: UsagePeriod {
                utilization: 42.5,
                resets_at: Some(now),
            },
            seven_day: UsagePeriod {
                utilization: 88.0,
                resets_at: Some(now),
            },
            seven_day_sonnet: Some(UsagePeriod {
                utilization: 0.0,
                resets_at: Some(now),
            }),
            extra_usage: Some(ExtraUsage {
                is_enabled: true,
                amount_used: Some(10.0),
                limit: Some(50.0),
            }),
        };

        let json = serde_json::to_string(&usage).expect("serialize");
        let parsed: UsageData = serde_json::from_str(&json).expect("deserialize");

        assert!((parsed.five_hour.utilization - 42.5).abs() < f64::EPSILON);
        assert!((parsed.seven_day.utilization - 88.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_parse_null_resets_at() {
        let json = r#"{
            "five_hour": {
                "utilization": 50.0,
                "resets_at": null
            },
            "seven_day": {
                "utilization": 25.0,
                "resets_at": null
            }
        }"#;

        let usage: UsageData = serde_json::from_str(json).expect("should parse");
        assert!((usage.five_hour.utilization - 50.0).abs() < f64::EPSILON);
        assert!(usage.five_hour.resets_at.is_none());
        assert!(usage.seven_day.resets_at.is_none());
    }
}
