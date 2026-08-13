//! Verification of artifact-bound correspondence-square receipts.

use crate::coordinates::{CoordinateModel, CoverageCeiling, analyze_coordinates};
use kerf::report::{ToolError, sha256_hex};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

pub const SQUARE_RECEIPT_SCHEMA: &str = "kerf.execution-square-receipt.v1";

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SquareReceipt {
    pub schema_version: String,
    pub receipt_id: String,
    pub square_id: String,
    pub left_executable: BoundExecutable,
    pub right_executable: BoundExecutable,
    pub left_command: Vec<String>,
    pub right_command: Vec<String>,
    pub common_inputs: Vec<BoundArtifact>,
    pub left_outputs: Vec<BoundArtifact>,
    pub right_outputs: Vec<BoundArtifact>,
    pub comparator: String,
    pub closure: CoverageCeiling,
    #[serde(default)]
    pub nonclaims: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BoundArtifact {
    pub id: String,
    pub coordinate: String,
    pub path: String,
    pub sha256: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BoundExecutable {
    pub path: String,
    pub sha256: String,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SquareVerificationStatus {
    Accepted,
    Refuted,
    BindingRejected,
}

#[derive(Debug, Clone, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SquareVerification {
    pub schema_version: &'static str,
    pub receipt_id: String,
    pub receipt_sha256: String,
    pub square_id: String,
    pub status: SquareVerificationStatus,
    pub ready_for_correspondence_claim: bool,
    pub closure: CoverageCeiling,
    pub verified_artifact_count: usize,
    pub compared_output_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub witness: Option<SquareWitness>,
    pub nonclaims: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SquareWitness {
    pub kind: &'static str,
    pub artifact_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_sha256: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub observed_sha256: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub byte_offset: Option<u64>,
}

/// Verify that a declared square receipt matches the coordinate declaration,
/// that every bound artifact still has its declared bytes, and that all named
/// output pairs are byte-identical.
///
/// This verifies already-produced artifacts. It does not prove that the
/// declared commands caused them, authenticate the machine, or attest a build.
///
/// # Errors
///
/// Returns [`ToolError`] when either schema is invalid, a reference or artifact
/// binding is malformed, or an artifact cannot be read. Semantic mismatches are
/// completed results with a witness.
#[allow(clippy::too_many_lines)]
pub fn verify_square(
    coordinates: &CoordinateModel,
    receipt: &SquareReceipt,
) -> Result<SquareVerification, ToolError> {
    let coordinate_analysis = analyze_coordinates(coordinates)?;
    if !coordinate_analysis.ready_for_square_execution {
        return Err(ToolError::invalid(
            "coordinate model is not structurally ready for square execution",
        ));
    }
    validate_receipt(receipt)?;
    let square = coordinates
        .verification_squares
        .iter()
        .find(|item| item.id == receipt.square_id)
        .ok_or_else(|| {
            ToolError::invalid(format!(
                "receipt references unknown verification square {:?}",
                receipt.square_id
            ))
        })?;
    if receipt.left_command.first() != Some(&receipt.left_executable.path)
        || receipt.right_command.first() != Some(&receipt.right_executable.path)
    {
        return result(
            receipt,
            SquareVerificationStatus::BindingRejected,
            0,
            0,
            Some(SquareWitness {
                kind: "executable_command_mismatch",
                artifact_id: receipt.square_id.clone(),
                path: None,
                expected_sha256: None,
                observed_sha256: None,
                byte_offset: None,
            }),
        );
    }
    if receipt.left_command != square.left_path.command
        || receipt.right_command != square.right_path.command
    {
        return result(
            receipt,
            SquareVerificationStatus::BindingRejected,
            0,
            0,
            Some(SquareWitness {
                kind: "command_mismatch",
                artifact_id: receipt.square_id.clone(),
                path: None,
                expected_sha256: None,
                observed_sha256: None,
                byte_offset: None,
            }),
        );
    }
    if receipt.comparator != square.comparator || receipt.closure != square.closure {
        return result(
            receipt,
            SquareVerificationStatus::BindingRejected,
            0,
            0,
            Some(SquareWitness {
                kind: "comparison_contract_mismatch",
                artifact_id: receipt.square_id.clone(),
                path: None,
                expected_sha256: None,
                observed_sha256: None,
                byte_offset: None,
            }),
        );
    }
    let input_coordinates: BTreeSet<_> = receipt
        .common_inputs
        .iter()
        .map(|item| item.coordinate.as_str())
        .collect();
    let expected_inputs: BTreeSet<_> = square.common_inputs.iter().map(String::as_str).collect();
    if input_coordinates != expected_inputs {
        return result(
            receipt,
            SquareVerificationStatus::BindingRejected,
            0,
            0,
            Some(SquareWitness {
                kind: "common_input_set_mismatch",
                artifact_id: receipt.square_id.clone(),
                path: None,
                expected_sha256: None,
                observed_sha256: None,
                byte_offset: None,
            }),
        );
    }
    let left_coordinates: BTreeSet<_> = receipt
        .left_outputs
        .iter()
        .map(|item| item.coordinate.as_str())
        .collect();
    let right_coordinates: BTreeSet<_> = receipt
        .right_outputs
        .iter()
        .map(|item| item.coordinate.as_str())
        .collect();
    if left_coordinates != BTreeSet::from([square.left_path.observation.as_str()])
        || right_coordinates != BTreeSet::from([square.right_path.observation.as_str()])
    {
        return result(
            receipt,
            SquareVerificationStatus::BindingRejected,
            0,
            0,
            Some(SquareWitness {
                kind: "output_coordinate_mismatch",
                artifact_id: receipt.square_id.clone(),
                path: None,
                expected_sha256: None,
                observed_sha256: None,
                byte_offset: None,
            }),
        );
    }

    let mut verified = 0;
    for executable in [&receipt.left_executable, &receipt.right_executable] {
        let observed = sha256_file(Path::new(&executable.path))?;
        if observed != executable.sha256 {
            return result(
                receipt,
                SquareVerificationStatus::BindingRejected,
                verified,
                0,
                Some(SquareWitness {
                    kind: "executable_hash_mismatch",
                    artifact_id: receipt.square_id.clone(),
                    path: Some(executable.path.clone()),
                    expected_sha256: Some(executable.sha256.clone()),
                    observed_sha256: Some(observed),
                    byte_offset: None,
                }),
            );
        }
        verified += 1;
    }
    for artifact in receipt
        .common_inputs
        .iter()
        .chain(&receipt.left_outputs)
        .chain(&receipt.right_outputs)
    {
        let observed = sha256_file(Path::new(&artifact.path))?;
        if observed != artifact.sha256 {
            return result(
                receipt,
                SquareVerificationStatus::BindingRejected,
                verified,
                0,
                Some(SquareWitness {
                    kind: "artifact_hash_mismatch",
                    artifact_id: artifact.id.clone(),
                    path: Some(artifact.path.clone()),
                    expected_sha256: Some(artifact.sha256.clone()),
                    observed_sha256: Some(observed),
                    byte_offset: None,
                }),
            );
        }
        verified += 1;
    }

    let left = index_outputs("left_outputs", &receipt.left_outputs)?;
    let right = index_outputs("right_outputs", &receipt.right_outputs)?;
    if left.keys().ne(right.keys()) {
        return result(
            receipt,
            SquareVerificationStatus::BindingRejected,
            verified,
            0,
            Some(SquareWitness {
                kind: "output_set_mismatch",
                artifact_id: receipt.square_id.clone(),
                path: None,
                expected_sha256: None,
                observed_sha256: None,
                byte_offset: None,
            }),
        );
    }
    let mut compared = 0;
    for (id, left_artifact) in left {
        let right_artifact = right[id];
        if let Some(offset) = first_difference(
            Path::new(&left_artifact.path),
            Path::new(&right_artifact.path),
        )? {
            return result(
                receipt,
                SquareVerificationStatus::Refuted,
                verified,
                compared,
                Some(SquareWitness {
                    kind: "output_byte_mismatch",
                    artifact_id: (*id).to_owned(),
                    path: Some(format!("{} != {}", left_artifact.path, right_artifact.path)),
                    expected_sha256: Some(left_artifact.sha256.clone()),
                    observed_sha256: Some(right_artifact.sha256.clone()),
                    byte_offset: Some(offset),
                }),
            );
        }
        compared += 1;
    }
    result(
        receipt,
        SquareVerificationStatus::Accepted,
        verified,
        compared,
        None,
    )
}

fn validate_receipt(receipt: &SquareReceipt) -> Result<(), ToolError> {
    if receipt.schema_version != SQUARE_RECEIPT_SCHEMA {
        return Err(ToolError::invalid(format!(
            "unsupported schema_version {:?}; expected {SQUARE_RECEIPT_SCHEMA:?}",
            receipt.schema_version
        )));
    }
    for (field, value) in [
        ("receipt_id", receipt.receipt_id.as_str()),
        ("square_id", receipt.square_id.as_str()),
        ("comparator", receipt.comparator.as_str()),
    ] {
        if value.is_empty() {
            return Err(ToolError::invalid(format!("{field} must not be empty")));
        }
    }
    if receipt.left_command.is_empty() || receipt.right_command.is_empty() {
        return Err(ToolError::invalid(
            "left_command and right_command must not be empty",
        ));
    }
    for (field, executable) in [
        ("left_executable", &receipt.left_executable),
        ("right_executable", &receipt.right_executable),
    ] {
        let path = Path::new(&executable.path);
        if !path.is_absolute() || !path.is_file() {
            return Err(ToolError::invalid(format!(
                "{field}.path {:?} must be an existing absolute file",
                executable.path
            )));
        }
        require_sha256(&format!("{field}.sha256"), &executable.sha256)?;
    }
    if receipt.common_inputs.is_empty() {
        return Err(ToolError::invalid("common_inputs must not be empty"));
    }
    if receipt.left_outputs.is_empty() || receipt.right_outputs.is_empty() {
        return Err(ToolError::invalid(
            "left_outputs and right_outputs must not be empty",
        ));
    }
    let mut ids = BTreeSet::new();
    for artifact in receipt
        .common_inputs
        .iter()
        .chain(&receipt.left_outputs)
        .chain(&receipt.right_outputs)
    {
        if artifact.id.is_empty() || artifact.coordinate.is_empty() {
            return Err(ToolError::invalid(
                "artifact ID and coordinate must not be empty",
            ));
        }
        let path = Path::new(&artifact.path);
        if !path.is_absolute() || !path.is_file() {
            return Err(ToolError::invalid(format!(
                "artifact path {:?} must be an existing absolute file",
                artifact.path
            )));
        }
        require_sha256("artifact.sha256", &artifact.sha256)?;
        let identity = format!("{}:{}", artifact.coordinate, artifact.id);
        if !ids.insert(identity.clone()) {
            return Err(ToolError::invalid(format!(
                "duplicate artifact identity {identity:?}"
            )));
        }
    }
    Ok(())
}

fn index_outputs<'a>(
    field: &str,
    artifacts: &'a [BoundArtifact],
) -> Result<BTreeMap<&'a str, &'a BoundArtifact>, ToolError> {
    let mut index = BTreeMap::new();
    for artifact in artifacts {
        if index.insert(artifact.id.as_str(), artifact).is_some() {
            return Err(ToolError::invalid(format!(
                "{field} contains duplicate ID {:?}",
                artifact.id
            )));
        }
    }
    Ok(index)
}

fn result(
    receipt: &SquareReceipt,
    status: SquareVerificationStatus,
    verified_artifact_count: usize,
    compared_output_count: usize,
    witness: Option<SquareWitness>,
) -> Result<SquareVerification, ToolError> {
    let bytes = serde_json::to_vec(receipt).map_err(|error| {
        ToolError::internal(format!("could not serialize square receipt: {error}"))
    })?;
    Ok(SquareVerification {
        schema_version: "kerf.execution-square-verification.v1",
        receipt_id: receipt.receipt_id.clone(),
        receipt_sha256: sha256_hex(&bytes),
        square_id: receipt.square_id.clone(),
        status,
        ready_for_correspondence_claim: status == SquareVerificationStatus::Accepted,
        closure: receipt.closure,
        verified_artifact_count,
        compared_output_count,
        witness,
        nonclaims: vec![
            "Acceptance proves current byte identity only for the declared artifacts and common-input bindings.",
            "This verifier does not execute commands, prove that commands caused artifacts, attest a build, authenticate a machine, or establish source completeness.",
            "The result cannot exceed the receipt's declared closure or the weakest coordinate evidence ceiling.",
        ],
    })
}

fn sha256_file(path: &Path) -> Result<String, ToolError> {
    let file = File::open(path)
        .map_err(|error| ToolError::io(format!("could not open {}: {error}", path.display())))?;
    let mut reader = BufReader::new(file);
    let mut hasher = Sha256::new();
    let mut buffer = vec![0_u8; 1024 * 1024];
    loop {
        let read = reader.read(&mut buffer).map_err(|error| {
            ToolError::io(format!("could not read {}: {error}", path.display()))
        })?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn first_difference(left: &Path, right: &Path) -> Result<Option<u64>, ToolError> {
    let left_file = File::open(left)
        .map_err(|error| ToolError::io(format!("could not open {}: {error}", left.display())))?;
    let right_file = File::open(right)
        .map_err(|error| ToolError::io(format!("could not open {}: {error}", right.display())))?;
    let mut left_reader = BufReader::new(left_file);
    let mut right_reader = BufReader::new(right_file);
    let mut left_buffer = vec![0_u8; 64 * 1024];
    let mut right_buffer = vec![0_u8; 64 * 1024];
    let mut offset = 0_u64;
    loop {
        let left_read = left_reader.read(&mut left_buffer).map_err(|error| {
            ToolError::io(format!("could not read {}: {error}", left.display()))
        })?;
        let right_read = right_reader.read(&mut right_buffer).map_err(|error| {
            ToolError::io(format!("could not read {}: {error}", right.display()))
        })?;
        let common = left_read.min(right_read);
        if let Some(index) = (0..common).find(|index| left_buffer[*index] != right_buffer[*index]) {
            return Ok(Some(offset + index as u64));
        }
        if left_read != right_read {
            return Ok(Some(offset + common as u64));
        }
        if left_read == 0 {
            return Ok(None);
        }
        offset += left_read as u64;
    }
}

fn require_sha256(field: &str, value: &str) -> Result<(), ToolError> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(ToolError::invalid(format!(
            "{field} must be 64 lowercase hexadecimal characters"
        )));
    }
    Ok(())
}
