use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Metric {
    pub metric: String,
    pub value: f64,
}
