use kerf::candidates::Direction;
use kerf::report::sha256_hex;
use semantic_architecture::converge::EvaluatorIdentity;
use semantic_architecture::cycle::{
    CAMPAIGN_STATE_SCHEMA, CampaignCandidate, CampaignMetricDefinition, CampaignState,
    CampaignSubject, CampaignSweep, CandidateVerification, GeneratorObligation, GeneratorOutcome,
    GeneratorReceipt, PurposeEnvelope, ResourceBudget, campaign_identities,
};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_ID: AtomicU64 = AtomicU64::new(0);

fn digest(value: &str) -> String {
    sha256_hex(value.as_bytes())
}

fn run(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_kerf-cycle"))
        .args(args)
        .output()
        .expect("run kerf-cycle")
}

fn fixture_path() -> PathBuf {
    let serial = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "semantic-architecture-cycle-cli-{}-{serial}.json",
        std::process::id()
    ))
}

fn terminal_state(evaluator: EvaluatorIdentity) -> (CampaignState, String, String) {
    let mut state = CampaignState {
        schema_version: CAMPAIGN_STATE_SCHEMA.to_owned(),
        campaign_id: "cli-greenfield".to_owned(),
        evaluator,
        subject: CampaignSubject::Greenfield {
            subject_id: "new-system".to_owned(),
            intent_model_sha256: digest("intent"),
        },
        envelope: PurposeEnvelope {
            envelope_id: "purpose".to_owned(),
            purpose_sha256: digest("purpose"),
            environment_sha256: digest("environment"),
            hard_constraints_sha256: digest("constraints"),
            metric_policy_sha256: digest("metric-policy"),
            transformation_policy_sha256: digest("transformation-policy"),
        },
        semantic_model_sha256: digest("model"),
        generators: vec![GeneratorObligation {
            generator_id: "semantic-generator".to_owned(),
            generator_version_sha256: digest("generator"),
            scope_id: "whole-system".to_owned(),
            scope_sha256: digest("scope"),
        }],
        metric_definitions: vec![CampaignMetricDefinition {
            id: "description_bits".to_owned(),
            unit: "bits".to_owned(),
            direction: Direction::Minimize,
        }],
        candidates: Vec::new(),
        current_candidate_id: Some("current".to_owned()),
        sweeps: Vec::new(),
        finite_universe: None,
        budget: ResourceBudget {
            unit: "sweeps".to_owned(),
            limit: 10,
            consumed: 2,
        },
        blockers: Vec::new(),
        unknowns: Vec::new(),
        parent_state_sha256: None,
        nonclaims: vec!["CLI fixture does not attest real generation".to_owned()],
    };
    let (search_basis, _) = campaign_identities(&state).expect("search basis");
    state.candidates.push(CampaignCandidate {
        id: "current".to_owned(),
        search_basis_sha256: search_basis.clone(),
        origin_sweep_id: None,
        verification: CandidateVerification::Verified {
            evidence_sha256: digest("candidate-evidence"),
        },
        metrics: BTreeMap::from([("description_bits".to_owned(), "42".to_owned())]),
    });
    let (_, decision_state) = campaign_identities(&state).expect("decision state");
    let generator = &state.generators[0];
    for (ordinal, id) in [(1, "quiet-1"), (2, "quiet-2")] {
        state.sweeps.push(CampaignSweep {
            id: id.to_owned(),
            ordinal,
            search_basis_sha256: search_basis.clone(),
            decision_state_sha256: decision_state.clone(),
            receipts: vec![GeneratorReceipt {
                generator_id: generator.generator_id.clone(),
                generator_version_sha256: generator.generator_version_sha256.clone(),
                scope_id: generator.scope_id.clone(),
                scope_sha256: generator.scope_sha256.clone(),
                evidence_sha256: digest(&format!("{id}-evidence")),
                outcome: GeneratorOutcome::Complete {
                    novel_candidate_ids: Vec::new(),
                    examined_finite_members: Vec::new(),
                },
            }],
        });
    }
    (state, search_basis, decision_state)
}

#[test]
fn cli_emits_hashes_and_requires_a_computed_terminal_state() {
    let identity_output = run(&["identity"]);
    assert!(identity_output.status.success());
    assert!(identity_output.stderr.is_empty());
    let identity_document: serde_json::Value =
        serde_json::from_slice(&identity_output.stdout).expect("identity JSON");
    let evaluator: EvaluatorIdentity =
        serde_json::from_value(identity_document["result"].clone()).expect("evaluator");

    let (state, search_basis, decision_state) = terminal_state(evaluator);

    let path = fixture_path();
    fs::write(
        &path,
        serde_json::to_vec_pretty(&state).expect("campaign JSON"),
    )
    .expect("write campaign");
    let path_text = path.to_str().expect("UTF-8 path");

    let hashes = run(&["hashes", path_text]);
    assert!(hashes.status.success());
    assert!(hashes.stderr.is_empty());
    let hashes_document: serde_json::Value =
        serde_json::from_slice(&hashes.stdout).expect("hashes JSON");
    assert_eq!(
        hashes_document["result"]["search_basis_sha256"],
        search_basis
    );
    assert_eq!(
        hashes_document["result"]["decision_state_sha256"],
        decision_state
    );

    let evaluation = run(&["evaluate", path_text, "--require-terminal"]);
    assert!(evaluation.status.success());
    assert!(evaluation.stderr.is_empty());
    let evaluation_document: serde_json::Value =
        serde_json::from_slice(&evaluation.stdout).expect("evaluation JSON");
    assert_eq!(
        evaluation_document["result"]["status"],
        "envelope_stationary"
    );

    fs::remove_file(path).expect("remove fixture");
}
