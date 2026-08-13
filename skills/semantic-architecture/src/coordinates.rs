//! Coordinate-labeled correspondence claims and executable verification squares.

use kerf::report::ToolError;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const COORDINATE_MODEL_SCHEMA: &str = "kerf.coordinate-model.v1";

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CoordinateModel {
    pub schema_version: String,
    pub model_id: String,
    pub facts: Vec<CoordinateFact>,
    #[serde(default)]
    pub conversions: Vec<CoordinateConversion>,
    #[serde(default)]
    pub verification_squares: Vec<VerificationSquare>,
    #[serde(default)]
    pub correspondence_claims: Vec<CorrespondenceClaim>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CoordinateFact {
    pub id: String,
    pub semantic_kind: String,
    #[serde(default)]
    pub unit: Option<String>,
    #[serde(default)]
    pub chart: Option<String>,
    pub clock: SemanticClock,
    pub authority: String,
    #[serde(default)]
    pub identity: Option<String>,
    #[serde(default)]
    pub writers: Vec<String>,
    #[serde(default)]
    pub authority_transfer: Option<String>,
    pub evidence_closure: CoverageCeiling,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SemanticClock {
    pub effective: String,
    #[serde(default)]
    pub observed_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CoordinateConversion {
    pub id: String,
    pub from: String,
    pub to: String,
    pub checker: CheckerBinding,
    pub closure: CoverageCeiling,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CheckerBinding {
    pub command: Vec<String>,
    pub artifact_sha256: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VerificationSquare {
    pub id: String,
    #[serde(default)]
    pub common_inputs: Vec<String>,
    pub left_path: ExecutablePath,
    pub right_path: ExecutablePath,
    pub comparator: String,
    pub closure: CoverageCeiling,
    pub falsifier: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutablePath {
    pub command: Vec<String>,
    pub observation: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CorrespondenceClaim {
    pub id: String,
    pub left: String,
    pub right: String,
    #[serde(default)]
    pub conversion: Option<String>,
    pub verification_square: String,
    pub closure: CoverageCeiling,
}

/// An explicitly ordered evidence ceiling for this checker. The order is part
/// of the schema: it is a claim-strength lattice fragment, not a statement that
/// all possible coverage notions are naturally comparable.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum CoverageCeiling {
    Unknown,
    ObservedOnly,
    Sampled,
    CompleteDeclaredCorpus,
    ExhaustiveFinite,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum CoordinateSeverity {
    Info,
    Warning,
    Blocker,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
pub struct CoordinateFinding {
    pub severity: CoordinateSeverity,
    pub code: String,
    pub subject: String,
    pub message: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
#[allow(clippy::struct_excessive_bools)]
pub struct CoordinateAnalysis {
    pub schema_version: &'static str,
    pub model_id: String,
    pub structurally_valid: bool,
    pub ready_for_square_execution: bool,
    pub execution_bound: bool,
    pub ready_for_correspondence_claims: bool,
    pub fact_count: usize,
    pub conversion_count: usize,
    pub verification_square_count: usize,
    pub correspondence_claim_count: usize,
    pub findings: Vec<CoordinateFinding>,
    pub nonclaims: Vec<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CoordinateSignature<'a> {
    semantic_kind: &'a str,
    unit: Option<&'a str>,
    chart: Option<&'a str>,
    clock: &'a SemanticClock,
    authority: &'a str,
    identity: Option<&'a str>,
}

/// Validate typed coordinate claims and compile unclosed comparisons into
/// findings. This checks declarations; it does not execute paths or establish
/// that their artifact hashes are authentic.
///
/// # Errors
///
/// Returns [`ToolError`] when the schema, IDs, references, commands, hashes, or
/// claim-to-square/conversion endpoints are structurally invalid.
pub fn analyze_coordinates(model: &CoordinateModel) -> Result<CoordinateAnalysis, ToolError> {
    require_equal(
        "schema_version",
        &model.schema_version,
        COORDINATE_MODEL_SCHEMA,
    )?;
    require_nonempty("model_id", &model.model_id)?;
    let facts = index_by_id("fact", &model.facts, |item| &item.id)?;
    let conversions = index_by_id("conversion", &model.conversions, |item| &item.id)?;
    let squares = index_by_id("verification square", &model.verification_squares, |item| {
        &item.id
    })?;
    let _claims = index_by_id(
        "correspondence claim",
        &model.correspondence_claims,
        |item| &item.id,
    )?;

    let mut findings = validate_facts(&model.facts)?;
    validate_conversions(&model.conversions, &facts)?;
    validate_squares(&model.verification_squares, &facts)?;
    findings.extend(analyze_correspondence_claims(
        &model.correspondence_claims,
        &facts,
        &conversions,
        &squares,
    )?);

    findings.sort();
    let structurally_ready = findings
        .iter()
        .all(|finding| finding.severity != CoordinateSeverity::Blocker);
    let has_positive_claims = !model.correspondence_claims.is_empty();
    if structurally_ready && has_positive_claims {
        findings.push(CoordinateFinding {
            severity: CoordinateSeverity::Warning,
            code: "square_execution_receipt_missing".to_owned(),
            subject: model.model_id.clone(),
            message: "the pure coordinate checker has no executed, artifact-bound square receipt; use kerf-pipeline verify-square before promoting correspondence".to_owned(),
        });
        findings.sort();
    }
    Ok(CoordinateAnalysis {
        schema_version: "kerf.coordinate-analysis.v1",
        model_id: model.model_id.clone(),
        structurally_valid: true,
        ready_for_square_execution: structurally_ready,
        execution_bound: !has_positive_claims,
        ready_for_correspondence_claims: structurally_ready && !has_positive_claims,
        fact_count: model.facts.len(),
        conversion_count: model.conversions.len(),
        verification_square_count: model.verification_squares.len(),
        correspondence_claim_count: model.correspondence_claims.len(),
        findings,
        nonclaims: vec![
            "The checker validates declared coordinates, references, ceilings, and executable-path manifests; it does not execute either path.",
            "A checker artifact hash is an identity binding, not a signature, authenticity proof, or semantic proof.",
            "A clean result establishes internal closure of the supplied coordinate model, not source completeness or real-system correspondence.",
            "Positive correspondence requires a separate artifact-bound square receipt from kerf-pipeline verify-square.",
        ],
    })
}

fn validate_facts(facts: &[CoordinateFact]) -> Result<Vec<CoordinateFinding>, ToolError> {
    let mut findings = Vec::new();
    for fact in facts {
        require_nonempty("fact.semantic_kind", &fact.semantic_kind)?;
        require_optional_nonempty("fact.unit", fact.unit.as_deref())?;
        require_optional_nonempty("fact.chart", fact.chart.as_deref())?;
        require_nonempty("fact.clock.effective", &fact.clock.effective)?;
        require_optional_nonempty("fact.clock.observed_at", fact.clock.observed_at.as_deref())?;
        require_nonempty("fact.authority", &fact.authority)?;
        require_optional_nonempty("fact.identity", fact.identity.as_deref())?;
        require_unique_nonempty("fact.writers", &fact.writers)?;
        require_optional_nonempty(
            "fact.authority_transfer",
            fact.authority_transfer.as_deref(),
        )?;
        if fact.writers.len() > 1 && fact.authority_transfer.is_none() {
            findings.push(finding(
                "multiple_writers_without_authority_transfer",
                &fact.id,
                "multiple writers are presented as one authority without an authority-transfer protocol",
            ));
        }
    }
    Ok(findings)
}

fn validate_conversions(
    conversions: &[CoordinateConversion],
    facts: &BTreeMap<&str, &CoordinateFact>,
) -> Result<(), ToolError> {
    for conversion in conversions {
        require_reference("conversion.from", &conversion.from, facts)?;
        require_reference("conversion.to", &conversion.to, facts)?;
        require_command("conversion.checker.command", &conversion.checker.command)?;
        require_sha256(
            "conversion.checker.artifact_sha256",
            &conversion.checker.artifact_sha256,
        )?;
    }
    Ok(())
}

fn validate_squares(
    squares: &[VerificationSquare],
    facts: &BTreeMap<&str, &CoordinateFact>,
) -> Result<(), ToolError> {
    for square in squares {
        require_unique_nonempty("verification_square.common_inputs", &square.common_inputs)?;
        for input in &square.common_inputs {
            require_reference("verification_square.common_inputs", input, facts)?;
        }
        require_command(
            "verification_square.left_path.command",
            &square.left_path.command,
        )?;
        require_command(
            "verification_square.right_path.command",
            &square.right_path.command,
        )?;
        require_reference(
            "verification_square.left_path.observation",
            &square.left_path.observation,
            facts,
        )?;
        require_reference(
            "verification_square.right_path.observation",
            &square.right_path.observation,
            facts,
        )?;
        require_nonempty("verification_square.comparator", &square.comparator)?;
        require_nonempty("verification_square.falsifier", &square.falsifier)?;
    }
    Ok(())
}

fn analyze_correspondence_claims(
    claims: &[CorrespondenceClaim],
    facts: &BTreeMap<&str, &CoordinateFact>,
    conversions: &BTreeMap<&str, &CoordinateConversion>,
    squares: &BTreeMap<&str, &VerificationSquare>,
) -> Result<Vec<CoordinateFinding>, ToolError> {
    let mut findings = Vec::new();
    for claim in claims {
        let left = require_reference("claim.left", &claim.left, facts)?;
        let right = require_reference("claim.right", &claim.right, facts)?;
        let square = require_reference(
            "claim.verification_square",
            &claim.verification_square,
            squares,
        )?;
        validate_claim_square(claim, square)?;
        let conversion = resolve_claim_conversion(claim, conversions)?;
        // ubs:ignore — coordinate signatures contain public model metadata, not secrets.
        if signature(left) != signature(right) && conversion.is_none() {
            findings.push(finding(
                "coordinate_conversion_missing",
                &claim.id,
                "correspondence compares different semantic kind/unit/chart/clock/authority/identity coordinates without a named checked conversion",
            ));
        }
        let mut ceiling = left.evidence_closure.min(right.evidence_closure);
        ceiling = ceiling.min(square.closure);
        if let Some(conversion) = conversion {
            ceiling = ceiling.min(conversion.closure);
        }
        if claim.closure > ceiling {
            findings.push(finding(
                "closure_exceeds_evidence",
                &claim.id,
                &format!(
                    "claim closure {:?} exceeds weakest declared input/conversion/square ceiling {:?}",
                    claim.closure, ceiling
                ),
            ));
        }
    }
    Ok(findings)
}

fn validate_claim_square(
    claim: &CorrespondenceClaim,
    square: &VerificationSquare,
) -> Result<(), ToolError> {
    // ubs:ignore — observation IDs are public schema identifiers, not secrets.
    if square.left_path.observation == claim.left && square.right_path.observation == claim.right {
        Ok(())
    } else {
        Err(ToolError::invalid(format!(
            "claim {:?} endpoints do not match verification square {:?} observations",
            claim.id, square.id
        )))
    }
}

fn resolve_claim_conversion<'a>(
    claim: &CorrespondenceClaim,
    conversions: &'a BTreeMap<&str, &CoordinateConversion>,
) -> Result<Option<&'a CoordinateConversion>, ToolError> {
    let Some(conversion_id) = &claim.conversion else {
        return Ok(None);
    };
    let conversion = require_reference("claim.conversion", conversion_id, conversions)?;
    if conversion.from != claim.left || conversion.to != claim.right {
        return Err(ToolError::invalid(format!(
            "claim {:?} conversion {:?} maps {:?} -> {:?}, not {:?} -> {:?}",
            claim.id, conversion.id, conversion.from, conversion.to, claim.left, claim.right
        )));
    }
    Ok(Some(conversion))
}

fn signature(fact: &CoordinateFact) -> CoordinateSignature<'_> {
    CoordinateSignature {
        semantic_kind: &fact.semantic_kind,
        unit: fact.unit.as_deref(),
        chart: fact.chart.as_deref(),
        clock: &fact.clock,
        authority: &fact.authority,
        identity: fact.identity.as_deref(),
    }
}

fn finding(code: &str, subject: &str, message: &str) -> CoordinateFinding {
    CoordinateFinding {
        severity: CoordinateSeverity::Blocker,
        code: code.to_owned(),
        subject: subject.to_owned(),
        message: message.to_owned(),
    }
}

fn index_by_id<'a, T>(
    kind: &str,
    items: &'a [T],
    id: impl Fn(&'a T) -> &'a str,
) -> Result<BTreeMap<&'a str, &'a T>, ToolError> {
    let mut indexed = BTreeMap::new();
    for item in items {
        let item_id = id(item);
        require_nonempty(&format!("{kind}.id"), item_id)?;
        if indexed.insert(item_id, item).is_some() {
            return Err(ToolError::invalid(format!(
                "duplicate {kind} ID {item_id:?}"
            )));
        }
    }
    Ok(indexed)
}

fn require_reference<'a, T>(
    field: &str,
    id: &str,
    items: &'a BTreeMap<&str, &'a T>,
) -> Result<&'a T, ToolError> {
    items
        .get(id)
        .copied()
        .ok_or_else(|| ToolError::invalid(format!("{field} references unknown ID {id:?}")))
}

fn require_equal(field: &str, actual: &str, expected: &str) -> Result<(), ToolError> {
    if actual == expected {
        Ok(())
    } else {
        Err(ToolError::invalid(format!(
            "{field} must equal {expected:?}, found {actual:?}"
        )))
    }
}

fn require_nonempty(field: &str, value: &str) -> Result<(), ToolError> {
    if value.trim().is_empty() {
        Err(ToolError::invalid(format!("{field} must be non-empty")))
    } else {
        Ok(())
    }
}

fn require_optional_nonempty(field: &str, value: Option<&str>) -> Result<(), ToolError> {
    if let Some(value) = value {
        require_nonempty(field, value)?;
    }
    Ok(())
}

fn require_unique_nonempty(field: &str, values: &[String]) -> Result<(), ToolError> {
    let mut seen = BTreeSet::new();
    for value in values {
        require_nonempty(field, value)?;
        if !seen.insert(value) {
            return Err(ToolError::invalid(format!(
                "{field} contains duplicate value {value:?}"
            )));
        }
    }
    Ok(())
}

fn require_command(field: &str, command: &[String]) -> Result<(), ToolError> {
    if command.is_empty() {
        return Err(ToolError::invalid(format!(
            "{field} must contain an executable and optional arguments"
        )));
    }
    for value in command {
        require_nonempty(field, value)?;
    }
    Ok(())
}

fn require_sha256(field: &str, value: &str) -> Result<(), ToolError> {
    if value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err(ToolError::invalid(format!(
            "{field} must be a 64-character hexadecimal SHA-256"
        )))
    }
}
