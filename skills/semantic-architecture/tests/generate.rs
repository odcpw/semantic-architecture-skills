use kerf::candidates::Direction;
use kerf::report::sha256_hex;
use semantic_architecture::cycle::PurposeEnvelope;
use semantic_architecture::generate::{
    CapabilityIntent, EnvironmentModel, GeneratorSubject, HardConstraint, MetricIntent, MetricRole,
    ProtectedObservation, PurposeModel, SemanticFactIntent, TRANSFORMATION_REQUEST_SCHEMA,
    TransformationCandidate, TransformationFamily, TransformationPolicy, TransformationRequest,
    environment_sha256, generate_transformations, hard_constraints_sha256, metric_policy_sha256,
    purpose_sha256, transformation_policy_sha256, validate_transformation_batch,
};
use semantic_architecture::semantic::parse_no_duplicates;
use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_ID: AtomicU64 = AtomicU64::new(0);

fn digest(value: &str) -> String {
    sha256_hex(value.as_bytes())
}

struct RepositoryFixture {
    root: PathBuf,
}

impl RepositoryFixture {
    fn new() -> Self {
        let serial = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "semantic-architecture-generator-{}-{serial}",
            std::process::id()
        ));
        fs::create_dir_all(root.join("src")).expect("fixture source directory");
        let first = r"
fn ingest_catalog() {
    let shared = load_catalog();
    validate_catalog(&shared);
    persist_catalog(shared);
}
";
        let second = r"
fn export_catalog() {
    let shared = load_catalog();
    validate_catalog(&shared);
    persist_catalog(shared);
}
";
        fs::write(root.join("src/ingest.rs"), first).expect("first source");
        fs::write(root.join("src/export.rs"), second).expect("second source");
        Self { root }
    }
}

impl Drop for RepositoryFixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn metric_intents() -> Vec<MetricIntent> {
    vec![
        MetricIntent {
            id: "capability_coverage".to_owned(),
            unit: "capabilities".to_owned(),
            direction: Direction::Maximize,
            role: MetricRole::CapabilityCoverage,
        },
        MetricIntent {
            id: "description_bits".to_owned(),
            unit: "bits".to_owned(),
            direction: Direction::Minimize,
            role: MetricRole::DescriptionCost,
        },
        MetricIntent {
            id: "duplicate_authorities".to_owned(),
            unit: "writers".to_owned(),
            direction: Direction::Minimize,
            role: MetricRole::DuplicateAuthority,
        },
        MetricIntent {
            id: "boundary_information".to_owned(),
            unit: "bits-per-step".to_owned(),
            direction: Direction::Minimize,
            role: MetricRole::BoundaryInformation,
        },
    ]
}

fn transformation_policy() -> TransformationPolicy {
    TransformationPolicy {
        allowed_families: vec![
            TransformationFamily::CapabilityConstruction,
            TransformationFamily::BehavioralFactorRealization,
            TransformationFamily::AuthorityConsolidation,
            TransformationFamily::IsomorphicSimplification,
            TransformationFamily::SplitFuseRelocate,
        ],
        source_extensions: vec!["rs".to_owned()],
        planned_source_root: "src".to_owned(),
        max_files: 100,
        max_bytes_per_file: 100_000,
        max_depth: 8,
        minimum_duplicate_lines: 3,
        max_candidates: 100,
    }
}

fn request(subject: GeneratorSubject) -> TransformationRequest {
    let purpose = PurposeModel {
        purpose_id: "catalog-service".to_owned(),
        capabilities: vec![
            CapabilityIntent {
                id: "ingest_catalog".to_owned(),
                responsibility: "ingest catalog records".to_owned(),
                reads: vec!["catalog".to_owned()],
                writes: vec!["catalog".to_owned()],
            },
            CapabilityIntent {
                id: "export_catalog".to_owned(),
                responsibility: "export catalog records".to_owned(),
                reads: vec!["catalog".to_owned()],
                writes: vec!["catalog".to_owned()],
            },
        ],
        facts: vec![SemanticFactIntent {
            id: "catalog".to_owned(),
            description: "authoritative catalog contents".to_owned(),
            writers: vec!["ingest_catalog".to_owned(), "export_catalog".to_owned()],
            readers: vec!["ingest_catalog".to_owned(), "export_catalog".to_owned()],
        }],
        protected_observations: vec![ProtectedObservation {
            id: "catalog-roundtrip".to_owned(),
            statement: "ingested catalog records are exported without semantic change".to_owned(),
            capability_ids: vec!["ingest_catalog".to_owned(), "export_catalog".to_owned()],
        }],
    };
    let environment = EnvironmentModel {
        environment_id: "linux-test".to_owned(),
        contexts: vec!["command-line".to_owned()],
        assumptions: vec!["local filesystem available".to_owned()],
    };
    let hard_constraints = vec![HardConstraint {
        id: "roundtrip-preserved".to_owned(),
        statement: "catalog roundtrip bytes remain compatible".to_owned(),
        affected_capability_ids: vec!["ingest_catalog".to_owned(), "export_catalog".to_owned()],
    }];
    let metrics = metric_intents();
    let policy = transformation_policy();
    let envelope = PurposeEnvelope {
        envelope_id: "catalog-envelope".to_owned(),
        purpose_sha256: purpose_sha256(&purpose).expect("purpose hash"),
        environment_sha256: environment_sha256(&environment).expect("environment hash"),
        hard_constraints_sha256: hard_constraints_sha256(&hard_constraints)
            .expect("constraint hash"),
        metric_policy_sha256: metric_policy_sha256(&metrics).expect("metric hash"),
        transformation_policy_sha256: transformation_policy_sha256(&policy).expect("policy hash"),
    };
    TransformationRequest {
        schema_version: TRANSFORMATION_REQUEST_SCHEMA.to_owned(),
        request_id: "catalog-generation".to_owned(),
        search_basis_sha256: digest("search-basis"),
        subject,
        envelope,
        purpose,
        environment,
        hard_constraints,
        metrics,
        policy,
    }
}

#[test]
fn greenfield_purpose_produces_three_semantic_families_without_a_seam() {
    let request = request(GeneratorSubject::Greenfield);
    let batch = generate_transformations(&request).expect("greenfield candidates");
    let families = batch
        .candidates
        .iter()
        .map(|candidate| candidate.family)
        .collect::<BTreeSet<_>>();
    assert!(families.contains(&TransformationFamily::CapabilityConstruction));
    assert!(families.contains(&TransformationFamily::BehavioralFactorRealization));
    assert!(families.contains(&TransformationFamily::AuthorityConsolidation));
    assert!(families.len() >= 3);
    assert!(batch.coverage.complete);
    assert!(batch.candidates.iter().all(|candidate| {
        !candidate.witnesses.is_empty()
            && !candidate.proof_obligations.is_empty()
            && candidate.predicted_metric_deltas.len() == request.metrics.len()
    }));
    validate_transformation_batch(&request, &batch).expect("valid batch");
}

#[test]
fn brownfield_discovery_finds_structure_not_file_size() {
    let repository = RepositoryFixture::new();
    let request = request(GeneratorSubject::Brownfield {
        repository_root: repository.root.to_string_lossy().into_owned(),
        revision: "fixture".to_owned(),
        source_tree_sha256: digest("fixture-tree"),
    });
    let batch = generate_transformations(&request).expect("brownfield candidates");
    let families = batch
        .candidates
        .iter()
        .map(|candidate| candidate.family)
        .collect::<BTreeSet<_>>();
    assert!(families.contains(&TransformationFamily::IsomorphicSimplification));
    assert!(families.contains(&TransformationFamily::SplitFuseRelocate));
    assert!(families.len() >= 3);
    assert_eq!(batch.coverage.scanned_files, 2);
    assert!(batch.coverage.complete);
}

#[test]
fn generation_is_deterministic() {
    let request = request(GeneratorSubject::Greenfield);
    let first = generate_transformations(&request).expect("first batch");
    let second = generate_transformations(&request).expect("second batch");
    assert_eq!(
        serde_json::to_value(first).expect("first JSON"),
        serde_json::to_value(second).expect("second JSON")
    );
}

#[test]
fn envelope_content_drift_is_rejected() {
    let mut request = request(GeneratorSubject::Greenfield);
    request.purpose.capabilities[0].responsibility = "silently changed".to_owned();
    let error = generate_transformations(&request).expect_err("drift must fail");
    assert!(error.message.contains("envelope hashes"));
}

#[test]
fn candidate_without_proof_obligations_is_rejected() {
    let request = request(GeneratorSubject::Greenfield);
    let mut batch = generate_transformations(&request).expect("batch");
    batch.candidates[0].proof_obligations.clear();
    let error = validate_transformation_batch(&request, &batch).expect_err("missing proof");
    assert!(error.message.contains("proof obligation"));
}

#[test]
fn edited_candidate_content_invalidates_its_identity() {
    let request = request(GeneratorSubject::Greenfield);
    let mut batch = generate_transformations(&request).expect("batch");
    batch.candidates[0].summary.push_str(" tampered");
    let error = validate_transformation_batch(&request, &batch).expect_err("tampered candidate");
    assert!(error.message.contains("content-derived ID"));
}

#[test]
fn file_size_is_not_an_admissible_semantic_witness() {
    let request = request(GeneratorSubject::Greenfield);
    let batch = generate_transformations(&request).expect("batch");
    let mut candidate = serde_json::to_value(&batch.candidates[0]).expect("candidate JSON");
    candidate["witnesses"] = serde_json::json!([{"kind": "file_size", "bytes": 10000}]);
    let bytes = serde_json::to_vec(&candidate).expect("candidate bytes");
    let error = parse_no_duplicates::<TransformationCandidate>(&bytes).expect_err("file size");
    assert!(error.message.contains("file_size") || error.message.contains("unknown variant"));
}

#[test]
fn candidate_bound_marks_discovery_incomplete_instead_of_claiming_exhaustion() {
    let mut request = request(GeneratorSubject::Greenfield);
    request.policy.max_candidates = 1;
    request.envelope.transformation_policy_sha256 =
        transformation_policy_sha256(&request.policy).expect("policy hash");
    let batch = generate_transformations(&request).expect("bounded batch");
    assert!(batch.coverage.truncated);
    assert!(!batch.coverage.complete);
    assert_eq!(batch.candidates.len(), 1);
}

#[test]
fn generator_cli_emits_a_valid_machine_batch() {
    let request = request(GeneratorSubject::Greenfield);
    let serial = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "semantic-architecture-generator-cli-{}-{serial}.json",
        std::process::id()
    ));
    fs::write(
        &path,
        serde_json::to_vec_pretty(&request).expect("request JSON"),
    )
    .expect("request fixture");
    let output = Command::new(env!("CARGO_BIN_EXE_kerf-generate"))
        .args([
            "generate",
            path.to_str().expect("UTF-8 path"),
            "--require-complete",
        ])
        .output()
        .expect("run kerf-generate");
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let document: serde_json::Value = serde_json::from_slice(&output.stdout).expect("result JSON");
    assert_eq!(document["status"], "ok");
    assert!(
        document["result"]["candidates"]
            .as_array()
            .is_some_and(|items| items.len() >= 3)
    );
    fs::remove_file(path).expect("remove request fixture");
}
