use kerf::report::sha256_hex;
use semantic_architecture::coordinates::CoordinateModel;
use semantic_architecture::execution::{
    BoundArtifact, BoundExecutable, SQUARE_RECEIPT_SCHEMA, SquareReceipt, SquareVerificationStatus,
    verify_square,
};
use semantic_architecture::semantic::parse_no_duplicates;
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_ID: AtomicU64 = AtomicU64::new(0);

struct Fixture {
    root: PathBuf,
    coordinates: CoordinateModel,
    receipt: SquareReceipt,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn fixture() -> Fixture {
    let serial = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "semantic-architecture-execution-{}-{serial}",
        std::process::id()
    ));
    fs::create_dir_all(&root).expect("fixture directory must be created");
    let input_path = root.join("input.json");
    let left_executable_path = root.join("left-executable.bin");
    let right_executable_path = root.join("right-executable.bin");
    let left_path = root.join("left.bin");
    let right_path = root.join("right.bin");
    fs::write(&input_path, b"input").expect("input must write");
    fs::write(&left_executable_path, b"left-executable").expect("left executable must write");
    fs::write(&right_executable_path, b"right-executable").expect("right executable must write");
    fs::write(&left_path, b"equal-output").expect("left must write");
    fs::write(&right_path, b"equal-output").expect("right must write");

    let left_command = vec![
        left_executable_path.to_string_lossy().into_owned(),
        "--mode".to_owned(),
        "1".to_owned(),
    ];
    let right_command = vec![
        right_executable_path.to_string_lossy().into_owned(),
        "--mode".to_owned(),
        "1".to_owned(),
    ];
    let coordinates_value = json!({
        "schema_version": "kerf.coordinate-model.v1",
        "model_id": "execution-test",
        "facts": [
            {
                "id": "common-input",
                "semantic_kind": "configuration",
                "clock": {"effective": "run_start"},
                "authority": "fixture",
                "evidence_closure": "complete_declared_corpus"
            },
            {
                "id": "left-output",
                "semantic_kind": "artifact_bundle",
                "clock": {"effective": "run_end"},
                "authority": "fixture",
                "evidence_closure": "complete_declared_corpus"
            },
            {
                "id": "right-output",
                "semantic_kind": "artifact_bundle",
                "clock": {"effective": "run_end"},
                "authority": "fixture",
                "evidence_closure": "complete_declared_corpus"
            }
        ],
        "verification_squares": [{
            "id": "square",
            "common_inputs": ["common-input"],
            "left_path": {"command": left_command.clone(), "observation": "left-output"},
            "right_path": {"command": right_command.clone(), "observation": "right-output"},
            "comparator": "exact_bytes",
            "closure": "complete_declared_corpus",
            "falsifier": "first differing byte"
        }],
        "correspondence_claims": [{
            "id": "correspond",
            "left": "left-output",
            "right": "right-output",
            "verification_square": "square",
            "closure": "complete_declared_corpus"
        }]
    });
    let coordinates: CoordinateModel = parse_no_duplicates(
        &serde_json::to_vec(&coordinates_value).expect("coordinates must serialize"),
    )
    .expect("coordinates must parse");
    let receipt = SquareReceipt {
        schema_version: SQUARE_RECEIPT_SCHEMA.to_owned(),
        receipt_id: "receipt".to_owned(),
        square_id: "square".to_owned(),
        left_executable: executable(&left_executable_path),
        right_executable: executable(&right_executable_path),
        left_command,
        right_command,
        common_inputs: vec![artifact("input", "common-input", &input_path)],
        left_outputs: vec![artifact("result", "left-output", &left_path)],
        right_outputs: vec![artifact("result", "right-output", &right_path)],
        comparator: "exact_bytes".to_owned(),
        closure: semantic_architecture::coordinates::CoverageCeiling::CompleteDeclaredCorpus,
        nonclaims: Vec::new(),
    };
    Fixture {
        root,
        coordinates,
        receipt,
    }
}

fn artifact(id: &str, coordinate: &str, path: &Path) -> BoundArtifact {
    BoundArtifact {
        id: id.to_owned(),
        coordinate: coordinate.to_owned(),
        path: path.to_string_lossy().into_owned(),
        sha256: sha256_hex(&fs::read(path).expect("artifact must read")),
    }
}

fn executable(path: &Path) -> BoundExecutable {
    BoundExecutable {
        path: path.to_string_lossy().into_owned(),
        sha256: sha256_hex(&fs::read(path).expect("executable must read")),
    }
}

#[test]
fn exact_artifact_square_is_accepted_and_bound() {
    let fixture = fixture();
    let result = verify_square(&fixture.coordinates, &fixture.receipt).expect("square must verify");
    assert_eq!(result.status, SquareVerificationStatus::Accepted);
    assert!(result.ready_for_correspondence_claim);
    assert_eq!(result.verified_artifact_count, 5);
    assert_eq!(result.compared_output_count, 1);
    assert!(result.witness.is_none());
}

#[test]
fn command_drift_rejects_binding_before_artifact_comparison() {
    let mut fixture = fixture();
    fixture.receipt.left_command[2] = "0".to_owned();
    let result = verify_square(&fixture.coordinates, &fixture.receipt).expect("drift is a result");
    assert_eq!(result.status, SquareVerificationStatus::BindingRejected);
    assert_eq!(
        result.witness.expect("witness required").kind,
        "command_mismatch"
    );
}

#[test]
fn resolved_input_hash_drift_rejects_binding() {
    let fixture = fixture();
    fs::write(&fixture.receipt.common_inputs[0].path, b"changed-input")
        .expect("input mutation must write");
    let result = verify_square(&fixture.coordinates, &fixture.receipt).expect("drift is a result");
    assert_eq!(result.status, SquareVerificationStatus::BindingRejected);
    assert_eq!(
        result.witness.expect("witness required").kind,
        "artifact_hash_mismatch"
    );
}

#[test]
fn exact_comparator_returns_first_differing_byte() {
    let mut fixture = fixture();
    let right_path = PathBuf::from(&fixture.receipt.right_outputs[0].path);
    fs::write(&right_path, b"equal-Xutput").expect("right mutation must write");
    fixture.receipt.right_outputs[0].sha256 =
        sha256_hex(&fs::read(&right_path).expect("right must read"));
    let result = verify_square(&fixture.coordinates, &fixture.receipt).expect("mismatch is result");
    assert_eq!(result.status, SquareVerificationStatus::Refuted);
    let witness = result.witness.expect("witness required");
    assert_eq!(witness.kind, "output_byte_mismatch");
    assert_eq!(witness.byte_offset, Some(6));
}
