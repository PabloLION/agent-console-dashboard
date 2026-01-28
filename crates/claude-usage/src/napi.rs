//! N-API bindings for Node.js.
//!
//! This module provides JavaScript/TypeScript bindings for the claude-usage crate
//! using napi-rs. Enable the `napi` feature to use these bindings.
//!
//! ## Usage from Node.js
//!
//! ```javascript
//! const { getUsage } = require('claude-usage');
//!
//! const usage = getUsage();
//! console.log(`5h utilization: ${usage.fiveHour.utilization}%`);
//! console.log(`7d utilization: ${usage.sevenDay.utilization}%`);
//! ```

use napi_derive::napi;

/// Usage data for a specific time period.
#[napi(object)]
pub struct JsUsagePeriod {
    /// Percentage of quota used (0.0 - 100.0+).
    pub utilization: f64,
    /// When this period's quota resets (ISO 8601 format).
    pub resets_at: String,
}

/// Extra usage billing information.
#[napi(object)]
pub struct JsExtraUsage {
    /// Whether extra usage billing is enabled.
    pub is_enabled: bool,
    /// Amount of extra usage consumed (in dollars, if enabled).
    pub amount_used: Option<f64>,
    /// Extra usage spending limit (in dollars, if set).
    pub limit: Option<f64>,
}

/// Main usage data returned by `getUsage()`.
#[napi(object)]
pub struct JsUsageData {
    /// 5-hour rolling window usage.
    pub five_hour: JsUsagePeriod,
    /// 7-day rolling window usage.
    pub seven_day: JsUsagePeriod,
    /// 7-day Sonnet-specific usage (if applicable).
    pub seven_day_sonnet: Option<JsUsagePeriod>,
    /// Extra usage billing information (if enabled).
    pub extra_usage: Option<JsExtraUsage>,
}

impl From<&crate::types::UsagePeriod> for JsUsagePeriod {
    fn from(period: &crate::types::UsagePeriod) -> Self {
        Self {
            utilization: period.utilization,
            resets_at: period.resets_at.to_rfc3339(),
        }
    }
}

impl From<&crate::types::ExtraUsage> for JsExtraUsage {
    fn from(extra: &crate::types::ExtraUsage) -> Self {
        Self {
            is_enabled: extra.is_enabled,
            amount_used: extra.amount_used,
            limit: extra.limit,
        }
    }
}

impl From<crate::types::UsageData> for JsUsageData {
    fn from(usage: crate::types::UsageData) -> Self {
        Self {
            five_hour: JsUsagePeriod::from(&usage.five_hour),
            seven_day: JsUsagePeriod::from(&usage.seven_day),
            seven_day_sonnet: usage.seven_day_sonnet.as_ref().map(JsUsagePeriod::from),
            extra_usage: usage.extra_usage.as_ref().map(JsExtraUsage::from),
        }
    }
}

/// Fetch current Claude API usage data (synchronous).
///
/// Retrieves credentials from platform-specific storage and calls the
/// Anthropic usage API.
///
/// @returns Usage data with 5-hour and 7-day utilization percentages
/// @throws Error if credentials are not found or API call fails
#[napi]
#[cfg(feature = "blocking")]
pub fn get_usage() -> napi::Result<JsUsageData> {
    let usage = crate::get_usage().map_err(|e| napi::Error::from_reason(e.to_string()))?;
    Ok(JsUsageData::from(usage))
}

/// Check if usage for a period is on pace.
///
/// @param utilization - Current utilization percentage (0.0 - 100.0+)
/// @param resets_at - ISO 8601 timestamp when the period resets
/// @param period_hours - Total duration of the period in hours (5 for 5-hour, 168 for 7-day)
/// @returns true if current utilization is sustainable
/// @throws Error if resets_at cannot be parsed as RFC3339 timestamp
#[napi]
pub fn is_on_pace(utilization: f64, resets_at: String, period_hours: u32) -> napi::Result<bool> {
    let parsed = chrono::DateTime::parse_from_rfc3339(&resets_at)
        .map_err(|_| napi::Error::from_reason("Invalid RFC3339 timestamp for resets_at"))?;

    let period = crate::types::UsagePeriod {
        utilization,
        resets_at: parsed.with_timezone(&chrono::Utc),
    };
    Ok(period.is_on_pace(period_hours))
}
