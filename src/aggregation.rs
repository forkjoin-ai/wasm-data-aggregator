use super::{Observation, AggregationResult};
use std::collections::HashMap;

/// Aggregate observations into a single result
pub fn aggregate_observations<'a>(observations: &'a [&'a Observation]) -> AggregationResult {
    if observations.is_empty() {
        return AggregationResult::empty();
    }

    let mut sum = 0.0;
    let mut min = observations[0].value;
    let mut max = observations[0].value;

    for obs in observations {
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

    AggregationResult {
        sum,
        average,
        weighted_average: average, // Default to average if no weights
        min,
        max,
        count,
    }
}

/// Group observations by specified keys
pub fn group_observations<'a>(
    observations: &'a [&'a Observation],
    group_by: &[String],
) -> HashMap<String, Vec<&'a Observation>> {
    let mut groups: HashMap<String, Vec<&Observation>> = HashMap::new();

    for obs in observations {
        let mut key_parts = Vec::new();
        for group_key in group_by {
            if let Some(value) = obs.metadata.get(group_key) {
                // Convert JSON value to string representation
                let value_str = match value {
                    serde_json::Value::String(s) => s.clone(),
                    serde_json::Value::Number(n) => n.to_string(),
                    serde_json::Value::Bool(b) => b.to_string(),
                    _ => value.to_string(),
                };
                key_parts.push(value_str);
            } else {
                key_parts.push("null".to_string());
            }
        }
        let key = key_parts.join("|");
        groups.entry(key).or_insert_with(Vec::new).push(obs);
    }

    groups
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_aggregate_observations() {
        let obs1 = Observation {
            timestamp: 1000,
            value: 10.0,
            metadata: HashMap::new(),
        };
        let obs2 = Observation {
            timestamp: 2000,
            value: 20.0,
            metadata: HashMap::new(),
        };

        let observations = vec![&obs1, &obs2];
        let result = aggregate_observations(&observations);

        assert_eq!(result.count, 2);
        assert_eq!(result.sum, 30.0);
        assert_eq!(result.average, 15.0);
        assert_eq!(result.min, 10.0);
        assert_eq!(result.max, 20.0);
    }

    #[test]
    fn test_group_observations() {
        let mut metadata1 = HashMap::new();
        metadata1.insert("category".to_string(), serde_json::Value::String("A".to_string()));
        
        let mut metadata2 = HashMap::new();
        metadata2.insert("category".to_string(), serde_json::Value::String("B".to_string()));

        let obs1 = Observation {
            timestamp: 1000,
            value: 10.0,
            metadata: metadata1,
        };
        let obs2 = Observation {
            timestamp: 2000,
            value: 20.0,
            metadata: metadata2,
        };

        let observations = vec![&obs1, &obs2];
        let groups = group_observations(&observations, &["category".to_string()]);

        assert_eq!(groups.len(), 2);
        assert_eq!(groups.get("A").unwrap().len(), 1);
        assert_eq!(groups.get("B").unwrap().len(), 1);
    }
}
