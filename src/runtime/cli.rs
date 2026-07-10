use crate::error::SableResult;
use crate::runtime::scenarios;

pub fn run() -> SableResult<()> {
    let scenario = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "final".to_string());
    let report = scenarios::run_named(&scenario)?;
    let json = serde_json::to_string_pretty(&report)
        .map_err(|error| crate::SableError::Serialization(error.to_string()))?;
    println!("{json}");
    Ok(())
}
