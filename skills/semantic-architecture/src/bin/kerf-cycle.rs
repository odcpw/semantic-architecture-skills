use kerf::report::{ToolError, sha256_hex};
use semantic_architecture::TOOL_VERSION;
use semantic_architecture::converge::current_evaluator_identity;
use semantic_architecture::cycle::{
    CampaignState, CampaignStatus, campaign_identities, evaluate_campaign,
};
use semantic_architecture::semantic::parse_no_duplicates;
use serde::Serialize;
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::io::{self, Write};
use std::path::Path;

const RESULT_SCHEMA: &str = "kerf.cycle-result.v1";
const ERROR_SCHEMA: &str = "kerf.cycle-error.v1";

#[derive(Serialize)]
struct ResultEnvelope<T> {
    schema_version: &'static str,
    tool_version: &'static str,
    operation: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    input_sha256: Option<String>,
    status: &'static str,
    result: T,
}

#[derive(Serialize)]
struct IdentityResult {
    search_basis_sha256: String,
    decision_state_sha256: String,
}

#[derive(Debug)]
struct ExecutionError {
    error: ToolError,
    operation: Option<&'static str>,
    input_sha256: Option<String>,
}

#[derive(Serialize)]
struct ErrorEnvelope<'a> {
    schema_version: &'static str,
    tool_version: &'static str,
    status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    operation: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    input_sha256: Option<&'a str>,
    error: &'a ToolError,
}

fn main() {
    if let Err(error) = run() {
        let envelope = ErrorEnvelope {
            schema_version: ERROR_SCHEMA,
            tool_version: TOOL_VERSION,
            status: "error",
            operation: error.operation,
            input_sha256: error.input_sha256.as_deref(),
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
            println!("kerf-cycle {TOOL_VERSION}");
            Ok(())
        }
        [flag] if flag == OsStr::new("--help") || flag == OsStr::new("-h") => {
            print_help();
            Ok(())
        }
        [command] if command == OsStr::new("identity") => write_result(
            "identity",
            None,
            current_evaluator_identity().map_err(|error| ExecutionError {
                error,
                operation: Some("identity"),
                input_sha256: None,
            })?,
        ),
        [command, path] if command == OsStr::new("hashes") => {
            let (state, input_sha256) = read_state(Path::new(path), "hashes")?;
            let (search_basis_sha256, decision_state_sha256) = campaign_identities(&state)
                .map_err(|error| ExecutionError {
                    error,
                    operation: Some("hashes"),
                    input_sha256: Some(input_sha256.clone()),
                })?;
            write_result(
                "hashes",
                Some(input_sha256),
                IdentityResult {
                    search_basis_sha256,
                    decision_state_sha256,
                },
            )
        }
        [command, path] if command == OsStr::new("evaluate") => {
            dispatch_evaluate(Path::new(path), false)
        }
        [command, path, flag]
            if command == OsStr::new("evaluate") && flag == OsStr::new("--require-terminal") =>
        {
            dispatch_evaluate(Path::new(path), true)
        }
        _ => Err(usage_error()),
    }
}

fn dispatch_evaluate(path: &Path, require_terminal: bool) -> Result<(), ExecutionError> {
    const OPERATION: &str = "evaluate";
    let (state, input_sha256) = read_state(path, OPERATION)?;
    let result = evaluate_campaign(&state).map_err(|error| ExecutionError {
        error,
        operation: Some(OPERATION),
        input_sha256: Some(input_sha256.clone()),
    })?;
    if require_terminal
        && !matches!(
            result.status,
            CampaignStatus::FrontierSet
                | CampaignStatus::EnvelopeStationary
                | CampaignStatus::BoundedExhaustiveFixedPoint
        )
    {
        return Err(ExecutionError {
            error: ToolError {
                code: "gate_unsatisfied",
                message: format!(
                    "campaign is {:?}; omit --require-terminal to inspect the result",
                    result.status
                ),
                details: Some(Box::new(serde_json::json!({
                    "campaign_status": result.status,
                    "reasons": result.reasons,
                }))),
            },
            operation: Some(OPERATION),
            input_sha256: Some(input_sha256),
        });
    }
    write_result(OPERATION, Some(input_sha256), result)
}

fn read_state(
    path: &Path,
    operation: &'static str,
) -> Result<(CampaignState, String), ExecutionError> {
    let bytes = fs::read(path).map_err(|error| ExecutionError {
        error: ToolError::io(format!("could not read {}: {error}", path.display())),
        operation: Some(operation),
        input_sha256: None,
    })?;
    let input_sha256 = sha256_hex(&bytes);
    let state = parse_no_duplicates(&bytes).map_err(|error| ExecutionError {
        error,
        operation: Some(operation),
        input_sha256: Some(input_sha256.clone()),
    })?;
    Ok((state, input_sha256))
}

fn write_result<T: Serialize>(
    operation: &'static str,
    input_sha256: Option<String>,
    result: T,
) -> Result<(), ExecutionError> {
    let envelope = ResultEnvelope {
        schema_version: RESULT_SCHEMA,
        tool_version: TOOL_VERSION,
        operation,
        input_sha256,
        status: "ok",
        result,
    };
    let mut stdout = io::stdout().lock();
    serde_json::to_writer(&mut stdout, &envelope).map_err(|error| ExecutionError {
        error: ToolError::internal(format!("could not serialize result: {error}")),
        operation: Some(operation),
        input_sha256: None,
    })?;
    writeln!(&mut stdout).map_err(|error| ExecutionError {
        error: ToolError::io(format!("could not write result: {error}")),
        operation: Some(operation),
        input_sha256: None,
    })?;
    stdout.flush().map_err(|error| ExecutionError {
        error: ToolError::io(format!("could not flush result: {error}")),
        operation: Some(operation),
        input_sha256: None,
    })
}

fn usage_error() -> ExecutionError {
    ExecutionError {
        error: ToolError::invalid(
            "usage: kerf-cycle identity | hashes CAMPAIGN.json | evaluate CAMPAIGN.json [--require-terminal]",
        ),
        operation: None,
        input_sha256: None,
    }
}

fn print_help() {
    println!(
        "kerf-cycle {TOOL_VERSION}\n\
Computed greenfield and brownfield optimization state.\n\n\
USAGE:\n  kerf-cycle identity\n  kerf-cycle hashes CAMPAIGN.json\n  kerf-cycle evaluate CAMPAIGN.json [--require-terminal]\n\n\
The evaluator derives coverage, budget exhaustion, the represented Pareto frontier, and stopping. It never accepts a caller-declared generator-exhausted or optimum boolean. Exit codes: 0=operation completed (and any requested gate passed), 2=invalid input, 3=requested terminal gate was not satisfied, 4=internal invariant, 5=I/O failure."
    );
}
