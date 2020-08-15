use serde::Deserialize;

#[derive(Deserialize)]
pub struct Metric {
    pub metric: String,
    pub value: f64,
}
