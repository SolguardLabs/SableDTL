use std::process::Command;

use serde_json::Value;

fn run_scenario(name: &str) -> Value {
    let output = Command::new(env!("CARGO_BIN_EXE_sable_dtl"))
        .arg(name)
        .output()
        .expect("scenario command should execute");
    assert!(
        output.status.success(),
        "scenario failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("scenario output should be json")
}

#[test]
fn valid_scenario_is_balanced() {
    let report = run_scenario("valid");
    assert_eq!(report["conservation_ok"], true);
    assert_eq!(report["invoices"]["accounting_open"]["cents"], 0);
    assert_eq!(report["settlement"]["obligations"], 0);
}

#[test]
fn final_scenario_posts_expected_claim_count() {
    let report = run_scenario("final");
    assert_eq!(report["conservation_ok"], true);
    assert_eq!(report["settlement"]["claims"], 2);
    assert_eq!(report["settlement"]["posted_claims"], 2);
}
