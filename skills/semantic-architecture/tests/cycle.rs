use kerf::candidates::Direction;
use kerf::report::sha256_hex;
use semantic_architecture::converge::current_evaluator_identity;
use semantic_architecture::cycle::{
    CAMPAIGN_STATE_SCHEMA, CampaignCandidate, CampaignMetricDefinition, CampaignState,
    CampaignStatus, CampaignSubject, CampaignSweep, CandidateVerification, FiniteSearchUniverse,
    GeneratorObligation, GeneratorOutcome, GeneratorReceipt, PurposeEnvelope, ResourceBudget,
    campaign_identities, evaluate_campaign,
};
use semantic_architecture::semantic::parse_no_duplicates;
use std::collections::BTreeMap;

fn digest(value: &str) -> String {
    sha256_hex(value.as_bytes())
}

#[derive(Clone, Copy)]
enum FixtureMode {
    Greenfield,
    Brownfield,
}

impl FixtureMode {
    const fn label(self) -> &'static str {
        match self {
            Self::Greenfield => "greenfield",
            Self::Brownfield => "brownfield",
        }
    }
}

fn subject(mode: FixtureMode) -> CampaignSubject {
    match mode {
        FixtureMode::Greenfield => CampaignSubject::Greenfield {
            subject_id: "new-system".to_owned(),
            intent_model_sha256: digest("intent"),
        },
        FixtureMode::Brownfield => CampaignSubject::Brownfield {
            subject_id: "existing-system".to_owned(),
            repository: "/tmp/existing-system".to_owned(),
            revision: "abc123".to_owned(),
            source_tree_sha256: digest("tree"),
        },
    }
}

fn base_state(mode: FixtureMode, finite_members: &[&str]) -> CampaignState {
    let mut state = CampaignState {
        schema_version: CAMPAIGN_STATE_SCHEMA.to_owned(),
        campaign_id: format!("{}-campaign", mode.label()),
        evaluator: current_evaluator_identity().expect("evaluator identity"),
        subject: subject(mode),
        envelope: PurposeEnvelope {
            envelope_id: "road-car".to_owned(),
            purpose_sha256: digest("purpose"),
            environment_sha256: digest("environment"),
            hard_constraints_sha256: digest("constraints"),
            metric_policy_sha256: digest("metrics"),
            transformation_policy_sha256: digest("transforms"),
        },
        semantic_model_sha256: digest("semantic-model"),
        generators: vec![GeneratorObligation {
            generator_id: "isomorphic-refactor".to_owned(),
            generator_version_sha256: digest("generator-v1"),
            scope_id: "whole-system".to_owned(),
            scope_sha256: digest("whole-system-scope"),
        }],
        metric_definitions: vec![
            CampaignMetricDefinition {
                id: "latency".to_owned(),
                unit: "nanoseconds".to_owned(),
                direction: Direction::Minimize,
            },
            CampaignMetricDefinition {
                id: "correctness".to_owned(),
                unit: "covered-observations".to_owned(),
                direction: Direction::Maximize,
            },
        ],
        candidates: Vec::new(),
        current_candidate_id: Some("current".to_owned()),
        sweeps: Vec::new(),
        finite_universe: (!finite_members.is_empty()).then(|| FiniteSearchUniverse {
            id: "declared-transformations".to_owned(),
            declared_members: finite_members.iter().map(ToString::to_string).collect(),
        }),
        budget: ResourceBudget {
            unit: "generator-invocations".to_owned(),
            limit: 100,
            consumed: 0,
        },
        blockers: Vec::new(),
        unknowns: Vec::new(),
        parent_state_sha256: None,
        nonclaims: vec!["fixture evidence does not attest physical execution".to_owned()],
    };
    let (basis, _) = campaign_identities(&state).expect("campaign identities");
    state.candidates.push(CampaignCandidate {
        id: "current".to_owned(),
        search_basis_sha256: basis,
        origin_sweep_id: None,
        verification: CandidateVerification::Verified {
            evidence_sha256: digest("current-evidence"),
        },
        metrics: BTreeMap::from([
            ("correctness".to_owned(), "10".to_owned()),
            ("latency".to_owned(), "10".to_owned()),
        ]),
    });
    state
}

fn add_sweep(
    state: &mut CampaignState,
    id: &str,
    ordinal: u64,
    novel_candidates: &[&str],
    finite_members: &[&str],
) {
    let (search_basis_sha256, decision_state_sha256) =
        campaign_identities(state).expect("campaign identities");
    let generator = &state.generators[0];
    state.sweeps.push(CampaignSweep {
        id: id.to_owned(),
        ordinal,
        search_basis_sha256,
        decision_state_sha256,
        receipts: vec![GeneratorReceipt {
            generator_id: generator.generator_id.clone(),
            generator_version_sha256: generator.generator_version_sha256.clone(),
            scope_id: generator.scope_id.clone(),
            scope_sha256: generator.scope_sha256.clone(),
            evidence_sha256: digest(&format!("{id}-evidence")),
            outcome: GeneratorOutcome::Complete {
                novel_candidate_ids: novel_candidates.iter().map(ToString::to_string).collect(),
                examined_finite_members: finite_members.iter().map(ToString::to_string).collect(),
            },
        }],
    });
}

fn add_candidate(
    state: &mut CampaignState,
    id: &str,
    origin_sweep_id: &str,
    latency: &str,
    correctness: &str,
) {
    let (basis, _) = campaign_identities(state).expect("campaign identities");
    state.candidates.push(CampaignCandidate {
        id: id.to_owned(),
        search_basis_sha256: basis,
        origin_sweep_id: Some(origin_sweep_id.to_owned()),
        verification: CandidateVerification::Verified {
            evidence_sha256: digest(&format!("{id}-evidence")),
        },
        metrics: BTreeMap::from([
            ("correctness".to_owned(), correctness.to_owned()),
            ("latency".to_owned(), latency.to_owned()),
        ]),
    });
}

#[test]
fn one_engine_evaluates_greenfield_and_brownfield_states() {
    for mode in [FixtureMode::Greenfield, FixtureMode::Brownfield] {
        let mut state = base_state(mode, &[]);
        add_sweep(&mut state, "quiet-1", 1, &[], &[]);
        add_sweep(&mut state, "quiet-2", 2, &[], &[]);
        let result = evaluate_campaign(&state).expect("campaign evaluation");
        assert_eq!(result.mode, mode.label());
        assert_eq!(result.status, CampaignStatus::EnvelopeStationary);
        assert_eq!(result.generator_coverage.unchanged_quiescent_sweeps, 2);
    }
}

#[test]
fn exhaustion_boolean_is_not_part_of_the_schema() {
    let state = base_state(FixtureMode::Greenfield, &[]);
    let mut document = serde_json::to_value(state).expect("state JSON");
    document.as_object_mut().expect("object").insert(
        "generators_exhausted".to_owned(),
        serde_json::Value::Bool(true),
    );
    let bytes = serde_json::to_vec(&document).expect("bytes");
    let error = parse_no_duplicates::<CampaignState>(&bytes).expect_err("unknown assertion");
    assert!(error.message.contains("generators_exhausted"));
}

#[test]
fn one_quiescent_sweep_is_not_stationarity() {
    let mut state = base_state(FixtureMode::Brownfield, &[]);
    add_sweep(&mut state, "quiet-1", 1, &[], &[]);
    let result = evaluate_campaign(&state).expect("campaign evaluation");
    assert_eq!(result.status, CampaignStatus::Open);
    assert!(result.reasons.iter().any(|reason| reason.contains("1/2")));
}

#[test]
fn budget_exhaustion_is_computed_and_preserves_unknowns() {
    let mut state = base_state(FixtureMode::Brownfield, &[]);
    state.budget.consumed = state.budget.limit;
    let result = evaluate_campaign(&state).expect("campaign evaluation");
    assert!(result.budget_exhausted);
    assert_eq!(result.status, CampaignStatus::BudgetExhaustedWithUnknowns);
}

#[test]
fn finite_declared_search_can_close_after_exact_enumeration() {
    let mut state = base_state(FixtureMode::Greenfield, &["a", "b"]);
    add_sweep(&mut state, "exhaustive", 1, &[], &["b", "a"]);
    let result = evaluate_campaign(&state).expect("campaign evaluation");
    assert_eq!(result.status, CampaignStatus::BoundedExhaustiveFixedPoint);
    let coverage = result.finite_coverage.expect("finite coverage");
    assert!(coverage.missing.is_empty());
    assert!(coverage.undeclared.is_empty());
}

#[test]
fn incomparable_verified_alternatives_form_a_frontier() {
    let mut state = base_state(FixtureMode::Brownfield, &[]);
    let pre_candidate_decision = campaign_identities(&state).expect("campaign identities").1;
    add_candidate(&mut state, "leaner", "discover", "8", "9");
    let search_basis = campaign_identities(&state).expect("campaign identities").0;
    let generator = &state.generators[0];
    state.sweeps.push(CampaignSweep {
        id: "discover".to_owned(),
        ordinal: 1,
        search_basis_sha256: search_basis,
        decision_state_sha256: pre_candidate_decision,
        receipts: vec![GeneratorReceipt {
            generator_id: generator.generator_id.clone(),
            generator_version_sha256: generator.generator_version_sha256.clone(),
            scope_id: generator.scope_id.clone(),
            scope_sha256: generator.scope_sha256.clone(),
            evidence_sha256: digest("discover-evidence"),
            outcome: GeneratorOutcome::Complete {
                novel_candidate_ids: vec!["leaner".to_owned()],
                examined_finite_members: Vec::new(),
            },
        }],
    });
    add_sweep(&mut state, "quiet-1", 2, &[], &[]);
    add_sweep(&mut state, "quiet-2", 3, &[], &[]);
    let result = evaluate_campaign(&state).expect("campaign evaluation");
    assert_eq!(result.status, CampaignStatus::FrontierSet);
    assert_eq!(result.frontier, vec!["current", "leaner"]);
}

#[test]
fn a_dominated_current_implementation_keeps_the_campaign_open() {
    let mut state = base_state(FixtureMode::Brownfield, &[]);
    let pre_candidate_decision = campaign_identities(&state).expect("campaign identities").1;
    add_candidate(&mut state, "better", "discover", "8", "10");
    let search_basis = campaign_identities(&state).expect("campaign identities").0;
    let generator = &state.generators[0];
    state.sweeps.push(CampaignSweep {
        id: "discover".to_owned(),
        ordinal: 1,
        search_basis_sha256: search_basis,
        decision_state_sha256: pre_candidate_decision,
        receipts: vec![GeneratorReceipt {
            generator_id: generator.generator_id.clone(),
            generator_version_sha256: generator.generator_version_sha256.clone(),
            scope_id: generator.scope_id.clone(),
            scope_sha256: generator.scope_sha256.clone(),
            evidence_sha256: digest("discover-evidence"),
            outcome: GeneratorOutcome::Complete {
                novel_candidate_ids: vec!["better".to_owned()],
                examined_finite_members: Vec::new(),
            },
        }],
    });
    add_sweep(&mut state, "quiet-1", 2, &[], &[]);
    add_sweep(&mut state, "quiet-2", 3, &[], &[]);
    let result = evaluate_campaign(&state).expect("campaign evaluation");
    assert_eq!(result.status, CampaignStatus::Open);
    assert_eq!(result.frontier, vec!["better"]);
    assert_eq!(result.dominated["current"], vec!["better"]);
}

#[test]
fn changing_the_model_invalidates_old_candidates_and_sweeps() {
    let mut state = base_state(FixtureMode::Brownfield, &[]);
    add_sweep(&mut state, "quiet-1", 1, &[], &[]);
    add_sweep(&mut state, "quiet-2", 2, &[], &[]);
    assert_eq!(
        evaluate_campaign(&state).expect("stationary").status,
        CampaignStatus::EnvelopeStationary
    );

    state.semantic_model_sha256 = digest("rebuilt-semantic-model");
    let result = evaluate_campaign(&state).expect("reopened campaign");
    assert_eq!(result.status, CampaignStatus::Open);
    assert!(result.frontier.is_empty());
    assert_eq!(result.generator_coverage.complete_current_sweeps, 0);
}
