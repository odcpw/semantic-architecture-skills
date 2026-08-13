use semantic_architecture::coordinates::{
    CoordinateModel, CoordinateSeverity, analyze_coordinates,
};
use semantic_architecture::semantic::parse_no_duplicates;
use serde_json::{Value, json};

fn model_value() -> Value {
    json!({
        "schema_version": "kerf.coordinate-model.v1",
        "model_id": "coordinate-checker-test",
        "facts": [
            {
                "id": "synthetic_shares",
                "semantic_kind": "shares",
                "unit": "shares",
                "chart": "adjusted_total_return",
                "clock": {"effective": "signal_session"},
                "authority": "simulation",
                "identity": "ticker",
                "writers": ["backtest"],
                "evidence_closure": "exhaustive_finite"
            },
            {
                "id": "broker_shares",
                "semantic_kind": "shares",
                "unit": "shares",
                "chart": "raw_contract",
                "clock": {"effective": "signal_session", "observed_at": "snapshot_utc"},
                "authority": "broker",
                "identity": "conid",
                "writers": ["broker_snapshot"],
                "evidence_closure": "observed_only"
            },
            {
                "id": "config",
                "semantic_kind": "configuration",
                "clock": {"effective": "run_start"},
                "authority": "canonical_config",
                "evidence_closure": "complete_declared_corpus"
            }
        ],
        "conversions": [{
            "id": "corporate_action_conversion",
            "from": "synthetic_shares",
            "to": "broker_shares",
            "checker": {
                "command": ["check-corporate-actions", "--json"],
                "artifact_sha256": "0000000000000000000000000000000000000000000000000000000000000000"
            },
            "closure": "observed_only"
        }],
        "verification_squares": [{
            "id": "shares_square",
            "common_inputs": ["config"],
            "left_path": {"command": ["run-simulation"], "observation": "synthetic_shares"},
            "right_path": {"command": ["read-broker-snapshot"], "observation": "broker_shares"},
            "comparator": "exact_after_conversion",
            "closure": "observed_only",
            "falsifier": "first unequal converted share vector"
        }],
        "correspondence_claims": [{
            "id": "shares_correspond",
            "left": "synthetic_shares",
            "right": "broker_shares",
            "conversion": "corporate_action_conversion",
            "verification_square": "shares_square",
            "closure": "observed_only"
        }]
    })
}

fn parse(value: &Value) -> CoordinateModel {
    parse_no_duplicates(&serde_json::to_vec(value).expect("JSON must serialize"))
        .expect("coordinate model must parse")
}

#[test]
fn declared_square_without_receipt_is_not_correspondence_ready() {
    let analysis = analyze_coordinates(&parse(&model_value())).expect("model must validate");
    assert!(analysis.structurally_valid);
    assert!(analysis.ready_for_square_execution);
    assert!(!analysis.execution_bound);
    assert!(!analysis.ready_for_correspondence_claims);
    assert!(
        analysis
            .findings
            .iter()
            .any(|finding| finding.code == "square_execution_receipt_missing")
    );
}

#[test]
fn mismatched_coordinates_require_a_named_conversion() {
    let mut value = model_value();
    value["correspondence_claims"][0]
        .as_object_mut()
        .expect("claim must be an object")
        .remove("conversion");
    let analysis = analyze_coordinates(&parse(&value)).expect("model must validate");
    assert!(!analysis.ready_for_correspondence_claims);
    assert!(analysis.findings.iter().any(|finding| {
        finding.code == "coordinate_conversion_missing"
            && finding.severity == CoordinateSeverity::Blocker
    }));
}

#[test]
fn claim_closure_cannot_exceed_weakest_declared_evidence() {
    let mut value = model_value();
    value["correspondence_claims"][0]["closure"] = Value::String("exhaustive_finite".into());
    let analysis = analyze_coordinates(&parse(&value)).expect("model must validate");
    assert!(
        analysis
            .findings
            .iter()
            .any(|finding| finding.code == "closure_exceeds_evidence")
    );
}

#[test]
fn multiple_writers_need_an_authority_transfer_protocol() {
    let mut value = model_value();
    value["facts"][0]["writers"] = json!(["backtest", "manual_override"]);
    let analysis = analyze_coordinates(&parse(&value)).expect("model must validate");
    assert!(
        analysis
            .findings
            .iter()
            .any(|finding| { finding.code == "multiple_writers_without_authority_transfer" })
    );
    value["facts"][0]["authority_transfer"] = Value::String("signed override receipt".into());
    let analysis = analyze_coordinates(&parse(&value)).expect("model must validate");
    assert!(
        !analysis
            .findings
            .iter()
            .any(|finding| { finding.code == "multiple_writers_without_authority_transfer" })
    );
}

#[test]
fn conversion_checker_is_hash_bound() {
    let mut value = model_value();
    value["conversions"][0]["checker"]["artifact_sha256"] = Value::String("not-a-hash".into());
    let error = analyze_coordinates(&parse(&value)).expect_err("bad hash must fail structurally");
    assert_eq!(error.code, "invalid_input");
    assert!(error.message.contains("SHA-256"));
}
