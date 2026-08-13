use kerf::report::sha256_hex;
use semantic_architecture::TOOL_VERSION;
use semantic_architecture::converge::{
    ConvergenceStatus, DECISION_CONTRACT_SCHEMA, DecisionContract, EligibleTest, EvaluatorIdentity,
    EvidencePackage, FiniteUniverse, MandatoryClaimRef, current_evaluator_identity, evaluate,
};
use semantic_architecture::manifest::{
    AuthorityEvidenceOrigin, BoundFile, BuildAttestation, CheckoutBinding, DirtyBinding,
    EXECUTION_PLAN_SCHEMA, ExecutedClaimCoordinate, ExecutedClaimPlan, ExecutedClaimSubject,
    ExecutionPlan, ExpectedObservation, GitFixtureEvidence, GitObjectPin, InvocationPlan,
    ManifestStatus, NegativeControlPlan, SubjectPin, ToolchainPin, ValidatorAuthority,
    ValidatorKind, ValidatorPlan, execute_plan,
};
use semantic_architecture::semantic::{ClaimKind, ClaimVerdict, ClosureKind, EpistemicClass};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_ID: AtomicU64 = AtomicU64::new(0);

struct Fixture {
    root: PathBuf,
    package: EvidencePackage,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[allow(clippy::too_many_lines)]
fn fixture() -> Fixture {
    let serial = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "semantic-architecture-converge-{}-{serial}",
        std::process::id()
    ));
    let repository = root.join("repository");
    fs::create_dir_all(&repository).expect("repository");
    fs::write(repository.join("Cargo.lock"), b"fixture-lock").expect("lock");
    let expectations = serde_json::to_vec_pretty(&serde_json::json!({
        "hello_sha256": sha256_hex(b"hello"),
        "wrong_sha256": sha256_hex(b"wrong")
    }))
    .expect("expectations");
    fs::write(repository.join("expectations.json"), &expectations).expect("expectations file");
    git(&repository, &["init", "-q"]);
    git(&repository, &["add", "Cargo.lock", "expectations.json"]);
    let status = Command::new("/usr/bin/git")
        .args([
            "-C",
            repository.to_str().expect("repo"),
            "commit",
            "-qm",
            "fixture",
        ])
        .env_clear()
        .env("GIT_AUTHOR_NAME", "Fixture")
        .env("GIT_AUTHOR_EMAIL", "fixture@example.invalid")
        .env("GIT_COMMITTER_NAME", "Fixture")
        .env("GIT_COMMITTER_EMAIL", "fixture@example.invalid")
        .status()
        .expect("commit");
    assert!(status.success());
    let commit = git_output(&repository, &["rev-parse", "HEAD"]);
    let tree = git_output(&repository, &["rev-parse", "HEAD^{tree}"]);
    fs::write(repository.join("candidate-marker"), b"candidate").expect("candidate marker");
    git(&repository, &["add", "candidate-marker"]);
    let status = Command::new("/usr/bin/git")
        .args([
            "-C",
            repository.to_str().expect("repo"),
            "commit",
            "-qm",
            "candidate",
        ])
        .env_clear()
        .env("GIT_AUTHOR_NAME", "Fixture")
        .env("GIT_AUTHOR_EMAIL", "fixture@example.invalid")
        .env("GIT_COMMITTER_NAME", "Fixture")
        .env("GIT_COMMITTER_EMAIL", "fixture@example.invalid")
        .status()
        .expect("candidate commit");
    assert!(status.success());
    let candidate_commit = git_output(&repository, &["rev-parse", "HEAD"]);
    let candidate_tree = git_output(&repository, &["rev-parse", "HEAD^{tree}"]);
    let printf = bound("/usr/bin/printf");
    let toolchain = ToolchainPin {
        program: printf.clone(),
        args: vec!["toolchain".to_owned()],
        stdout_sha256: sha256_hex(b"toolchain"),
        target: "fixture-target".to_owned(),
        profile: "test".to_owned(),
    };
    let subject =
        |id: &str, commit: &str, tree: &str, checkout: CheckoutBinding, dirty: DirtyBinding| {
            SubjectPin {
                id: id.to_owned(),
                repository: repository.to_string_lossy().into_owned(),
                git: GitObjectPin {
                    commit: commit.to_owned(),
                    tree: tree.to_owned(),
                },
                checkout,
                dirty,
                executable: printf.clone(),
                locks: vec![bound(repository.join("Cargo.lock"))],
                toolchain: toolchain.clone(),
                build_attestation: BuildAttestation::None,
            }
        };
    let invocation = |id: &str, subject_id: &str| InvocationPlan {
        id: id.to_owned(),
        case_id: "hello-case".to_owned(),
        subject_id: subject_id.to_owned(),
        argv: vec![printf.path.clone(), "hello".to_owned()],
        cwd: repository.to_string_lossy().into_owned(),
        environment: BTreeMap::new(),
        stdin_hex: String::new(),
        input_topologies: Vec::new(),
        output_root: None,
    };
    let evidence = |pointer: &str| GitFixtureEvidence {
        subject_id: "baseline".to_owned(),
        commit: commit.clone(),
        path: "expectations.json".to_owned(),
        sha256: sha256_hex(&expectations),
        origin: AuthorityEvidenceOrigin::ProspectiveBaseline,
        json_pointer: Some(pointer.to_owned()),
    };
    let plan = ExecutionPlan {
        schema_version: EXECUTION_PLAN_SCHEMA.to_owned(),
        manifest_id: "manifest".to_owned(),
        decision_id: "decision".to_owned(),
        closure: ClosureKind::CompleteForDeclaredCorpus,
        git_program: bound("/usr/bin/git"),
        subjects: vec![
            subject(
                "baseline",
                &commit,
                &tree,
                CheckoutBinding::ObjectOnly,
                DirtyBinding::Unattested,
            ),
            subject(
                "candidate",
                &candidate_commit,
                &candidate_tree,
                CheckoutBinding::CurrentCheckout,
                DirtyBinding::Clean {
                    patch_sha256: clean_dirty_digest(),
                },
            ),
        ],
        invocations: vec![
            invocation("baseline-run", "baseline"),
            invocation("candidate-run", "candidate"),
        ],
        validators: vec![
            ValidatorPlan {
                id: "parity".to_owned(),
                kind: ValidatorKind::ExactStdout,
                authority: ValidatorAuthority::IndependentFixture,
                observed_invocation_id: "candidate-run".to_owned(),
                expected: ExpectedObservation::Literal {
                    value: sha256_hex(b"hello"),
                },
                authority_evidence: Some(evidence("/hello_sha256")),
            },
            ValidatorPlan {
                id: "wrong-control".to_owned(),
                kind: ValidatorKind::ExactStdout,
                authority: ValidatorAuthority::IndependentFixture,
                observed_invocation_id: "candidate-run".to_owned(),
                expected: ExpectedObservation::Literal {
                    value: sha256_hex(b"wrong"),
                },
                authority_evidence: Some(evidence("/wrong_sha256")),
            },
            ValidatorPlan {
                id: "baseline-observation".to_owned(),
                kind: ValidatorKind::ExactStdout,
                authority: ValidatorAuthority::SelfProduced,
                observed_invocation_id: "baseline-run".to_owned(),
                expected: ExpectedObservation::Literal {
                    value: sha256_hex(b"hello"),
                },
                authority_evidence: None,
            },
        ],
        claims: vec![ExecutedClaimPlan {
            id: "parity-claim".to_owned(),
            description: "candidate matches the declared fixture".to_owned(),
            subject: ExecutedClaimSubject {
                coordinate: ExecutedClaimCoordinate::BoundExecutable,
                subject_ids: vec!["baseline".to_owned(), "candidate".to_owned()],
            },
            kind: ClaimKind::Equivalent,
            epistemic_class: EpistemicClass::BoundedCheck,
            closure: ClosureKind::CompleteForDeclaredCorpus,
            validator_ids: vec!["parity".to_owned(), "baseline-observation".to_owned()],
            negative_control_id: "wrong-is-rejected".to_owned(),
        }],
        negative_controls: vec![NegativeControlPlan {
            id: "wrong-is-rejected".to_owned(),
            claim_id: "parity-claim".to_owned(),
            validator_id: "wrong-control".to_owned(),
        }],
        nonclaims: vec!["fixture only".to_owned()],
    };
    let manifest = execute_plan(&plan).expect("execute plan");
    assert_eq!(manifest.status, ManifestStatus::Accepted);
    Fixture {
        root,
        package: EvidencePackage { plan, manifest },
    }
}

fn contract(package: &EvidencePackage) -> DecisionContract {
    let plan_sha256 = sha256_hex(&serde_json::to_vec(&package.plan).expect("plan"));
    DecisionContract {
        schema_version: DECISION_CONTRACT_SCHEMA.to_owned(),
        decision_id: "decision".to_owned(),
        retirement_condition: "fixture parity supported".to_owned(),
        retirement_condition_met: true,
        evaluator: current_evaluator_identity().expect("evaluator identity"),
        blockers: Vec::new(),
        mandatory_claims: vec![MandatoryClaimRef {
            plan_sha256,
            claim_id: "parity-claim".to_owned(),
            required_closure: ClosureKind::CompleteForDeclaredCorpus,
        }],
        eligible_tests: Vec::new(),
        generators_exhausted: true,
        finite_universe: None,
        budget_exhausted: false,
        prior_state_sha256: None,
        provider_versions: BTreeMap::from([("fixture".to_owned(), "v1".to_owned())]),
        nonclaims: vec!["not universal equivalence".to_owned()],
    }
}

fn cli_evaluator_identity() -> EvaluatorIdentity {
    let output = Command::new(env!("CARGO_BIN_EXE_kerf-evidence"))
        .arg("identity")
        .output()
        .expect("evaluator identity CLI");
    assert!(output.status.success(), "{:?}", output.stderr);
    let envelope: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("identity JSON");
    serde_json::from_value(envelope["result"].clone()).expect("evaluator identity result")
}

fn bound(path: impl AsRef<Path>) -> BoundFile {
    let path = path.as_ref();
    BoundFile {
        path: path.to_string_lossy().into_owned(),
        sha256: sha256_hex(&fs::read(path).expect("bound file")),
    }
}

fn git(repository: &Path, args: &[&str]) {
    assert!(
        Command::new("/usr/bin/git")
            .arg("-C")
            .arg(repository)
            .args(args)
            .env_clear()
            .status()
            .expect("git")
            .success()
    );
}

fn git_output(repository: &Path, args: &[&str]) -> String {
    let output = Command::new("/usr/bin/git")
        .arg("-C")
        .arg(repository)
        .args(args)
        .env_clear()
        .output()
        .expect("git output");
    assert!(output.status.success());
    String::from_utf8(output.stdout)
        .expect("UTF-8")
        .trim()
        .to_owned()
}

fn clean_dirty_digest() -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"kerf.git-dirty.v1\0");
    hasher.update(0_u64.to_le_bytes());
    format!("{:x}", hasher.finalize())
}

#[test]
fn fixed_point_requires_one_unchanged_state_sweep() {
    let fixture = fixture();
    let mut contract = contract(&fixture.package);
    let first = evaluate(&contract, std::slice::from_ref(&fixture.package)).expect("first");
    assert_eq!(first.status, ConvergenceStatus::Open);
    contract.prior_state_sha256 = Some(first.canonical_state_sha256.clone());
    let second = evaluate(&contract, std::slice::from_ref(&fixture.package)).expect("second");
    assert_eq!(second.status, ConvergenceStatus::DecisionFixedPoint);
}

#[test]
fn evaluator_content_identity_invalidates_a_prior_fixed_point() {
    let fixture = fixture();
    let mut executable_contract = contract(&fixture.package);
    let first =
        evaluate(&executable_contract, std::slice::from_ref(&fixture.package)).expect("first");
    executable_contract.prior_state_sha256 = Some(first.canonical_state_sha256);
    executable_contract.evaluator.executable_sha256 = "0".repeat(64);
    let error = evaluate(&executable_contract, std::slice::from_ref(&fixture.package))
        .expect_err("changed evaluator identity must reject");
    assert!(error.message.contains("evaluator identity"));

    let mut skill_contract = contract(&fixture.package);
    skill_contract.evaluator.skill_sha256 = "0".repeat(64);
    let error = evaluate(&skill_contract, std::slice::from_ref(&fixture.package))
        .expect_err("changed skill identity must reject");
    assert!(error.message.contains("evaluator identity"));
}

#[test]
fn bounded_exhaustion_derives_explicit_member_coverage() {
    let fixture = fixture();
    let mut contract = contract(&fixture.package);
    contract.finite_universe = Some(FiniteUniverse {
        id: "fixtures".to_owned(),
        closure: ClosureKind::ExhaustiveFinite,
        declared_members: vec!["hello-case".to_owned()],
    });
    let first = evaluate(&contract, std::slice::from_ref(&fixture.package)).expect("first");
    contract.prior_state_sha256 = Some(first.canonical_state_sha256);
    let second = evaluate(&contract, std::slice::from_ref(&fixture.package)).expect("second");
    assert_eq!(
        second.status,
        ConvergenceStatus::BoundedExhaustiveFixedPoint
    );
    assert_eq!(second.observed_finite_members, vec!["hello-case"]);
}

#[test]
fn non_mandatory_claim_cannot_pad_decision_coverage() {
    let mut fixture = fixture();
    let mut padded = fixture.package.plan.invocations[1].clone();
    padded.id = "candidate-padding-run".to_owned();
    padded.case_id = "unearned-case".to_owned();
    let mut control_invocation = fixture.package.plan.invocations[1].clone();
    control_invocation.id = "candidate-padding-control-run".to_owned();
    control_invocation.case_id = "unearned-control".to_owned();
    fixture
        .package
        .plan
        .invocations
        .extend([padded, control_invocation]);

    let mut support = fixture.package.plan.validators[0].clone();
    support.id = "padding-support".to_owned();
    support.observed_invocation_id = "candidate-padding-run".to_owned();
    let mut control = fixture.package.plan.validators[1].clone();
    control.id = "padding-control-validator".to_owned();
    control.observed_invocation_id = "candidate-padding-control-run".to_owned();
    fixture.package.plan.validators.extend([support, control]);
    fixture.package.plan.claims.push(ExecutedClaimPlan {
        id: "non-mandatory-padding-claim".to_owned(),
        description: "candidate-only auxiliary observation".to_owned(),
        subject: ExecutedClaimSubject {
            coordinate: ExecutedClaimCoordinate::BoundExecutable,
            subject_ids: vec!["candidate".to_owned()],
        },
        kind: ClaimKind::RequirementSatisfied,
        epistemic_class: EpistemicClass::BoundedCheck,
        closure: ClosureKind::CompleteForDeclaredCorpus,
        validator_ids: vec!["padding-support".to_owned()],
        negative_control_id: "padding-wrong-is-rejected".to_owned(),
    });
    fixture
        .package
        .plan
        .negative_controls
        .push(NegativeControlPlan {
            id: "padding-wrong-is-rejected".to_owned(),
            claim_id: "non-mandatory-padding-claim".to_owned(),
            validator_id: "padding-control-validator".to_owned(),
        });
    fixture.package.manifest = execute_plan(&fixture.package.plan).expect("padded plan");
    assert_eq!(fixture.package.manifest.status, ManifestStatus::Accepted);

    let mut contract = contract(&fixture.package);
    contract.finite_universe = Some(FiniteUniverse {
        id: "mandatory-only".to_owned(),
        closure: ClosureKind::ExhaustiveFinite,
        declared_members: vec!["hello-case".to_owned()],
    });
    let result = evaluate(&contract, std::slice::from_ref(&fixture.package)).expect("evaluate");
    assert_eq!(result.observed_finite_members, vec!["hello-case"]);
    assert!(
        !result
            .observed_finite_members
            .contains(&"unearned-case".to_owned())
    );
}

#[test]
fn fake_member_counts_cannot_establish_exhaustion() {
    let fixture = fixture();
    let mut contract = contract(&fixture.package);
    contract.finite_universe = Some(FiniteUniverse {
        id: "fixtures".to_owned(),
        closure: ClosureKind::ExhaustiveFinite,
        declared_members: vec!["hello-case".to_owned(), "missing-case".to_owned()],
    });
    contract.prior_state_sha256 = Some("0".repeat(64));
    let result = evaluate(&contract, std::slice::from_ref(&fixture.package)).expect("open");
    assert_eq!(result.status, ConvergenceStatus::Open);
    assert!(
        result
            .witness
            .is_some_and(|value| value.contains("missing-case"))
    );
}

#[test]
fn budget_expiry_can_never_be_success() {
    let fixture = fixture();
    let mut contract = contract(&fixture.package);
    contract.budget_exhausted = true;
    let result = evaluate(&contract, std::slice::from_ref(&fixture.package)).expect("budget");
    assert_eq!(
        result.status,
        ConvergenceStatus::BudgetExhaustedWithUnknowns
    );
}

#[test]
fn zero_information_test_is_rejected() {
    let fixture = fixture();
    let mut contract = contract(&fixture.package);
    contract.eligible_tests.push(EligibleTest {
        id: "zero-gain".to_owned(),
        affected_claims: contract.mandatory_claims.clone(),
        legal_outcomes: vec![ClaimVerdict::Supported],
    });
    let error = evaluate(&contract, std::slice::from_ref(&fixture.package)).expect_err("zero gain");
    assert!(error.message.contains("no information gain"));
}

#[test]
fn tampered_manifest_cannot_reach_fixed_point() {
    let mut fixture = fixture();
    fixture.package.manifest.invocation_receipts[0].executed = false;
    let contract = contract(&fixture.package);
    let result = evaluate(&contract, &[fixture.package.clone()]).expect("binding");
    assert_eq!(result.status, ConvergenceStatus::BindingRejected);
}

#[test]
fn contract_is_bound_to_exact_plan_hash_and_closure() {
    let fixture = fixture();
    let mut contract = contract(&fixture.package);
    contract.mandatory_claims[0].plan_sha256 = "0".repeat(64);
    let result = evaluate(&contract, std::slice::from_ref(&fixture.package)).expect("open");
    assert_eq!(result.status, ConvergenceStatus::Open);
}

#[test]
fn required_claim_closure_is_enforced() {
    let fixture = fixture();
    let mut contract = contract(&fixture.package);
    contract.mandatory_claims[0].required_closure = ClosureKind::ObservedOnly;
    let result = evaluate(&contract, std::slice::from_ref(&fixture.package)).expect("open");
    assert_eq!(result.status, ConvergenceStatus::Open);
    assert!(
        result
            .witness
            .is_some_and(|value| value.contains("closure"))
    );
}

#[test]
fn empty_mandatory_claim_set_is_invalid() {
    let fixture = fixture();
    let mut contract = contract(&fixture.package);
    contract.mandatory_claims.clear();
    let error =
        evaluate(&contract, std::slice::from_ref(&fixture.package)).expect_err("empty claims");
    assert!(error.message.contains("mandatory_claims"));
}

#[test]
fn failed_negative_control_cannot_contribute_to_fixed_point() {
    let mut fixture = fixture();
    fixture.package.plan.validators[1].expected = ExpectedObservation::Literal {
        value: sha256_hex(b"hello"),
    };
    let evidence = fixture.package.plan.validators[1]
        .authority_evidence
        .as_mut()
        .expect("evidence");
    evidence.json_pointer = Some("/hello_sha256".to_owned());
    fixture.package.manifest = execute_plan(&fixture.package.plan).expect("execute control");
    assert_eq!(
        fixture.package.manifest.status,
        ManifestStatus::NegativeControlFailed
    );
    let contract = contract(&fixture.package);
    let result = evaluate(&contract, &[fixture.package.clone()]).expect("binding result");
    assert_eq!(result.status, ConvergenceStatus::BindingRejected);
}

#[test]
fn changing_nonclaim_changes_canonical_state() {
    let fixture = fixture();
    let first = evaluate(
        &contract(&fixture.package),
        std::slice::from_ref(&fixture.package),
    )
    .expect("first");
    let mut changed = contract(&fixture.package);
    changed.nonclaims.push("a new decision boundary".to_owned());
    let second = evaluate(&changed, std::slice::from_ref(&fixture.package)).expect("second");
    assert_ne!(first.canonical_state_sha256, second.canonical_state_sha256);
}

#[test]
fn irrelevant_raw_diagnostics_do_not_prevent_semantic_convergence() {
    let fixture = fixture();
    let first = evaluate(
        &contract(&fixture.package),
        std::slice::from_ref(&fixture.package),
    )
    .expect("first");
    let mut noisy = fixture.package.clone();
    noisy.manifest.invocation_receipts[0].stderr_sha256 = "a".repeat(64);
    noisy.manifest.invocation_receipts[0].stderr_len = 123;
    let second = evaluate(&contract(&noisy), std::slice::from_ref(&noisy)).expect("second");
    assert_ne!(first.package_sha256s, second.package_sha256s);
    assert_eq!(
        first.decision_projection_sha256s,
        second.decision_projection_sha256s
    );
    assert_eq!(first.canonical_state_sha256, second.canonical_state_sha256);
}

#[test]
fn evidence_cli_composes_execute_envelopes_and_exposes_a_fixed_point_gate() {
    let fixture = fixture();
    let package_path = fixture.root.join("package-envelope.json");
    let contract_path = fixture.root.join("contract.json");
    let package = serde_json::json!({
        "schema_version": "kerf.evidence-result.v1",
        "tool_version": TOOL_VERSION,
        "operation": "execute",
        "input_sha256s": {"plan": "0".repeat(64)},
        "status": "ok",
        "result": fixture.package
    });
    fs::write(
        &package_path,
        serde_json::to_vec_pretty(&package).expect("package JSON"),
    )
    .expect("package file");

    let mut decision = contract(&fixture.package);
    decision.evaluator = cli_evaluator_identity();
    fs::write(
        &contract_path,
        serde_json::to_vec_pretty(&decision).expect("contract JSON"),
    )
    .expect("contract file");
    let first = Command::new(env!("CARGO_BIN_EXE_kerf-evidence"))
        .args(["evaluate"])
        .arg(&contract_path)
        .arg(&package_path)
        .output()
        .expect("first evaluation");
    assert!(first.status.success(), "{:?}", first.stderr);
    let first_json: serde_json::Value =
        serde_json::from_slice(&first.stdout).expect("first output JSON");
    assert_eq!(first_json["result"]["status"], "open");

    let gated_open = Command::new(env!("CARGO_BIN_EXE_kerf-evidence"))
        .args(["evaluate"])
        .arg(&contract_path)
        .arg(&package_path)
        .arg("--require-fixed-point")
        .output()
        .expect("gated open evaluation");
    assert_eq!(gated_open.status.code(), Some(3));
    let error_json: serde_json::Value =
        serde_json::from_slice(&gated_open.stderr).expect("gate error JSON");
    assert_eq!(error_json["error"]["code"], "gate_unsatisfied");

    decision.prior_state_sha256 = first_json["result"]["canonical_state_sha256"]
        .as_str()
        .map(ToOwned::to_owned);
    fs::write(
        &contract_path,
        serde_json::to_vec_pretty(&decision).expect("stable contract JSON"),
    )
    .expect("stable contract file");
    let fixed = Command::new(env!("CARGO_BIN_EXE_kerf-evidence"))
        .args(["evaluate"])
        .arg(&contract_path)
        .arg(&package_path)
        .arg("--require-fixed-point")
        .output()
        .expect("fixed-point evaluation");
    assert!(fixed.status.success(), "{:?}", fixed.stderr);
    let fixed_json: serde_json::Value =
        serde_json::from_slice(&fixed.stdout).expect("fixed output JSON");
    assert_eq!(fixed_json["result"]["status"], "decision_fixed_point");
}
