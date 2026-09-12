use wasm_bindgen::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Initialize panic hook for better error messages
#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();
}

mod aggregation;
mod filtering;
mod decay;

use aggregation::*;
use filtering::*;
use decay::*;

/// Observation with timestamp for weighted aggregation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Observation {
    pub timestamp: u64, // Unix timestamp in milliseconds
    pub value: f64,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Aggregation result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AggregationResult {
    pub sum: f64,
    pub average: f64,
    pub weighted_average: f64,
    pub min: f64,
    pub max: f64,
    pub count: usize,
}

/// Fallback JSON when an `AggregationResult` cannot be serialized.
const EMPTY_AGGREGATION_JSON: &str =
    "{\"sum\":0,\"average\":0,\"weightedAverage\":0,\"min\":0,\"max\":0,\"count\":0}";

impl AggregationResult {
    /// Result for an empty (or unparseable) observation set.
    pub fn empty() -> Self {
        AggregationResult {
            sum: 0.0,
            average: 0.0,
            weighted_average: 0.0,
            min: 0.0,
            max: 0.0,
            count: 0,
        }
    }

    fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| EMPTY_AGGREGATION_JSON.to_string())
    }
}

/// Filter observations by date range and apply exponential decay aggregation
/// 
/// # Arguments
/// * `observations_json` - JSON string of Observation array
/// * `time_window_ms` - Time window in milliseconds for exponential decay
/// * `current_time_ms` - Current timestamp in milliseconds
/// 
/// # Returns
/// JSON string of AggregationResult
#[wasm_bindgen]
pub fn aggregate_with_decay(
    observations_json: &str,
    time_window_ms: f64,
    current_time_ms: u64,
) -> String {
    // Parse JSON input
    let observations: Vec<Observation> = match serde_json::from_str(observations_json) {
        Ok(obs) => obs,
        Err(_) => return AggregationResult::empty().to_json(),
    };

    decay_weighted_aggregation(&observations, time_window_ms, current_time_ms).to_json()
}

/// Sum/average/min/max plus an average weighted by exponential decay of each
/// observation's age relative to `current_time_ms`.
fn decay_weighted_aggregation(
    observations: &[Observation],
    time_window_ms: f64,
    current_time_ms: u64,
) -> AggregationResult {
    if observations.is_empty() {
        return AggregationResult::empty();
    }

    let mut weighted_sum = 0.0;
    let mut weight_sum = 0.0;
    let mut sum = 0.0;
    let mut min = observations[0].value;
    let mut max = observations[0].value;

    for obs in observations {
        let age = (current_time_ms as f64) - (obs.timestamp as f64);
        let weight = calculate_decay_weight(age, time_window_ms);

        weighted_sum += obs.value * weight;
        weight_sum += weight;
        sum += obs.value;

        if obs.value < min {
            min = obs.value;
        }
        if obs.value > max {
            max = obs.value;
        }
    }

    let count = observations.len();
    let average = if count > 0 { sum / count as f64 } else { 0.0 };
    let weighted_average = if weight_sum > 0.0 { weighted_sum / weight_sum } else { 0.0 };

    AggregationResult {
        sum,
        average,
        weighted_average,
        min,
        max,
        count,
    }
}

/// Filter and aggregate observations by multiple dimensions
/// 
/// # Arguments
/// * `observations_json` - JSON string of Observation array
/// * `filters_json` - JSON string of filter criteria
/// * `group_by_json` - JSON string of grouping keys (optional)
/// 
/// # Returns
/// JSON string of aggregated results grouped by keys
#[wasm_bindgen]
pub fn filter_and_aggregate(
    observations_json: &str,
    filters_json: &str,
    group_by_json: &str,
) -> String {
    // Parse inputs
    let observations: Vec<Observation> = match serde_json::from_str(observations_json) {
        Ok(obs) => obs,
        Err(_) => return "{}".to_string(),
    };

    let filters: HashMap<String, serde_json::Value> = match serde_json::from_str(filters_json) {
        Ok(f) => f,
        Err(_) => return "{}".to_string(),
    };

    let group_by: Vec<String> = match serde_json::from_str(group_by_json) {
        Ok(g) => g,
        Err(_) => vec![],
    };

    // Apply filters
    let filtered: Vec<&Observation> = observations
        .iter()
        .filter(|obs| apply_filters(obs, &filters))
        .collect();

    // Group and aggregate
    if group_by.is_empty() {
        // No grouping - aggregate all filtered observations
        let result = aggregate_observations(&filtered);
        serde_json::to_string(&result).unwrap_or_else(|_| "{}".to_string())
    } else {
        // Group by specified keys
        let grouped = group_observations(&filtered, &group_by);
        let result: HashMap<String, AggregationResult> = grouped
            .into_iter()
            .map(|(key, obs)| (key, aggregate_observations(&obs)))
            .collect();
        serde_json::to_string(&result).unwrap_or_else(|_| "{}".to_string())
    }
}

/// Daily metric entry for a single day
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyMetric {
    pub date: String,
    pub count: usize,
    pub sum: f64,
    pub average: f64,
    pub min: f64,
    pub max: f64,
}

/// Calculate daily metrics aggregation (optimized for dashboard calculations)
///
/// Groups observations by day (using millisecond timestamps) and computes
/// per-day aggregation metrics within the specified date range.
///
/// # Arguments
/// * `observations_json` - JSON string of Observation array
/// * `date_range_json` - JSON string with `startTimestamp` and `endTimestamp` (u64 ms)
///
/// # Returns
/// JSON string of HashMap<String, DailyMetric> keyed by date string (YYYY-MM-DD approx day index)
#[wasm_bindgen]
pub fn calculate_daily_metrics(
    observations_json: &str,
    date_range_json: &str,
) -> String {
    let observations: Vec<Observation> = match serde_json::from_str(observations_json) {
        Ok(o) => o,
        Err(_) => return "{}".to_string(),
    };

    let date_range: HashMap<String, u64> = match serde_json::from_str(date_range_json) {
        Ok(d) => d,
        Err(_) => return "{}".to_string(),
    };

    let start_ts = date_range.get("startTimestamp").copied().unwrap_or(0);
    let end_ts = date_range.get("endTimestamp").copied().unwrap_or(u64::MAX);

    // Group in-range observations by day (ms / MS_PER_DAY)
    let mut daily: HashMap<u64, Vec<f64>> = HashMap::new();
    for obs in observations
        .iter()
        .filter(|obs| obs.timestamp >= start_ts && obs.timestamp <= end_ts)
    {
        daily.entry(obs.timestamp / MS_PER_DAY).or_insert_with(Vec::new).push(obs.value);
    }

    let result: HashMap<String, DailyMetric> = daily
        .iter()
        .map(|(day, values)| (format!("day-{}", day), DailyMetric::from_values(*day, values)))
        .collect();

    serde_json::to_string(&result).unwrap_or_else(|_| "{}".to_string())
}

const MS_PER_DAY: u64 = 86_400_000;

impl DailyMetric {
    /// Summarize one day's values; `day` is the day index since the Unix epoch.
    fn from_values(day: u64, values: &[f64]) -> Self {
        let count = values.len();
        let sum: f64 = values.iter().sum();
        DailyMetric {
            date: format!("{}", day * MS_PER_DAY),
            count,
            sum,
            average: if count > 0 { sum / count as f64 } else { 0.0 },
            min: values.iter().cloned().fold(f64::INFINITY, f64::min),
            max: values.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aggregate_with_decay() {
        let observations = vec![
            Observation {
                timestamp: 1000,
                value: 10.0,
                metadata: HashMap::new(),
            },
            Observation {
                timestamp: 2000,
                value: 20.0,
                metadata: HashMap::new(),
            },
        ];

        let json = serde_json::to_string(&observations).unwrap();
        let result = aggregate_with_decay(&json, 10000.0, 3000);
        let parsed: AggregationResult = serde_json::from_str(&result).unwrap();

        assert_eq!(parsed.count, 2);
        assert_eq!(parsed.sum, 30.0);
        assert_eq!(parsed.average, 15.0);
    }

    #[test]
    fn test_filter_and_aggregate() {
        let observations = vec![
            Observation {
                timestamp: 1000,
                value: 10.0,
                metadata: {
                    let mut m = HashMap::new();
                    m.insert("category".to_string(), serde_json::Value::String("A".to_string()));
                    m
                },
            },
            Observation {
                timestamp: 2000,
                value: 20.0,
                metadata: {
                    let mut m = HashMap::new();
                    m.insert("category".to_string(), serde_json::Value::String("B".to_string()));
                    m
                },
            },
        ];

        let json = serde_json::to_string(&observations).unwrap();
        let filters = "{}";
        let group_by = "[]";
        let result = filter_and_aggregate(&json, filters, group_by);

        let parsed: AggregationResult = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed.count, 2);
    }

    #[test]
    fn test_calculate_daily_metrics() {
        let ms_per_day: u64 = 86_400_000;
        let observations = vec![
            Observation {
                timestamp: ms_per_day * 10 + 1000,
                value: 5.0,
                metadata: HashMap::new(),
            },
            Observation {
                timestamp: ms_per_day * 10 + 2000,
                value: 15.0,
                metadata: HashMap::new(),
            },
            Observation {
                timestamp: ms_per_day * 11 + 1000,
                value: 25.0,
                metadata: HashMap::new(),
            },
        ];

        let json = serde_json::to_string(&observations).unwrap();
        let range = format!("{{\"startTimestamp\":{},\"endTimestamp\":{}}}", ms_per_day * 10, ms_per_day * 12);
        let result = calculate_daily_metrics(&json, &range);
        let parsed: HashMap<String, DailyMetric> = serde_json::from_str(&result).unwrap();

        assert_eq!(parsed.len(), 2);
        let day10 = parsed.get("day-10").unwrap();
        assert_eq!(day10.count, 2);
        assert_eq!(day10.sum, 20.0);
        assert_eq!(day10.average, 10.0);
        let day11 = parsed.get("day-11").unwrap();
        assert_eq!(day11.count, 1);
        assert_eq!(day11.sum, 25.0);
    }

    #[test]
    fn test_aggregate_with_decay_empty() {
        let result = aggregate_with_decay("[]", 10000.0, 3000);
        let parsed: AggregationResult = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed.count, 0);
        assert_eq!(parsed.sum, 0.0);
    }

    #[test]
    fn test_aggregate_with_decay_invalid_json() {
        let result = aggregate_with_decay("not json", 10000.0, 3000);
        let parsed: AggregationResult = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed.count, 0);
    }

    #[test]
    fn test_calculate_daily_metrics_empty() {
        let result = calculate_daily_metrics("[]", "{\"startTimestamp\":0,\"endTimestamp\":999999999}");
        let parsed: HashMap<String, DailyMetric> = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed.len(), 0);
    }
}
