use super::Observation;
use std::collections::HashMap;

/// Apply filters to an observation
pub fn apply_filters(obs: &Observation, filters: &HashMap<String, serde_json::Value>) -> bool {
    filters
        .iter()
        .all(|(key, value)| filter_matches(obs, key, value))
}

/// Evaluate one filter criterion. Bounds that are not numbers are ignored;
/// any other key must equal the observation's metadata value.
fn filter_matches(obs: &Observation, key: &str, value: &serde_json::Value) -> bool {
    match key {
        "minValue" => value.as_f64().map_or(true, |min| !(obs.value < min)),
        "maxValue" => value.as_f64().map_or(true, |max| !(obs.value > max)),
        "minTimestamp" => timestamp_bound(value).map_or(true, |min| i128::from(obs.timestamp) >= min),
        "maxTimestamp" => timestamp_bound(value).map_or(true, |max| i128::from(obs.timestamp) <= max),
        _ => obs.metadata.get(key) == Some(value),
    }
}

/// Integer timestamp bound. Widened to i128 so a negative bound compares
/// correctly against unsigned timestamps instead of wrapping to a huge u64.
fn timestamp_bound(value: &serde_json::Value) -> Option<i128> {
    value
        .as_u64()
        .map(i128::from)
        .or_else(|| value.as_i64().map(i128::from))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_apply_filters_min_value() {
        let obs = Observation {
            timestamp: 1000,
            value: 10.0,
            metadata: HashMap::new(),
        };

        let mut filters = HashMap::new();
        filters.insert(
            "minValue".to_string(),
            serde_json::Value::Number(serde_json::Number::from_f64(5.0).unwrap()),
        );
        
        assert!(apply_filters(&obs, &filters));
        
        filters.insert(
            "minValue".to_string(),
            serde_json::Value::Number(serde_json::Number::from_f64(15.0).unwrap()),
        );
        assert!(!apply_filters(&obs, &filters));
    }

    #[test]
    fn test_apply_filters_metadata() {
        let mut metadata = HashMap::new();
        metadata.insert("category".to_string(), serde_json::Value::String("A".to_string()));
        
        let obs = Observation {
            timestamp: 1000,
            value: 10.0,
            metadata,
        };

        let mut filters = HashMap::new();
        filters.insert("category".to_string(), serde_json::Value::String("A".to_string()));
        
        assert!(apply_filters(&obs, &filters));
        
        filters.insert("category".to_string(), serde_json::Value::String("B".to_string()));
        assert!(!apply_filters(&obs, &filters));
    }

    #[test]
    fn test_apply_filters_timestamp_bounds() {
        let obs = Observation {
            timestamp: 1000,
            value: 10.0,
            metadata: HashMap::new(),
        };

        let mut filters = HashMap::new();
        filters.insert("minTimestamp".to_string(), serde_json::json!(500));
        filters.insert("maxTimestamp".to_string(), serde_json::json!(1000));
        assert!(apply_filters(&obs, &filters));

        filters.insert("maxTimestamp".to_string(), serde_json::json!(999));
        assert!(!apply_filters(&obs, &filters));

        // A negative lower bound admits every timestamp rather than wrapping
        let mut filters = HashMap::new();
        filters.insert("minTimestamp".to_string(), serde_json::json!(-1));
        assert!(apply_filters(&obs, &filters));

        // A negative upper bound admits nothing
        let mut filters = HashMap::new();
        filters.insert("maxTimestamp".to_string(), serde_json::json!(-1));
        assert!(!apply_filters(&obs, &filters));
    }

    #[test]
    fn test_apply_filters_missing_metadata_rejects() {
        let obs = Observation {
            timestamp: 1000,
            value: 10.0,
            metadata: HashMap::new(),
        };
        let mut filters = HashMap::new();
        filters.insert("category".to_string(), serde_json::json!("A"));
        assert!(!apply_filters(&obs, &filters));
    }
}
