use semantic_architecture::semantic::{
    SemanticModel, analyze, derive_work, parse_no_duplicates, project,
};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;

fn repo_path(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(relative)
}

fn example_value() -> Value {
    serde_json::from_slice(include_bytes!("../examples/semantic-toggle.model.json"))
        .expect("semantic example must be JSON")
}

fn parse_value(value: &Value) -> Result<SemanticModel, kerf::report::ToolError> {
    parse_no_duplicates(&serde_json::to_vec(value).expect("value must serialize"))
}

#[test]
fn semantic_example_is_clean_and_projection_is_hash_bound() {
    let model = parse_value(&example_value()).expect("example must parse");
    let analysis = analyze(&model).expect("example must be structurally valid");
    assert!(analysis.structurally_valid);
    assert!(analysis.findings.is_empty());
    assert_eq!(analysis.entity_count, 11);

    let first = project(&model, "toggle-lts").expect("projection must validate");
    let second = project(&model, "toggle-lts").expect("projection must be deterministic");
    let greenfield =
        project(&model, "toggle-intent-lts").expect("greenfield projection must validate");
    assert_eq!(first.target_input_sha256, second.target_input_sha256);
    assert_eq!(first.manifest.omitted_entities, 1);
    assert_eq!(first.manifest.omitted_relations, 3);
    assert_eq!(first.target_schema, "kerf.lts.v1");
    assert_eq!(greenfield.manifest.subject, "intent");
    assert_eq!(greenfield.manifest.mapped_entities, 3);
    assert_eq!(greenfield.manifest.omitted_entities, 3);
}

#[test]
fn projection_requires_exact_entity_and_relation_accounting() {
    let mut value = example_value();
    value["projections"][1]["omitted_entities"] = Value::Array(Vec::new());
    let model = parse_value(&value).expect("schema shape must parse");
    let error = analyze(&model).expect_err("unaccounted scope entity must fail");
    assert_eq!(error.code, "invalid_input");
    assert!(error.message.contains("entity coverage"));
    assert!(error.message.contains("impl-authority"));
}

#[test]
fn projection_rejects_nonexistent_target_pointer() {
    let mut value = example_value();
    value["projections"][0]["entity_map"][0]["target_pointer"] =
        Value::String("/machine/not-present".to_owned());
    let model = parse_value(&value).expect("schema shape must parse");
    let error = project(&model, "toggle-lts").expect_err("bad pointer must fail");
    assert!(error.message.contains("does not resolve"));
}

#[test]
fn duplicate_json_members_are_rejected_at_nested_depth() {
    let bytes = br#"{
      "schema_version":"kerf.semantic-model.v1",
      "model_id":"duplicates",
      "subjects":[{"id":"intent","id":"shadow","kind":"intent"}],
      "scopes":[],"entities":[]
    }"#;
    let error = parse_no_duplicates::<SemanticModel>(bytes).expect_err("duplicate ID must fail");
    assert_eq!(error.code, "invalid_input");
    assert!(
        error
            .message
            .contains("duplicate JSON object member \"id\"")
    );
}

#[test]
fn exact_finite_result_cannot_be_laundered_into_source_correspondence() {
    let mut value = example_value();
    value["claims"]
        .as_array_mut()
        .expect("claims must be an array")
        .push(serde_json::json!({
            "id": "claim-source-exact",
            "subject": "implementation",
            "scope": "implementation-scope",
            "kind": "source_correspondence",
            "statement": "The finite model is exactly the source implementation",
            "epistemic_class": "exact_checked_model",
            "verdict": "supported",
            "subjects": ["impl-file"],
            "checker": {
                "tool": "kerf",
                "operation": "validate-lts",
                "input_sha256": "0000000000000000000000000000000000000000000000000000000000000000",
                "result_sha256": "1111111111111111111111111111111111111111111111111111111111111111",
                "status": "accepted"
            }
        }));
    let model = parse_value(&value).expect("model must parse");
    let analysis = analyze(&model).expect("model must be structurally valid");
    assert!(analysis.findings.iter().any(|finding| {
        finding.code == "source_claim_laundering"
            && finding.severity == semantic_architecture::semantic::Severity::Blocker
    }));
}

#[test]
fn claim_cannot_borrow_reality_contact_from_an_unrelated_subject() {
    let mut value = example_value();
    value["subjects"]
        .as_array_mut()
        .expect("subjects must be an array")
        .push(serde_json::json!({"id": "unrelated", "kind": "candidate"}));
    value["bindings"]
        .as_array_mut()
        .expect("bindings must be an array")
        .push(serde_json::json!({
            "id": "binding-unrelated",
            "subject": "unrelated",
            "kind": "source_artifact",
            "artifact_sha256": "2222222222222222222222222222222222222222222222222222222222222222",
            "locator": "unrelated"
        }));
    value["evidence"]
        .as_array_mut()
        .expect("evidence must be an array")
        .push(serde_json::json!({
            "id": "ev-unrelated",
            "subject": "unrelated",
            "binding": "binding-unrelated",
            "kind": "runtime",
            "acquisition": "observed",
            "sha256": "3333333333333333333333333333333333333333333333333333333333333333",
            "source": "unrelated receipt",
            "coverage": {}
        }));
    value["claims"][0]["support"] = serde_json::json!(["ev-unrelated"]);
    let model = parse_value(&value).expect("model shape must parse");
    let error = analyze(&model).expect_err("unrelated evidence must fail structurally");
    assert!(error.message.contains("unrelated subject"));
}

#[test]
fn exact_claim_checker_operation_must_match_its_projection() {
    let base = parse_value(&example_value()).expect("base model must parse");
    let target_sha256 = project(&base, "toggle-lts")
        .expect("projection must validate")
        .target_input_sha256;
    let mut value = example_value();
    value["evidence"][1]["kind"] = Value::String("generated".to_owned());
    let result_sha256 = value["evidence"][1]["sha256"]
        .as_str()
        .expect("evidence hash must be a string")
        .to_owned();
    value["claims"]
        .as_array_mut()
        .expect("claims must be an array")
        .push(serde_json::json!({
            "id": "claim-operation-mismatch",
            "subject": "implementation",
            "scope": "implementation-scope",
            "kind": "equivalent",
            "statement": "The projected finite implementation model is valid",
            "epistemic_class": "exact_checked_model",
            "verdict": "supported",
            "subjects": ["impl-file"],
            "support": ["ev-lts"],
            "checker": {
                "tool": "kerf",
                "operation": "factor",
                "input_sha256": target_sha256,
                "result_sha256": result_sha256,
                "status": "accepted"
            }
        }));
    value["projections"][1]["claim_ids"] = serde_json::json!(["claim-operation-mismatch"]);
    let model = parse_value(&value).expect("model must parse");
    let analysis = analyze(&model).expect("model must be structurally valid");
    assert!(analysis.findings.iter().any(|finding| {
        finding.code == "exact_claim_checker_identity_mismatch"
            && finding.severity == semantic_architecture::semantic::Severity::Blocker
    }));
}

#[test]
fn missing_requirement_semantics_compile_into_ordered_obligations() {
    let mut value = example_value();
    value["relations"] = Value::Array(
        value["relations"]
            .as_array()
            .expect("relations must be an array")
            .iter()
            .filter(|relation| {
                !matches!(
                    relation["id"].as_str(),
                    Some("req-observed-by" | "req-realized-by")
                )
            })
            .cloned()
            .collect(),
    );
    value["entities"][0]["acceptance_claims"] = Value::Array(Vec::new());
    value["work_items"] = Value::Array(Vec::new());
    value["projections"] = Value::Array(Vec::new());
    let model = parse_value(&value).expect("model must parse");
    let work = derive_work(&model).expect("obligations must compile");
    assert!(work.acyclic);
    assert!(!work.ready_for_implementation);
    assert_eq!(work.items.len(), 4);
    assert_eq!(work.edges.len(), 4);
    assert!(work.items.iter().all(|item| {
        item.origin == semantic_architecture::semantic::WorkOrigin::GeneratedObligation
    }));
}

#[test]
fn unauthorized_writer_is_a_blocking_exact_structural_finding() {
    let mut value = example_value();
    value["entities"][8]["writers"] = serde_json::json!(["impl-toggle"]);
    let model = parse_value(&value).expect("model must parse");
    let analysis = analyze(&model).expect("model must be structurally valid");
    assert!(analysis.findings.iter().any(|finding| {
        finding.code == "unauthorized_writer"
            && finding.severity == semantic_architecture::semantic::Severity::Blocker
    }));
}

#[test]
fn grounding_keeps_formality_and_reality_contact_as_separate_axes() {
    let mut value = example_value();
    value["scopes"][0]["required_reality_contact"] = Value::String("observed".to_owned());
    value["claims"][0]["verdict"] = Value::String("supported".to_owned());
    value["claims"][0]["epistemic_class"] = Value::String("evidence_inference".to_owned());
    value["claims"][0]["support"] = serde_json::json!(["ev-lts"]);
    let model = parse_value(&value).expect("model must parse");
    let analysis = analyze(&model).expect("model must be structurally valid");
    assert!(analysis.findings.is_empty());
    assert_eq!(analysis.claim_groundings.len(), 1);
    assert_eq!(
        analysis.claim_groundings[0].formal_class,
        semantic_architecture::semantic::EpistemicClass::EvidenceInference
    );
    assert_eq!(
        analysis.claim_groundings[0].reality_contact,
        semantic_architecture::semantic::RealityContact::Observed
    );
}

#[test]
fn runtime_binding_requires_alphabet_session_and_negative_control() {
    let mut value = example_value();
    value["bindings"][0]["kind"] = Value::String("runtime_protocol".to_owned());
    let model = parse_value(&value).expect("model must parse");
    let analysis = analyze(&model).expect("model must be structurally valid");
    let codes: std::collections::BTreeSet<_> = analysis
        .findings
        .iter()
        .map(|finding| finding.code)
        .collect();
    assert!(codes.contains("runtime_binding_alphabet_missing"));
    assert!(codes.contains("runtime_binding_session_missing"));
    assert!(codes.contains("runtime_binding_negative_control_missing"));
}

#[test]
fn declaration_order_does_not_change_canonical_model_identity() {
    let original = parse_value(&example_value()).expect("example must parse");
    let original_hash = analyze(&original)
        .expect("example must analyze")
        .model_sha256;
    let mut reordered = example_value();
    for field in [
        "subjects",
        "scopes",
        "entities",
        "relations",
        "bindings",
        "evidence",
        "closures",
        "claims",
        "work_items",
        "projections",
    ] {
        reordered[field]
            .as_array_mut()
            .expect("declaration field must be an array")
            .reverse();
    }
    let reordered = parse_value(&reordered).expect("reordered model must parse");
    assert_eq!(
        analyze(&reordered)
            .expect("reordered model must analyze")
            .model_sha256,
        original_hash
    );
}

#[test]
fn declared_work_cycle_prevents_implementation_readiness() {
    let mut value = example_value();
    value["work_items"][0]["depends_on"] = serde_json::json!(["work-review"]);
    value["work_items"]
        .as_array_mut()
        .expect("work items must be an array")
        .push(serde_json::json!({
            "id": "work-review",
            "intent": "Review the toggle implementation",
            "depends_on": ["work-toggle"]
        }));
    let model = parse_value(&value).expect("model must parse");
    let work = derive_work(&model).expect("cycle is a completed analysis");
    assert!(!work.acyclic);
    assert!(!work.ready_for_implementation);
    assert!(work.items.iter().any(|item| {
        item.resolves_findings
            .iter()
            .any(|finding| finding.contains("work_dependency_cycle"))
    }));
}

#[test]
fn subject_and_evidence_cycles_are_invalid_not_provenance() {
    let mut subject_cycle = example_value();
    subject_cycle["subjects"][0]["parent"] = Value::String("implementation".to_owned());
    subject_cycle["subjects"][1]["parent"] = Value::String("intent".to_owned());
    let subject_model = parse_value(&subject_cycle).expect("shape must parse");
    assert!(
        analyze(&subject_model)
            .expect_err("subject ancestry cycle must fail")
            .message
            .contains("subject.parent contains a cycle")
    );

    let mut evidence_cycle = example_value();
    evidence_cycle["evidence"][1]["derived_from"] = serde_json::json!(["ev-lts"]);
    let evidence_model = parse_value(&evidence_cycle).expect("shape must parse");
    assert!(
        analyze(&evidence_model)
            .expect_err("evidence derivation cycle must fail")
            .message
            .contains("evidence.derived_from contains a cycle")
    );
}

#[test]
fn authority_writer_cannot_silently_cross_subjects() {
    let mut value = example_value();
    value["entities"][2]["writers"] = serde_json::json!(["impl-file"]);
    let model = parse_value(&value).expect("shape must parse");
    let error = analyze(&model).expect_err("cross-subject authority must fail");
    assert!(error.message.contains("authority.writers"));
    assert!(error.message.contains("crosses"));
}

#[test]
fn cli_emits_versioned_json_without_decorative_stderr() {
    let output = Command::new(env!("CARGO_BIN_EXE_kerf-model"))
        .arg("analyze")
        .arg(repo_path("examples/semantic-toggle.model.json"))
        .output()
        .expect("kerf-model must run");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    let envelope: Value = serde_json::from_slice(&output.stdout).expect("stdout must be JSON");
    assert_eq!(envelope["schema_version"], "kerf.model-result.v1");
    assert_eq!(envelope["operation"], "analyze");
    assert_eq!(envelope["result"]["finding_counts"]["blocker"], 0);
}

#[test]
fn cli_errors_are_typed_and_input_bound() {
    let output = Command::new(env!("CARGO_BIN_EXE_kerf-model"))
        .arg("project")
        .arg(repo_path("examples/semantic-toggle.model.json"))
        .arg("absent")
        .output()
        .expect("kerf-model must run");
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    let envelope: Value = serde_json::from_slice(&output.stderr).expect("stderr must be JSON");
    assert_eq!(envelope["schema_version"], "kerf.model-error.v1");
    assert_eq!(envelope["operation"], "project");
    assert_eq!(envelope["error"]["code"], "invalid_input");
    assert_eq!(envelope["input_sha256"].as_str().map(str::len), Some(64));
}
