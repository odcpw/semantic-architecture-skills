use kerf::report::{ToolError, sha256_hex};
use semantic_architecture::TOOL_VERSION;
use semantic_architecture::generate::{TransformationBatch, TransformationRequest};
use semantic_architecture::metabolize::{MetabolismPlan, execute_metabolism};
use semantic_architecture::semantic::parse_no_duplicates;
use serde::{Serialize, de::DeserializeOwned};
use std::collections::BTreeMap;
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::io::{self, Write};
use std::path::Path;

const RESULT_SCHEMA: &str = "kerf.metabolize-result.v1";
const ERROR_SCHEMA: &str = "kerf.metabolize-error.v1";

#[derive(Serialize)]
struct ResultEnvelope<T> {
    schema_version: &'static str,
    tool_version: &'static str,
    operation: &'static str,
    input_sha256s: BTreeMap<String, String>,
    status: &'static str,
    result: T,
}

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
            println!("kerf-metabolize {TOOL_VERSION}");
            Ok(())
        }
        [flag] if flag == OsStr::new("--help") || flag == OsStr::new("-h") => {
            print_help();
            Ok(())
        }
        [command, request, batch, plan] if command == OsStr::new("execute") => {
            dispatch(Path::new(request), Path::new(batch), Path::new(plan), false)
        }
        [command, request, batch, plan, flag]
            if command == OsStr::new("execute") && flag == OsStr::new("--require-accepted") =>
        {
            dispatch(Path::new(request), Path::new(batch), Path::new(plan), true)
        }
        _ => Err(usage_error()),
    }
}

fn dispatch(
    request_path: &Path,
    batch_path: &Path,
    plan_path: &Path,
    require_accepted: bool,
) -> Result<(), ExecutionError> {
    const OPERATION: &str = "execute";
    let (request, mut inputs) = read::<TransformationRequest>(request_path, "request", OPERATION)?;
    let (batch, batch_inputs) = read::<TransformationBatch>(batch_path, "batch", OPERATION)?;
    let (plan, plan_inputs) = read::<MetabolismPlan>(plan_path, "plan", OPERATION)?;
    inputs.extend(batch_inputs);
    inputs.extend(plan_inputs);
    let receipt = execute_metabolism(&request, &batch, &plan).map_err(|error| ExecutionError {
        error,
        operation: Some(OPERATION),
        input_sha256s: inputs.clone(),
    })?;
    if require_accepted && !receipt.accepted {
        return Err(ExecutionError {
            error: ToolError {
                code: "gate_unsatisfied",
                message: "candidate was rejected by the metabolism gate".to_owned(),
                details: Some(Box::new(serde_json::json!({
                    "status": receipt.status,
                    "counterexamples": receipt.counterexamples,
                }))),
            },
            operation: Some(OPERATION),
            input_sha256s: inputs,
        });
    }
    let envelope = ResultEnvelope {
        schema_version: RESULT_SCHEMA,
        tool_version: TOOL_VERSION,
        operation: OPERATION,
        input_sha256s: inputs,
        status: "ok",
        result: receipt,
    };
    let mut stdout = io::stdout().lock();
    serde_json::to_writer(&mut stdout, &envelope).map_err(|error| ExecutionError {
        error: ToolError::internal(format!("could not serialize result: {error}")),
        operation: Some(OPERATION),
        input_sha256s: BTreeMap::new(),
    })?;
    writeln!(&mut stdout).map_err(|error| ExecutionError {
        error: ToolError::io(format!("could not write result: {error}")),
        operation: Some(OPERATION),
        input_sha256s: BTreeMap::new(),
    })?;
    Ok(())
}

fn read<T: DeserializeOwned>(
    path: &Path,
    label: &str,
    operation: &'static str,
) -> Result<(T, BTreeMap<String, String>), ExecutionError> {
    let bytes = fs::read(path).map_err(|error| ExecutionError {
        error: ToolError::io(format!("could not read {}: {error}", path.display())),
        operation: Some(operation),
        input_sha256s: BTreeMap::new(),
    })?;
    let inputs = BTreeMap::from([(label.to_owned(), sha256_hex(&bytes))]);
    let value = parse_no_duplicates(&bytes).map_err(|error| ExecutionError {
        error,
        operation: Some(operation),
        input_sha256s: inputs.clone(),
    })?;
    Ok((value, inputs))
}

fn usage_error() -> ExecutionError {
    ExecutionError {
        error: ToolError::invalid(
            "usage: kerf-metabolize execute REQUEST.json BATCH.json PLAN.json [--require-accepted]",
        ),
        operation: None,
        input_sha256s: BTreeMap::new(),
    }
}

fn print_help() {
    println!(
        "kerf-metabolize {TOOL_VERSION}\n\
Apply, verify, measure, and rebuild one generated transformation in isolated copies.\n\n\
USAGE:\n  kerf-metabolize execute REQUEST.json BATCH.json PLAN.json [--require-accepted]\n\n\
The source repository is never edited. The workspace root must be new and outside the source repository. Acceptance requires changed source bytes, complete protected-observation and hard-constraint coverage, passing negative controls, improvement on at least one declared metric, no forbidden regression, and a structurally valid rebuilt model naming the new content revision."
    );
}
