use kerf::candidates::Direction;
use kerf::report::sha256_hex;
use semantic_architecture::cycle::PurposeEnvelope;
use semantic_architecture::generate::{
    CapabilityIntent, EnvironmentModel, GeneratorSubject, HardConstraint, MetricIntent, MetricRole,
    ProtectedObservation, PurposeModel, TRANSFORMATION_REQUEST_SCHEMA, TransformationFamily,
    TransformationPolicy, TransformationRequest, environment_sha256, generate_transformations,
    hard_constraints_sha256, metric_policy_sha256, purpose_sha256, transformation_policy_sha256,
};
use semantic_architecture::metabolize::{
    BoundCommand, BoundExternalFile, BoundProgram, BoundSourceFile, CheckKind, FileMutation,
    IsolationPolicy, LiteralReplacement, METABOLISM_PLAN_SCHEMA, MetabolismPlan, MetabolismStatus,
    MetricProbe, ModelRebuild, NegativeControl, VerificationCheck, execute_metabolism,
};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_ID: AtomicU64 = AtomicU64::new(0);

struct Fixture {
    source: PathBuf,
    workspace: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let serial = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let base = std::env::temp_dir().join(format!(
            "semantic-architecture-metabolism-{}-{serial}",
            std::process::id()
        ));
        let source = base.join("source");
        let workspace = base.join("execution");
        fs::create_dir_all(&source).expect("source fixture");
        fs::write(source.join("left.py"), baseline_module()).expect("left source");
        fs::write(source.join("right.py"), baseline_module()).expect("right source");
        fs::write(source.join("harness.py"), harness()).expect("test harness");
        Self { source, workspace }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        if let Some(base) = self.source.parent() {
            let _ = fs::remove_dir_all(base);
        }
    }
}

fn baseline_module() -> &'static str {
    r"def compute(value):
    normalized = value + 11
    normalized = normalized * 3
    normalized = normalized - 7
    normalized = normalized // 2
    return normalized
"
}

fn shared_module() -> &'static str {
    r"def compute(value):
    value = value + 11
    value = value * 3
    value = value - 7
    return value // 2
"
}

fn wrapper_module() -> &'static str {
    "from shared import compute\n"
}

fn harness() -> &'static str {
    r#"import importlib.util
import json
import os
import pathlib
import sys

root = pathlib.Path(__file__).parent
mode = sys.argv[1]
if mode == "negative":
    raise SystemExit(7)
if mode == "metric":
    print(sum(len(path.read_bytes()) for path in root.glob("*.py") if path.name != "harness.py"))
    raise SystemExit(0)
if mode == "model":
    model = {
        "schema_version": "kerf.semantic-model.v1",
        "model_id": "metabolism-fixture-model",
        "subjects": [{
            "id": "implementation",
            "kind": "implementation",
            "revision": os.environ["KERF_POST_REVISION"],
        }],
        "scopes": [],
        "entities": [],
    }
    (root / "rebuilt.model.json").write_text(json.dumps(model, sort_keys=True))
    raise SystemExit(0)
sys.path.insert(0, str(root))
import left
import right
print(json.dumps([[left.compute(i), right.compute(i)] for i in [-9, 0, 4, 101]]))
"#
}

fn digest_file(path: &Path) -> String {
    sha256_hex(&fs::read(path).expect("fixture bytes"))
}

fn request(root: &Path) -> TransformationRequest {
    let purpose = PurposeModel {
        purpose_id: "normalizer".to_owned(),
        capabilities: vec![CapabilityIntent {
            id: "normalize".to_owned(),
            responsibility: "normalize integer inputs through both public entry points".to_owned(),
            reads: vec![],
            writes: vec![],
        }],
        facts: vec![],
        protected_observations: vec![ProtectedObservation {
            id: "entrypoint-parity".to_owned(),
            statement: "both public entry points retain their output mapping".to_owned(),
            capability_ids: vec!["normalize".to_owned()],
        }],
    };
    let environment = EnvironmentModel {
        environment_id: "linux-python".to_owned(),
        contexts: vec!["command line".to_owned()],
        assumptions: vec![],
    };
    let hard_constraints = vec![HardConstraint {
        id: "tests-pass".to_owned(),
        statement: "the executable observation harness succeeds".to_owned(),
        affected_capability_ids: vec!["normalize".to_owned()],
    }];
    let metrics = vec![MetricIntent {
        id: "source_bytes".to_owned(),
        unit: "bytes".to_owned(),
        direction: Direction::Minimize,
        role: MetricRole::DescriptionCost,
    }];
    let policy = TransformationPolicy {
        allowed_families: vec![TransformationFamily::IsomorphicSimplification],
        source_extensions: vec!["py".to_owned()],
        planned_source_root: "src".to_owned(),
        max_files: 10,
        max_bytes_per_file: 100_000,
        max_depth: 4,
        minimum_duplicate_lines: 4,
        max_candidates: 20,
    };
    let envelope = PurposeEnvelope {
        envelope_id: "normalizer-v1".to_owned(),
        purpose_sha256: purpose_sha256(&purpose).expect("purpose hash"),
        environment_sha256: environment_sha256(&environment).expect("environment hash"),
        hard_constraints_sha256: hard_constraints_sha256(&hard_constraints)
            .expect("constraint hash"),
        metric_policy_sha256: metric_policy_sha256(&metrics).expect("metric hash"),
        transformation_policy_sha256: transformation_policy_sha256(&policy).expect("policy hash"),
    };
    TransformationRequest {
        schema_version: TRANSFORMATION_REQUEST_SCHEMA.to_owned(),
        request_id: "normalizer-metabolism".to_owned(),
        search_basis_sha256: sha256_hex(b"normalizer-search-v1"),
        subject: GeneratorSubject::Brownfield {
            repository_root: root.display().to_string(),
            revision: "fixture-baseline".to_owned(),
            source_tree_sha256: sha256_hex(b"normalizer-tree-v1"),
        },
        envelope,
        purpose,
        environment,
        hard_constraints,
        metrics,
        policy,
    }
}

fn python_command(mode: &str) -> BoundCommand {
    let program = "/usr/bin/python3";
    BoundCommand {
        program: BoundProgram {
            path: program.to_owned(),
            sha256: digest_file(Path::new(program)),
        },
        external_files: vec![],
        arguments: vec!["harness.py".to_owned(), mode.to_owned()],
        cwd: ".".to_owned(),
        environment: BTreeMap::new(),
        stdin: String::new(),
    }
}

fn plan(fixture: &Fixture, candidate_id: String) -> MetabolismPlan {
    MetabolismPlan {
        schema_version: METABOLISM_PLAN_SCHEMA.to_owned(),
        plan_id: "normalizer-extraction-v1".to_owned(),
        candidate_id,
        source_revision: "fixture-baseline".to_owned(),
        source_manifest: vec![
            BoundSourceFile {
                path: "left.py".to_owned(),
                sha256: digest_file(&fixture.source.join("left.py")),
            },
            BoundSourceFile {
                path: "right.py".to_owned(),
                sha256: digest_file(&fixture.source.join("right.py")),
            },
            BoundSourceFile {
                path: "harness.py".to_owned(),
                sha256: digest_file(&fixture.source.join("harness.py")),
            },
        ],
        isolation: IsolationPolicy {
            workspace_root: fixture.workspace.display().to_string(),
            copy_paths: vec![
                "left.py".to_owned(),
                "right.py".to_owned(),
                "harness.py".to_owned(),
            ],
            excluded_names: vec![],
            max_files: 10,
            max_bytes: 100_000,
        },
        mutations: vec![
            FileMutation::ReplaceText {
                path: "left.py".to_owned(),
                preimage_sha256: digest_file(&fixture.source.join("left.py")),
                replacements: vec![LiteralReplacement {
                    old: baseline_module().to_owned(),
                    new: wrapper_module().to_owned(),
                    expected_occurrences: 1,
                }],
            },
            FileMutation::ReplaceText {
                path: "right.py".to_owned(),
                preimage_sha256: digest_file(&fixture.source.join("right.py")),
                replacements: vec![LiteralReplacement {
                    old: baseline_module().to_owned(),
                    new: wrapper_module().to_owned(),
                    expected_occurrences: 1,
                }],
            },
            FileMutation::CreateText {
                path: "shared.py".to_owned(),
                content: shared_module().to_owned(),
                postimage_sha256: sha256_hex(shared_module().as_bytes()),
            },
        ],
        checks: vec![VerificationCheck {
            id: "observable-mapping".to_owned(),
            kind: CheckKind::StdoutParity,
            command: python_command("check"),
            protected_observation_ids: vec!["entrypoint-parity".to_owned()],
            hard_constraint_ids: vec!["tests-pass".to_owned()],
        }],
        negative_controls: vec![NegativeControl {
            id: "known-bad-input".to_owned(),
            command: python_command("negative"),
            expected_exit_code: 7,
        }],
        metrics: vec![MetricProbe {
            metric_id: "source_bytes".to_owned(),
            command: python_command("metric"),
        }],
        forbidden_regression_metrics: vec!["source_bytes".to_owned()],
        model_rebuild: ModelRebuild {
            command: python_command("model"),
            model_path: "rebuilt.model.json".to_owned(),
            implementation_subject_id: "implementation".to_owned(),
        },
        nonclaims: vec![
            "fixture evidence does not establish universal source equivalence".to_owned(),
        ],
    }
}

#[test]
fn accepts_only_a_real_measured_change_with_a_rebuilt_model() {
    let fixture = Fixture::new();
    let request = request(&fixture.source);
    let batch = generate_transformations(&request).expect("generated candidates");
    let candidate = batch
        .candidates
        .iter()
        .find(|item| item.family == TransformationFamily::IsomorphicSimplification)
        .expect("duplicate extraction candidate");
    let plan = plan(&fixture, candidate.id.clone());
    let receipt = execute_metabolism(&request, &batch, &plan).expect("completed metabolism");
    assert_eq!(receipt.status, MetabolismStatus::Accepted);
    assert!(receipt.accepted);
    assert!(receipt.counterexamples.is_empty());
    assert!(receipt.metric_receipts[0].candidate < receipt.metric_receipts[0].baseline);
    assert!(receipt.command_receipts.iter().all(|item| item.passed));
    assert_eq!(
        receipt
            .model_receipt
            .as_ref()
            .expect("model receipt")
            .model_revision,
        receipt.candidate_revision.expect("candidate revision")
    );
}

#[test]
fn a_forbidden_metric_regression_is_retained_as_a_counterexample() {
    let fixture = Fixture::new();
    let request = request(&fixture.source);
    let batch = generate_transformations(&request).expect("generated candidates");
    let candidate = batch
        .candidates
        .iter()
        .find(|item| item.family == TransformationFamily::IsomorphicSimplification)
        .expect("duplicate extraction candidate");
    let mut plan = plan(&fixture, candidate.id.clone());
    let larger = format!("{}\n# deliberately larger candidate\n", baseline_module());
    plan.mutations = vec![FileMutation::ReplaceText {
        path: "left.py".to_owned(),
        preimage_sha256: digest_file(&fixture.source.join("left.py")),
        replacements: vec![LiteralReplacement {
            old: baseline_module().to_owned(),
            new: larger,
            expected_occurrences: 1,
        }],
    }];
    let receipt = execute_metabolism(&request, &batch, &plan).expect("completed metabolism");
    assert_eq!(receipt.status, MetabolismStatus::Rejected);
    assert!(!receipt.accepted);
    assert!(receipt.metric_receipts[0].forbidden_regression);
    assert!(
        receipt
            .counterexamples
            .iter()
            .any(|item| item.contains("regressed"))
    );
}

#[test]
fn a_drifted_interpreter_input_is_rejected_before_spawn() {
    let fixture = Fixture::new();
    let request = request(&fixture.source);
    let batch = generate_transformations(&request).expect("generated candidates");
    let candidate = batch
        .candidates
        .iter()
        .find(|item| item.family == TransformationFamily::IsomorphicSimplification)
        .expect("duplicate extraction candidate");
    let external = fixture
        .source
        .parent()
        .expect("fixture base")
        .join("external-probe.py");
    fs::write(&external, "print('bound')\n").expect("external input");
    let mut plan = plan(&fixture, candidate.id.clone());
    plan.checks[0].command.external_files = vec![BoundExternalFile {
        path: external.display().to_string(),
        sha256: digest_file(&external),
    }];
    fs::write(&external, "print('drifted')\n").expect("drift external input");
    let error = execute_metabolism(&request, &batch, &plan).expect_err("drift must fail closed");
    assert!(error.message.contains("external input"));
}
