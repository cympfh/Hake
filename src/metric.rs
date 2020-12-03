use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Metric {
    pub metric: String,
    pub value: f64,
}

pub fn average(ms: Vec<Metric>) -> Option<Metric> {
    if ms.is_empty() {
        None
    } else {
        let name = ms[0].metric.clone();
        let avg = ms.iter().map(|m| m.value).sum::<f64>() / ms.len() as f64;
        Some(Metric {
            metric: name,
            value: avg,
        })
    }
}
