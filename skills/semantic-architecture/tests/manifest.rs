use kerf::report::sha256_hex;
use semantic_architecture::manifest::{
    AuthorityEvidenceOrigin, BoundFile, BuildAttestation, CheckoutBinding, DirtyBinding,
    EXECUTION_PLAN_SCHEMA, ExecutedClaimCoordinate, ExecutedClaimPlan, ExecutedClaimSubject,
    ExecutionPlan, ExpectedObservation, GitFixtureEvidence, GitObjectPin, InvocationCommand,
    InvocationPlan, ManifestStatus, NegativeControlPlan, PARITY_CORPUS_SCHEMA, ParityCorpusCase,
    ParityCorpusClaim, ParityCorpusFixture, ParityCorpusNegativeControl, ParityCorpusSpec,
    SubjectPin, ToolchainPin, TopologyNodeKind, TopologyPin, ValidatorAuthority, ValidatorKind,
    ValidatorPlan, audit_manifest, execute_plan, expand_parity_corpus, snapshot_topology,
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
    repository: PathBuf,
    input_root: PathBuf,
    plan: ExecutionPlan,
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
        "semantic-architecture-manifest-{}-{serial}",
        std::process::id()
    ));
    fs::create_dir(&root).expect("fixture root");
    let repository = root.join("repository");
    let input_root = root.join("input");
    fs::create_dir(&repository).expect("repository");
    fs::create_dir(&input_root).expect("input root");
    fs::create_dir(input_root.join("empty")).expect("empty input directory");
    fs::write(input_root.join("input.txt"), b"input").expect("input file");
    fs::write(repository.join("Cargo.lock"), b"fixture-lock").expect("lock file");
    let fixture_expectations = serde_json::to_vec_pretty(&serde_json::json!({
        "hello_sha256": sha256_hex(b"hello"),
        "not_hello_sha256": sha256_hex(b"not-hello"),
        "empty_sha256": sha256_hex(b"")
    }))
    .expect("fixture expectations");
    fs::write(repository.join("expectations.json"), &fixture_expectations)
        .expect("fixture expectations file");
    git(&repository, &["init", "-q"]);
    git(&repository, &["add", "Cargo.lock", "expectations.json"]);
    let status = Command::new("/usr/bin/git")
        .args([
            "-C",
            repository.to_str().expect("repository path"),
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
        .expect("git commit");
    assert!(status.success());
    let commit = git_output(&repository, &["rev-parse", "HEAD"]);
    let tree = git_output(&repository, &["rev-parse", "HEAD^{tree}"]);
    fs::write(repository.join("candidate-marker"), b"candidate").expect("candidate marker");
    git(&repository, &["add", "candidate-marker"]);
    let status = Command::new("/usr/bin/git")
        .args([
            "-C",
            repository.to_str().expect("repository path"),
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
    let git_program = bound("/usr/bin/git");
    let lock = bound(repository.join("Cargo.lock"));
    let toolchain = ToolchainPin {
        program: printf.clone(),
        args: vec!["fixture-toolchain".to_owned()],
        stdout_sha256: sha256_hex(b"fixture-toolchain"),
        target: "fixture-target".to_owned(),
        profile: "test".to_owned(),
    };
    let baseline = SubjectPin {
        id: "baseline".to_owned(),
        repository: repository.to_string_lossy().into_owned(),
        git: GitObjectPin {
            commit: commit.clone(),
            tree: tree.clone(),
        },
        checkout: CheckoutBinding::ObjectOnly,
        dirty: DirtyBinding::Unattested,
        executable: printf.clone(),
        locks: vec![lock.clone()],
        toolchain: toolchain.clone(),
        build_attestation: BuildAttestation::None,
    };
    let candidate = SubjectPin {
        id: "candidate".to_owned(),
        repository: repository.to_string_lossy().into_owned(),
        git: GitObjectPin {
            commit: candidate_commit,
            tree: candidate_tree,
        },
        checkout: CheckoutBinding::CurrentCheckout,
        dirty: DirtyBinding::Clean {
            patch_sha256: clean_dirty_digest(),
        },
        executable: printf.clone(),
        locks: vec![lock],
        toolchain,
        build_attestation: BuildAttestation::None,
    };
    let topology = snapshot_topology(&input_root).expect("input topology");
    let invocation = |id: &str, subject_id: &str, value: &str| InvocationPlan {
        id: id.to_owned(),
        case_id: "hello-case".to_owned(),
        subject_id: subject_id.to_owned(),
        argv: vec![printf.path.clone(), value.to_owned()],
        cwd: repository.to_string_lossy().into_owned(),
        environment: BTreeMap::new(),
        stdin_hex: String::new(),
        input_topologies: vec![TopologyPin {
            root: input_root.to_string_lossy().into_owned(),
            digest: topology.digest.clone(),
        }],
        output_root: None,
    };
    let fixture_evidence = |pointer: &str| GitFixtureEvidence {
        subject_id: "baseline".to_owned(),
        commit: commit.clone(),
        path: "expectations.json".to_owned(),
        sha256: sha256_hex(&fixture_expectations),
        origin: AuthorityEvidenceOrigin::ProspectiveBaseline,
        json_pointer: Some(pointer.to_owned()),
    };
    let plan = ExecutionPlan {
        schema_version: EXECUTION_PLAN_SCHEMA.to_owned(),
        manifest_id: "fixture-manifest".to_owned(),
        decision_id: "fixture-decision".to_owned(),
        closure: ClosureKind::CompleteForDeclaredCorpus,
        git_program,
        subjects: vec![baseline, candidate],
        invocations: vec![
            invocation("baseline-run", "baseline", "hello"),
            invocation("candidate-run", "candidate", "hello"),
        ],
        validators: vec![
            ValidatorPlan {
                id: "stdout-parity".to_owned(),
                kind: ValidatorKind::ExactStdout,
                authority: ValidatorAuthority::IndependentFixture,
                observed_invocation_id: "candidate-run".to_owned(),
                expected: ExpectedObservation::Literal {
                    value: sha256_hex(b"hello"),
                },
                authority_evidence: Some(fixture_evidence("/hello_sha256")),
            },
            ValidatorPlan {
                id: "wrong-output-control".to_owned(),
                kind: ValidatorKind::ExactStdout,
                authority: ValidatorAuthority::IndependentFixture,
                observed_invocation_id: "candidate-run".to_owned(),
                expected: ExpectedObservation::Literal {
                    value: sha256_hex(b"not-hello"),
                },
                authority_evidence: Some(fixture_evidence("/not_hello_sha256")),
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
            id: "output-contract".to_owned(),
            description: "candidate emits the independently declared fixture output".to_owned(),
            subject: ExecutedClaimSubject {
                coordinate: ExecutedClaimCoordinate::BoundExecutable,
                subject_ids: vec!["baseline".to_owned(), "candidate".to_owned()],
            },
            kind: ClaimKind::Equivalent,
            epistemic_class: EpistemicClass::BoundedCheck,
            closure: ClosureKind::CompleteForDeclaredCorpus,
            validator_ids: vec![
                "stdout-parity".to_owned(),
                "baseline-observation".to_owned(),
            ],
            negative_control_id: "wrong-output-is-rejected".to_owned(),
        }],
        negative_controls: vec![NegativeControlPlan {
            id: "wrong-output-is-rejected".to_owned(),
            claim_id: "output-contract".to_owned(),
            validator_id: "wrong-output-control".to_owned(),
        }],
        nonclaims: vec!["fixture execution does not attest a build".to_owned()],
    };
    Fixture {
        root,
        repository,
        input_root,
        plan,
    }
}

fn bound(path: impl AsRef<Path>) -> BoundFile {
    let path = path.as_ref();
    BoundFile {
        path: path.to_string_lossy().into_owned(),
        sha256: sha256_hex(&fs::read(path).expect("bound file")),
    }
}

fn git(repository: &Path, args: &[&str]) {
    let status = Command::new("/usr/bin/git")
        .arg("-C")
        .arg(repository)
        .args(args)
        .env_clear()
        .status()
        .expect("git");
    assert!(status.success(), "git {args:?}");
}

fn git_output(repository: &Path, args: &[&str]) -> String {
    let output = Command::new("/usr/bin/git")
        .arg("-C")
        .arg(repository)
        .args(args)
        .env_clear()
        .output()
        .expect("git output");
    assert!(output.status.success(), "git {args:?}");
    String::from_utf8(output.stdout)
        .expect("git UTF-8")
        .trim()
        .to_owned()
}

fn clean_dirty_digest() -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"kerf.git-dirty.v1\0");
    hasher.update(0_u64.to_le_bytes());
    format!("{:x}", hasher.finalize())
}

fn parity_spec(fixture: &Fixture) -> ParityCorpusSpec {
    let mut subjects = fixture.plan.subjects.clone();
    subjects[1].executable = bound("/usr/bin/echo");
    let baseline = &fixture.plan.invocations[0];
    let candidate = &fixture.plan.invocations[1];
    let command = |invocation: &InvocationPlan, argv: Vec<String>| InvocationCommand {
        argv,
        cwd: invocation.cwd.clone(),
        environment: invocation.environment.clone(),
        stdin_hex: invocation.stdin_hex.clone(),
        input_topologies: invocation.input_topologies.clone(),
        output_root: invocation.output_root.clone(),
    };
    let fixture_evidence = fixture.plan.validators[0]
        .authority_evidence
        .as_ref()
        .expect("fixture evidence");
    ParityCorpusSpec {
        schema_version: PARITY_CORPUS_SCHEMA.to_owned(),
        manifest_id: "expanded-parity-manifest".to_owned(),
        decision_id: "expanded-parity-decision".to_owned(),
        closure: ClosureKind::CompleteForDeclaredCorpus,
        git_program: fixture.plan.git_program.clone(),
        subjects,
        baseline_subject_id: "baseline".to_owned(),
        candidate_subject_id: "candidate".to_owned(),
        validator_kind: ValidatorKind::ExactStdout,
        fixture: ParityCorpusFixture {
            subject_id: fixture_evidence.subject_id.clone(),
            commit: fixture_evidence.commit.clone(),
            path: fixture_evidence.path.clone(),
            sha256: fixture_evidence.sha256.clone(),
        },
        cases: vec![ParityCorpusCase {
            id: "hello".to_owned(),
            baseline: command(
                baseline,
                vec!["/usr/bin/printf".to_owned(), "hello".to_owned()],
            ),
            candidate: command(
                candidate,
                vec![
                    "/usr/bin/echo".to_owned(),
                    "-n".to_owned(),
                    "hello".to_owned(),
                ],
            ),
            expected_literal: sha256_hex(b"hello"),
            fixture_json_pointer: "/hello_sha256".to_owned(),
        }],
        negative_control: ParityCorpusNegativeControl {
            id: "wrong-output-is-rejected".to_owned(),
            case_id: "wrong-output".to_owned(),
            invocation_id: "candidate-wrong-output-control".to_owned(),
            validator_id: "fixture-wrong-output-control".to_owned(),
            command: command(
                candidate,
                vec![
                    "/usr/bin/echo".to_owned(),
                    "-n".to_owned(),
                    "not-hello".to_owned(),
                ],
            ),
            expected_literal: sha256_hex(b"hello"),
            fixture_json_pointer: "/hello_sha256".to_owned(),
        },
        claim: ParityCorpusClaim {
            id: "hello-parity".to_owned(),
            description: "two distinct implementations emit the declared bytes".to_owned(),
            kind: ClaimKind::Equivalent,
            epistemic_class: EpistemicClass::BoundedCheck,
        },
        nonclaims: vec!["one finite case is not universal equivalence".to_owned()],
    }
}

#[test]
fn compact_parity_corpus_expands_and_executes_with_real_independent_evidence() {
    let fixture = fixture();
    let plan = expand_parity_corpus(&parity_spec(&fixture)).expect("expand parity corpus");
    assert_eq!(plan.invocations.len(), 3);
    assert_eq!(plan.validators.len(), 3);
    assert_eq!(plan.claims[0].validator_ids.len(), 2);
    assert_eq!(plan.invocations[0].id, "baseline-hello");
    assert_eq!(plan.invocations[1].id, "candidate-hello");
    let manifest = execute_plan(&plan).expect("execute expanded plan");
    assert_eq!(manifest.status, ManifestStatus::Accepted);
    assert_eq!(manifest.claim_receipts[0].verdict, ClaimVerdict::Supported);
    assert!(manifest.negative_control_receipts[0].passed);
}

#[test]
fn compact_parity_corpus_rejects_duplicate_case_identity() {
    let fixture = fixture();
    let mut spec = parity_spec(&fixture);
    spec.cases.push(spec.cases[0].clone());
    let error = expand_parity_corpus(&spec).expect_err("duplicate case must reject");
    assert!(error.message.contains("duplicate ID"));
}

#[test]
fn evidence_cli_expands_compact_parity_corpus_as_versioned_json() {
    let fixture = fixture();
    let spec_path = fixture.root.join("parity-spec.json");
    fs::write(
        &spec_path,
        serde_json::to_vec(&parity_spec(&fixture)).expect("serialize parity spec"),
    )
    .expect("write parity spec");
    let output = Command::new(env!("CARGO_BIN_EXE_kerf-evidence"))
        .args([
            "expand-parity-corpus",
            spec_path.to_str().expect("spec path"),
        ])
        .output()
        .expect("kerf-evidence");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).expect("versioned JSON");
    assert_eq!(value["schema_version"], "kerf.evidence-result.v1");
    assert_eq!(value["operation"], "expand-parity-corpus");
    assert_eq!(
        value["result"]["schema_version"],
        "kerf.executed-evidence-plan.v1"
    );
    assert_eq!(
        value["result"]["invocations"].as_array().map(Vec::len),
        Some(3)
    );

    let envelope_path = fixture.root.join("expanded-plan-envelope.json");
    fs::write(&envelope_path, &output.stdout).expect("write expanded plan envelope");
    let execution = Command::new(env!("CARGO_BIN_EXE_kerf-evidence"))
        .args([
            "execute",
            envelope_path.to_str().expect("envelope path"),
            "--require-accepted",
        ])
        .output()
        .expect("execute expanded envelope");
    assert!(
        execution.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&execution.stderr)
    );
    let execution_value: serde_json::Value =
        serde_json::from_slice(&execution.stdout).expect("execute result JSON");
    assert_eq!(execution_value["operation"], "execute");
    assert_eq!(execution_value["result"]["manifest"]["status"], "accepted");
}

#[test]
fn executed_manifest_binds_commands_inputs_and_negative_control() {
    let fixture = fixture();
    let manifest = execute_plan(&fixture.plan).expect("execute manifest");
    assert_eq!(manifest.status, ManifestStatus::Accepted);
    assert_eq!(manifest.invocation_receipts.len(), 2);
    assert!(
        manifest
            .invocation_receipts
            .iter()
            .all(|item| item.executed)
    );
    assert_eq!(manifest.claim_receipts[0].verdict, ClaimVerdict::Supported);
    assert!(manifest.negative_control_receipts[0].passed);
    assert_eq!(
        audit_manifest(&fixture.plan, &manifest).expect("manifest audit"),
        ManifestStatus::Accepted
    );
}

#[test]
fn changed_input_byte_rejects_binding_before_execution() {
    let fixture = fixture();
    fs::write(fixture.input_root.join("input.txt"), b"changed").expect("change input");
    let manifest = execute_plan(&fixture.plan).expect("binding result");
    assert_eq!(manifest.status, ManifestStatus::BindingRejected);
    assert_eq!(
        manifest.witness.expect("witness").kind,
        "input_topology_mismatch"
    );
}

#[test]
fn lockfile_drift_rejects_binding() {
    let fixture = fixture();
    fs::write(fixture.repository.join("Cargo.lock"), b"changed-lock").expect("change lock");
    let manifest = execute_plan(&fixture.plan).expect("binding result");
    assert_eq!(manifest.status, ManifestStatus::BindingRejected);
    assert!(manifest.witness.expect("witness").kind.contains("lock"));
}

#[test]
fn executable_substitution_rejects_binding() {
    let mut fixture = fixture();
    fixture.plan.subjects[1].executable.sha256 = "0".repeat(64);
    let manifest = execute_plan(&fixture.plan).expect("binding result");
    assert_eq!(manifest.status, ManifestStatus::BindingRejected);
    assert!(
        manifest
            .witness
            .expect("witness")
            .kind
            .contains("executable")
    );
}

#[test]
fn self_golden_cannot_promote_source_correspondence() {
    let mut fixture = fixture();
    fixture.plan.validators[0].authority = ValidatorAuthority::SelfProduced;
    fixture.plan.validators[0].authority_evidence = None;
    fixture.plan.claims[0].kind = ClaimKind::SourceCorrespondence;
    fixture.plan.claims[0].epistemic_class = EpistemicClass::EvidenceInference;
    fixture.plan.closure = ClosureKind::ObservedOnly;
    fixture.plan.claims[0].closure = ClosureKind::ObservedOnly;
    let manifest = execute_plan(&fixture.plan).expect("binding result");
    assert_eq!(manifest.status, ManifestStatus::BindingRejected);
    assert_eq!(
        manifest.witness.expect("witness").kind,
        "claim_authority_ceiling"
    );
}

#[test]
fn negative_control_matching_positive_output_fails_closed() {
    let mut fixture = fixture();
    fixture.plan.validators[1].expected = ExpectedObservation::Literal {
        value: sha256_hex(b"hello"),
    };
    fixture.plan.validators[1].authority_evidence = fixture.plan.validators[0]
        .authority_evidence
        .clone()
        .map(|mut evidence| {
            evidence.json_pointer = Some("/hello_sha256".to_owned());
            evidence
        });
    let manifest = execute_plan(&fixture.plan).expect("negative control result");
    assert_eq!(manifest.status, ManifestStatus::NegativeControlFailed);
    assert!(!manifest.negative_control_receipts[0].passed);
}

#[test]
fn accepted_manifest_cannot_be_relabelled_unexecuted() {
    let fixture = fixture();
    let mut manifest = execute_plan(&fixture.plan).expect("manifest");
    manifest.invocation_receipts[0].executed = false;
    assert_eq!(
        audit_manifest(&fixture.plan, &manifest).expect("audit"),
        ManifestStatus::BindingRejected
    );
}

#[test]
fn manifest_audit_is_bound_to_the_exact_plan() {
    let mut fixture = fixture();
    let manifest = execute_plan(&fixture.plan).expect("manifest");
    fixture
        .plan
        .nonclaims
        .push("changed after execution".to_owned());
    assert_eq!(
        audit_manifest(&fixture.plan, &manifest).expect("audit"),
        ManifestStatus::BindingRejected
    );
}

#[test]
fn removed_or_tampered_validator_receipt_is_rejected() {
    let fixture = fixture();
    let mut removed = execute_plan(&fixture.plan).expect("manifest");
    removed.validator_receipts.pop();
    assert_eq!(
        audit_manifest(&fixture.plan, &removed).expect("removed audit"),
        ManifestStatus::BindingRejected
    );

    let mut tampered = execute_plan(&fixture.plan).expect("manifest");
    tampered.validator_receipts[0].status =
        semantic_architecture::manifest::ValidatorStatus::Refuted;
    assert_eq!(
        audit_manifest(&fixture.plan, &tampered).expect("tampered audit"),
        ManifestStatus::BindingRejected
    );
}

#[test]
fn fake_reproducible_labels_cannot_establish_source_correspondence() {
    let mut fixture = fixture();
    fixture.plan.subjects[1].build_attestation = BuildAttestation::ReproducibleReceipt;
    fixture.plan.validators[0].authority = ValidatorAuthority::ReproducibleBuild;
    fixture.plan.validators[0]
        .authority_evidence
        .as_mut()
        .expect("authority evidence")
        .origin = AuthorityEvidenceOrigin::ExternalReceipt;
    fixture.plan.claims[0].kind = ClaimKind::SourceCorrespondence;
    let manifest = execute_plan(&fixture.plan).expect("binding result");
    assert_eq!(manifest.status, ManifestStatus::BindingRejected);
    assert_eq!(
        manifest.witness.expect("witness").kind,
        "claim_authority_ceiling"
    );
}

#[test]
fn self_produced_git_golden_cannot_be_relabelled_independent() {
    let mut fixture = fixture();
    let candidate = fixture.plan.subjects[1].clone();
    let evidence = fixture.plan.validators[0]
        .authority_evidence
        .as_mut()
        .expect("fixture evidence");
    evidence.subject_id = candidate.id;
    evidence.commit = candidate.git.commit;
    let error = execute_plan(&fixture.plan).expect_err("self golden must reject");
    assert!(error.message.contains("distinct earlier subject"));
}

#[test]
fn two_claims_cannot_share_one_claim_specific_control() {
    let mut fixture = fixture();
    let mut second = fixture.plan.claims[0].clone();
    second.id = "second-output-contract".to_owned();
    fixture.plan.claims.push(second);
    let error = execute_plan(&fixture.plan).expect_err("shared control must reject");
    assert!(error.message.contains("distinct negative control"));
}

#[test]
fn orphan_validator_cannot_make_an_invocation_decision_relevant() {
    let mut fixture = fixture();
    let mut invocation = fixture.plan.invocations[1].clone();
    invocation.id = "orphan-run".to_owned();
    invocation.case_id = "unearned-case".to_owned();
    fixture.plan.invocations.push(invocation);
    fixture.plan.validators.push(ValidatorPlan {
        id: "orphan-validator".to_owned(),
        kind: ValidatorKind::ExactStdout,
        authority: ValidatorAuthority::SelfProduced,
        observed_invocation_id: "orphan-run".to_owned(),
        expected: ExpectedObservation::Literal {
            value: sha256_hex(b"hello"),
        },
        authority_evidence: None,
    });
    let error = execute_plan(&fixture.plan).expect_err("orphan validator must reject");
    assert!(error.message.contains("orphan-validator"));
}

#[test]
fn structured_claim_subject_must_match_validator_reachability() {
    let mut fixture = fixture();
    fixture.plan.claims[0].subject.subject_ids = vec!["candidate".to_owned()];
    let error = execute_plan(&fixture.plan).expect_err("subject omission must reject");
    assert!(error.message.contains("executable subjects"));
}

#[test]
fn self_produced_equivalence_is_rejected() {
    let mut fixture = fixture();
    fixture.plan.validators[0].authority = ValidatorAuthority::SelfProduced;
    fixture.plan.validators[0].authority_evidence = None;
    let manifest = execute_plan(&fixture.plan).expect("binding result");
    assert_eq!(manifest.status, ManifestStatus::BindingRejected);
}

#[test]
fn second_implementation_cannot_compare_an_invocation_with_itself() {
    let mut fixture = fixture();
    fixture.plan.validators[0].authority = ValidatorAuthority::SecondImplementation;
    fixture.plan.validators[0].authority_evidence = None;
    fixture.plan.validators[0].expected = ExpectedObservation::Invocation {
        invocation_id: "candidate-run".to_owned(),
    };
    let error = execute_plan(&fixture.plan).expect_err("self comparison must reject");
    assert!(error.message.contains("distinct invocations"));
}

#[test]
fn negative_control_cannot_reuse_a_support_validator() {
    let mut fixture = fixture();
    fixture.plan.negative_controls[0].validator_id = "stdout-parity".to_owned();
    let error = execute_plan(&fixture.plan).expect_err("control overlap must reject");
    assert!(error.message.contains("distinct validator"));
}

#[test]
fn claim_cannot_exceed_plan_closure() {
    let mut fixture = fixture();
    fixture.plan.closure = ClosureKind::ObservedOnly;
    let error = execute_plan(&fixture.plan).expect_err("closure promotion must reject");
    assert!(error.message.contains("closure must equal"));
}

#[test]
fn current_checkout_cannot_leave_dirty_state_unattested() {
    let mut fixture = fixture();
    fixture.plan.subjects[1].dirty = DirtyBinding::Unattested;
    let error = execute_plan(&fixture.plan).expect_err("unattested checkout must reject");
    assert!(error.message.contains("must attest clean or dirty state"));
}

#[test]
fn tracked_and_untracked_changes_refute_a_clean_checkout_binding() {
    let tracked_fixture = fixture();
    fs::write(
        tracked_fixture.repository.join("expectations.json"),
        b"changed",
    )
    .expect("tracked change");
    let tracked = execute_plan(&tracked_fixture.plan).expect("tracked binding result");
    assert_eq!(tracked.status, ManifestStatus::BindingRejected);
    assert_eq!(
        tracked.witness.expect("tracked witness").kind,
        "clean_checkout_binding_mismatch"
    );

    let untracked_fixture = fixture();
    fs::write(untracked_fixture.repository.join("untracked.txt"), b"new")
        .expect("untracked change");
    let untracked = execute_plan(&untracked_fixture.plan).expect("untracked binding result");
    assert_eq!(untracked.status, ManifestStatus::BindingRejected);
    assert_eq!(
        untracked.witness.expect("untracked witness").kind,
        "clean_checkout_binding_mismatch"
    );
}

#[test]
fn toolchain_output_drift_rejects_binding() {
    let mut fixture = fixture();
    fixture.plan.subjects[0].toolchain.stdout_sha256 = "0".repeat(64);
    let manifest = execute_plan(&fixture.plan).expect("toolchain binding result");
    assert_eq!(manifest.status, ManifestStatus::BindingRejected);
    assert_eq!(
        manifest.witness.expect("witness").kind,
        "toolchain_identity_mismatch"
    );
}

#[test]
fn inherited_environment_is_absent() {
    let mut fixture = fixture();
    let env_program = bound("/usr/bin/env");
    for subject in &mut fixture.plan.subjects {
        subject.executable = env_program.clone();
    }
    for invocation in &mut fixture.plan.invocations {
        invocation.argv = vec![env_program.path.clone()];
    }
    fixture.plan.validators[0].expected = ExpectedObservation::Literal {
        value: sha256_hex(b""),
    };
    fixture.plan.validators[2].expected = ExpectedObservation::Literal {
        value: sha256_hex(b""),
    };
    let evidence = fixture.plan.validators[0]
        .authority_evidence
        .as_mut()
        .expect("fixture evidence");
    evidence.json_pointer = Some("/empty_sha256".to_owned());
    let manifest = execute_plan(&fixture.plan).expect("environment execution");
    assert_eq!(manifest.status, ManifestStatus::Accepted);
    assert!(
        manifest
            .invocation_receipts
            .iter()
            .all(|receipt| receipt.stdout_len == 0)
    );
}

#[test]
fn duplicate_invocation_identity_is_invalid() {
    let mut fixture = fixture();
    let duplicate = fixture.plan.invocations[0].id.clone();
    fixture.plan.invocations[1].id = duplicate;
    let error = execute_plan(&fixture.plan).expect_err("duplicate ID must reject");
    assert!(error.message.contains("duplicate ID"));
}

#[test]
fn unconsumed_invocation_cannot_inflate_finite_coverage() {
    let mut fixture = fixture();
    let mut extra = fixture.plan.invocations[0].clone();
    extra.id = "unconsumed-run".to_owned();
    extra.case_id = "unearned-case".to_owned();
    fixture.plan.invocations.push(extra);
    let error = execute_plan(&fixture.plan).expect_err("unconsumed invocation must reject");
    assert!(error.message.contains("unconsumed-run"));
}

#[test]
fn empty_directory_changes_typed_topology_digest() {
    let fixture = fixture();
    let before = snapshot_topology(&fixture.input_root).expect("before");
    fs::remove_dir(fixture.input_root.join("empty")).expect("remove empty directory");
    let after = snapshot_topology(&fixture.input_root).expect("after");
    assert_ne!(before.digest, after.digest);
    assert!(
        before.nodes.iter().any(|node| {
            node.relative_path == "empty" && node.kind == TopologyNodeKind::Directory
        })
    );
}

#[cfg(unix)]
#[test]
fn symlink_to_identical_bytes_differs_from_regular_file() {
    use std::os::unix::fs::symlink;

    let fixture = fixture();
    let file = fixture.input_root.join("input.txt");
    let bytes = fs::read(&file).expect("file bytes");
    let before = snapshot_topology(&fixture.input_root).expect("regular file topology");
    fs::remove_file(&file).expect("remove file");
    fs::write(fixture.input_root.join("target.txt"), bytes).expect("target file");
    symlink("target.txt", &file).expect("symlink");
    let after = snapshot_topology(&fixture.input_root).expect("symlink topology");
    assert_ne!(before.digest, after.digest);
    assert!(after.nodes.iter().any(|node| {
        node.relative_path == "input.txt" && node.kind == TopologyNodeKind::Symlink
    }));
}

#[cfg(unix)]
#[test]
fn dangling_symlink_is_typed_and_changes_when_target_appears() {
    use std::os::unix::fs::symlink;

    let fixture = fixture();
    let link = fixture.input_root.join("future-link");
    symlink("future-target", &link).expect("dangling symlink");
    let dangling = snapshot_topology(&fixture.input_root).expect("dangling topology");
    let dangling_node = dangling
        .nodes
        .iter()
        .find(|node| node.relative_path == "future-link")
        .expect("link node");
    assert_eq!(dangling_node.kind, TopologyNodeKind::Symlink);
    assert_eq!(dangling_node.dangling, Some(true));

    fs::write(fixture.input_root.join("future-target"), b"now present").expect("target");
    let resolved = snapshot_topology(&fixture.input_root).expect("resolved topology");
    let resolved_node = resolved
        .nodes
        .iter()
        .find(|node| node.relative_path == "future-link")
        .expect("link node");
    assert_eq!(resolved_node.dangling, Some(false));
    assert_ne!(dangling.digest, resolved.digest);
}

#[test]
fn evidence_cli_snapshots_topology_as_versioned_json() {
    let fixture = fixture();
    let output = Command::new(env!("CARGO_BIN_EXE_kerf-evidence"))
        .args([
            "snapshot-topology",
            fixture.input_root.to_str().expect("path"),
        ])
        .output()
        .expect("kerf-evidence");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).expect("versioned JSON");
    assert_eq!(value["schema_version"], "kerf.evidence-result.v1");
    assert_eq!(value["operation"], "snapshot-topology");
    assert_eq!(value["result"]["algorithm"], "kerf.fs-topology.v1");
}
