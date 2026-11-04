use std::cmp::Ordering;

use serde::{Deserialize, Serialize};

/// Hybrid logical clock timestamp. Ordered by wall time, then counter, then
/// device, so two devices never produce equal stamps and last-writer-wins is
/// deterministic.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Hlc {
    /// Milliseconds since the Unix epoch.
    pub wall_ms: i64,
    /// Logical counter for stamps within the same millisecond.
    pub counter: u32,
    pub device: String,
}

impl Ord for Hlc {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.wall_ms, self.counter, &self.device).cmp(&(other.wall_ms, other.counter, &other.device))
    }
}

impl PartialOrd for Hlc {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hlc(wall_ms: i64, counter: u32, device: &str) -> Hlc {
        Hlc { wall_ms, counter, device: device.into() }
    }

    #[test]
    fn orders_by_wall_then_counter_then_device() {
        assert!(hlc(1, 9, "z") < hlc(2, 0, "a"));
        assert!(hlc(2, 0, "z") < hlc(2, 1, "a"));
        assert!(hlc(2, 1, "a") < hlc(2, 1, "b"));
    }

    #[test]
    fn round_trips_through_json() {
        let h = hlc(1_761_472_800_000, 3, "s24u");
        let json = serde_json::to_string(&h).unwrap();
        assert_eq!(json, r#"{"wall_ms":1761472800000,"counter":3,"device":"s24u"}"#);
        assert_eq!(serde_json::from_str::<Hlc>(&json).unwrap(), h);
    }
}
