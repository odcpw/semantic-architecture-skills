use kerf::report::{ToolError, sha256_hex};
use semantic_architecture::TOOL_VERSION;
use semantic_architecture::converge::{
    ConvergenceStatus, DecisionContract, EvidencePackage, current_evaluator_identity, evaluate,
};
use semantic_architecture::manifest::{
    ExecutionPlan, ManifestStatus, ParityCorpusSpec, execute_plan, expand_parity_corpus,
    snapshot_topology,
};
use semantic_architecture::semantic::parse_no_duplicates;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

const RESULT_SCHEMA: &str = "kerf.evidence-result.v1";
const ERROR_SCHEMA: &str = "kerf.evidence-error.v1";

#[derive(Serialize)]
struct ResultEnvelope<T> {
    schema_version: &'static str,
    tool_version: &'static str,
    operation: &'static str,
    input_sha256s: BTreeMap<String, String>,
    status: &'static str,
    result: T,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ExecuteResultEnvelope {
    schema_version: String,
    tool_version: String,
    operation: String,
    input_sha256s: BTreeMap<String, String>,
    status: String,
    result: EvidencePackage,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ExpandResultEnvelope {
    schema_version: String,
    tool_version: String,
    operation: String,
    input_sha256s: BTreeMap<String, String>,
    status: String,
    result: ExecutionPlan,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum ExecutionPlanDocument {
    Plan(ExecutionPlan),
    ExpandEnvelope(ExpandResultEnvelope),
}

#[derive(Deserialize)]
#[serde(untagged)]
enum EvidencePackageDocument {
    Package(EvidencePackage),
    ExecuteEnvelope(ExecuteResultEnvelope),
}

#[derive(Debug)]
struct ExecutionError {
    error: ToolError,
    operation: Option<&'static str>,
    input_sha256s: BTreeMap<String, String>,
}

#[derive(Serialize)]
struct ErrorEnvelope<'a> {
    schema_version: &'static str,
    tool_version: &'static str,
    status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    operation: Option<&'static str>,
    input_sha256s: &'a BTreeMap<String, String>,
    error: &'a ToolError,
}

fn main() {
    if let Err(error) = run() {
        let envelope = ErrorEnvelope {
            schema_version: ERROR_SCHEMA,
            tool_version: TOOL_VERSION,
            status: "error",
            operation: error.operation,
            input_sha256s: &error.input_sha256s,
            error: &error.error,
        };
        let mut stderr = io::stderr().lock();
        serde_json::to_writer(&mut stderr, &envelope).ok();
        writeln!(&mut stderr).ok();
        std::process::exit(match error.error.code {
            "gate_unsatisfied" => 3,
            "internal_invariant" => 4,
            "io_error" => 5,
            _ => 2,
        });
    }
}

fn run() -> Result<(), ExecutionError> {
    let args: Vec<_> = env::args_os().skip(1).collect();
    match args.as_slice() {
        [flag] if flag == OsStr::new("--version") || flag == OsStr::new("-V") => {
            println!("kerf-evidence {TOOL_VERSION}");
            Ok(())
        }
        [flag] if flag == OsStr::new("--help") || flag == OsStr::new("-h") => {
            print_help();
            Ok(())
        }
        [command] if command == OsStr::new("identity") => dispatch_identity(),
        [command, root] if command == OsStr::new("snapshot-topology") => {
            dispatch_snapshot(Path::new(root))
        }
        [command, spec] if command == OsStr::new("expand-parity-corpus") => {
            dispatch_expand_parity_corpus(Path::new(spec))
        }
        [command, plan] if command == OsStr::new("execute") => {
            dispatch_execute(Path::new(plan), false)
        }
        [command, plan, flag]
            if command == OsStr::new("execute") && flag == OsStr::new("--require-accepted") =>
        {
            dispatch_execute(Path::new(plan), true)
        }
        [command, contract, packages @ ..]
            if command == OsStr::new("evaluate") && !packages.is_empty() =>
        {
            let (packages, require_fixed_point) = match packages.split_last() {
                Some((flag, preceding)) if flag == OsStr::new("--require-fixed-point") => {
                    (preceding, true)
                }
                _ => (packages, false),
            };
            if packages.is_empty() {
                return Err(usage_error());
            }
            dispatch_evaluate(
                Path::new(contract),
                &packages.iter().map(PathBuf::from).collect::<Vec<_>>(),
                require_fixed_point,
            )
        }
        _ => Err(usage_error()),
    }
}

fn usage_error() -> ExecutionError {
    ExecutionError {
        error: ToolError::invalid(
            "usage: kerf-evidence identity | snapshot-topology ROOT | expand-parity-corpus SPEC.json | execute PLAN.json [--require-accepted] | evaluate CONTRACT.json PACKAGE.json... [--require-fixed-point]",
        ),
        operation: None,
        input_sha256s: BTreeMap::new(),
    }
}

fn dispatch_identity() -> Result<(), ExecutionError> {
    const OPERATION: &str = "identity";
    let identity = current_evaluator_identity().map_err(|error| ExecutionError {
        error,
        operation: Some(OPERATION),
        input_sha256s: BTreeMap::new(),
    })?;
    write_result(OPERATION, BTreeMap::new(), identity)
}

fn dispatch_expand_parity_corpus(path: &Path) -> Result<(), ExecutionError> {
    const OPERATION: &str = "expand-parity-corpus";
    let (bytes, sha256) = read_input(path, OPERATION, BTreeMap::new())?;
    let inputs = BTreeMap::from([("spec".to_owned(), sha256)]);
    let spec: ParityCorpusSpec = parse_no_duplicates(&bytes).map_err(|error| ExecutionError {
        error,
        operation: Some(OPERATION),
        input_sha256s: inputs.clone(),
    })?;
    let plan = expand_parity_corpus(&spec).map_err(|error| ExecutionError {
        error,
        operation: Some(OPERATION),
        input_sha256s: inputs.clone(),
    })?;
    write_result(OPERATION, inputs, plan)
}

fn dispatch_snapshot(root: &Path) -> Result<(), ExecutionError> {
    const OPERATION: &str = "snapshot-topology";
    let result = snapshot_topology(root).map_err(|error| ExecutionError {
        error,
        operation: Some(OPERATION),
        input_sha256s: BTreeMap::new(),
    })?;
    write_result(OPERATION, BTreeMap::new(), result)
}

fn dispatch_execute(path: &Path, require_accepted: bool) -> Result<(), ExecutionError> {
    const OPERATION: &str = "execute";
    let (bytes, sha256) = read_input(path, OPERATION, BTreeMap::new())?;
    let inputs = BTreeMap::from([("plan".to_owned(), sha256)]);
    let plan = parse_plan(&bytes).map_err(|error| ExecutionError {
        error,
        operation: Some(OPERATION),
        input_sha256s: inputs.clone(),
    })?;
    let result = execute_plan(&plan).map_err(|error| ExecutionError {
        error,
        operation: Some(OPERATION),
        input_sha256s: inputs.clone(),
    })?;
    if require_accepted && result.status != ManifestStatus::Accepted {
        return Err(gate_unsatisfied(
            OPERATION,
            inputs,
            format!(
                "execution completed with {:?}; omit --require-accepted to inspect the result",
                result.status
            ),
            serde_json::json!({"manifest_status": result.status}),
        ));
    }
    write_result(
        OPERATION,
        inputs,
        EvidencePackage {
            plan,
            manifest: result,
        },
    )
}

fn parse_plan(bytes: &[u8]) -> Result<ExecutionPlan, ToolError> {
    match parse_no_duplicates::<ExecutionPlanDocument>(bytes)? {
        ExecutionPlanDocument::Plan(plan) => Ok(plan),
        ExecutionPlanDocument::ExpandEnvelope(envelope) => {
            if envelope.schema_version != RESULT_SCHEMA
                || envelope.operation != "expand-parity-corpus"
                || envelope.status != "ok"
                || envelope.tool_version != TOOL_VERSION
                || !envelope.input_sha256s.contains_key("spec")
            {
                return Err(ToolError::invalid(
                    "plan envelope must be a successful kerf-evidence expand-parity-corpus result from this tool version",
                ));
            }
            Ok(envelope.result)
        }
    }
}

fn dispatch_evaluate(
    contract: &Path,
    packages: &[PathBuf],
    require_fixed_point: bool,
) -> Result<(), ExecutionError> {
    const OPERATION: &str = "evaluate";
    let (contract_bytes, contract_sha256) = read_input(contract, OPERATION, BTreeMap::new())?;
    let mut inputs = BTreeMap::from([("contract".to_owned(), contract_sha256)]);
    let contract: DecisionContract =
        parse_no_duplicates(&contract_bytes).map_err(|error| ExecutionError {
            error,
            operation: Some(OPERATION),
            input_sha256s: inputs.clone(),
        })?;
    let mut parsed = Vec::new();
    for (index, path) in packages.iter().enumerate() {
        let (bytes, sha256) = read_input(path, OPERATION, inputs.clone())?;
        inputs.insert(format!("package_{index}"), sha256);
        parsed.push(parse_package(&bytes).map_err(|error| ExecutionError {
            error,
            operation: Some(OPERATION),
            input_sha256s: inputs.clone(),
        })?);
    }
    let result = evaluate(&contract, &parsed).map_err(|error| ExecutionError {
        error,
        operation: Some(OPERATION),
        input_sha256s: inputs.clone(),
    })?;
    if require_fixed_point
        && !matches!(
            result.status,
            ConvergenceStatus::DecisionFixedPoint | ConvergenceStatus::BoundedExhaustiveFixedPoint
        )
    {
        return Err(gate_unsatisfied(
            OPERATION,
            inputs,
            format!(
                "evaluation completed with {:?}; omit --require-fixed-point to inspect the result",
                result.status
            ),
            serde_json::json!({"convergence_status": result.status}),
        ));
    }
    write_result(OPERATION, inputs, result)
}

fn parse_package(bytes: &[u8]) -> Result<EvidencePackage, ToolError> {
    match parse_no_duplicates::<EvidencePackageDocument>(bytes)? {
        EvidencePackageDocument::Package(package) => Ok(package),
        EvidencePackageDocument::ExecuteEnvelope(envelope) => {
            if envelope.schema_version != RESULT_SCHEMA
                || envelope.operation != "execute"
                || envelope.status != "ok"
                || envelope.tool_version != TOOL_VERSION
                || !envelope.input_sha256s.contains_key("plan")
            {
                return Err(ToolError::invalid(
                    "package envelope must be a successful kerf-evidence execute result from this tool version",
                ));
            }
            Ok(envelope.result)
        }
    }
}

fn gate_unsatisfied(
    operation: &'static str,
    input_sha256s: BTreeMap<String, String>,
    message: String,
    details: serde_json::Value,
) -> ExecutionError {
    ExecutionError {
        error: ToolError {
            code: "gate_unsatisfied",
            message,
            details: Some(Box::new(details)),
        },
        operation: Some(operation),
        input_sha256s,
    }
}

fn read_input(
    path: &Path,
    operation: &'static str,
    inputs: BTreeMap<String, String>,
) -> Result<(Vec<u8>, String), ExecutionError> {
    let bytes = fs::read(path).map_err(|error| ExecutionError {
        error: ToolError::io(format!("could not read {}: {error}", path.display())),
        operation: Some(operation),
        input_sha256s: inputs,
    })?;
    let sha256 = sha256_hex(&bytes);
    Ok((bytes, sha256))
}

fn write_result<T: Serialize>(
    operation: &'static str,
    input_sha256s: BTreeMap<String, String>,
    result: T,
) -> Result<(), ExecutionError> {
    let envelope = ResultEnvelope {
        schema_version: RESULT_SCHEMA,
        tool_version: TOOL_VERSION,
        operation,
        input_sha256s,
        status: "ok",
        result,
    };
    let mut stdout = io::stdout().lock();
    serde_json::to_writer(&mut stdout, &envelope).map_err(|error| ExecutionError {
        error: ToolError::internal(format!("could not serialize result: {error}")),
        operation: Some(operation),
        input_sha256s: BTreeMap::new(),
    })?;
    writeln!(&mut stdout).map_err(|error| ExecutionError {
        error: ToolError::io(format!("could not write result: {error}")),
        operation: Some(operation),
        input_sha256s: BTreeMap::new(),
    })?;
    stdout.flush().map_err(|error| ExecutionError {
        error: ToolError::io(format!("could not flush result: {error}")),
        operation: Some(operation),
        input_sha256s: BTreeMap::new(),
    })
}

fn print_help() {
    println!(
        "kerf-evidence {TOOL_VERSION}\n\
Executed evidence and convergence checks.\n\n\
USAGE:\n  kerf-evidence identity\n  kerf-evidence snapshot-topology ROOT\n  kerf-evidence expand-parity-corpus SPEC.json\n  kerf-evidence execute PLAN.json [--require-accepted]\n  kerf-evidence evaluate CONTRACT.json PACKAGE.json... [--require-fixed-point]\n\n\
PLAN.json may be either a raw plan or the complete JSON envelope emitted by expand-parity-corpus.\n\
PACKAGE.json may be either the raw package or the complete JSON envelope emitted by execute.\n\
Exit codes: 0=operation completed (and any requested gate passed), 2=invalid input, 3=requested semantic gate was not satisfied, 4=internal invariant, 5=I/O failure.\n\n\
The runner clears inherited environment but does not sandbox, cap runtime/output, attest source-to-binary derivation, or replace an external semantic oracle."
    );
}
