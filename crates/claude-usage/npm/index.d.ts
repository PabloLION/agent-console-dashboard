/**
 * Usage data for a specific time period.
 */
export interface UsagePeriod {
  /** Percentage of quota used (0.0 - 100.0+). */
  utilization: number;
  /** When this period's quota resets (ISO 8601 format). */
  resetsAt: string;
}

/**
 * Extra usage billing information.
 */
export interface ExtraUsage {
  /** Whether extra usage billing is enabled. */
  isEnabled: boolean;
  /** Amount of extra usage consumed (in dollars, if enabled). */
  amountUsed?: number;
  /** Extra usage spending limit (in dollars, if set). */
  limit?: number;
}

/**
 * Main usage data returned by `getUsage()`.
 */
export interface UsageData {
  /** 5-hour rolling window usage. */
  fiveHour: UsagePeriod;
  /** 7-day rolling window usage. */
  sevenDay: UsagePeriod;
  /** 7-day Sonnet-specific usage (if applicable). */
  sevenDaySonnet?: UsagePeriod;
  /** Extra usage billing information (if enabled). */
  extraUsage?: ExtraUsage;
}

/**
 * Fetch current Claude API usage data (synchronous).
 *
 * Retrieves credentials from platform-specific storage and calls the
 * Anthropic usage API.
 *
 * @returns Usage data with 5-hour and 7-day utilization percentages
 * @throws Error if credentials are not found or API call fails
 */
export function getUsage(): UsageData;

/**
 * Check if usage for a period is on pace.
 *
 * @param utilization - Current utilization percentage (0.0 - 100.0+)
 * @param resetsAt - ISO 8601 timestamp when the period resets
 * @param periodHours - Total duration of the period in hours (5 for 5-hour, 168 for 7-day)
 * @returns true if current utilization is sustainable
 *
 * @example
 * const usage = getUsage();
 * const fiveHourOnPace = isOnPace(usage.fiveHour.utilization, usage.fiveHour.resetsAt, 5);
 * const sevenDayOnPace = isOnPace(usage.sevenDay.utilization, usage.sevenDay.resetsAt, 168);
 */
export function isOnPace(
  utilization: number,
  resetsAt: string,
  periodHours: number
): boolean;
