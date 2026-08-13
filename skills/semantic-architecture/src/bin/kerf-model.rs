use kerf::report::{ToolError, sha256_hex};
use semantic_architecture::TOOL_VERSION;
use semantic_architecture::coordinates::{CoordinateModel, analyze_coordinates};
use semantic_architecture::semantic::{
    SemanticModel, analyze, derive_work, parse_no_duplicates, project,
};
use serde::Serialize;
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::io::{self, Write};
use std::path::Path;

const RESULT_SCHEMA: &str = "kerf.model-result.v1";
const ERROR_SCHEMA: &str = "kerf.model-error.v1";

#[derive(Serialize)]
struct ResultEnvelope<T> {
    schema_version: &'static str,
    tool_version: &'static str,
    operation: &'static str,
    input_sha256: String,
    status: &'static str,
    result: T,
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
        std::process::exit(if error.error.code == "internal_invariant" {
            4
        } else {
            2
        });
    }
}

fn run() -> Result<(), ExecutionError> {
    let args: Vec<_> = env::args_os().skip(1).collect();
    match args.as_slice() {
        [flag] if flag == OsStr::new("--version") || flag == OsStr::new("-V") => {
            println!("kerf-model {TOOL_VERSION}");
            Ok(())
        }
        [flag] if flag == OsStr::new("--help") || flag == OsStr::new("-h") => {
            print_help();
            Ok(())
        }
        [command, path] if command == OsStr::new("analyze") => {
            dispatch("analyze", Path::new(path), analyze)
        }
        [command, path] if command == OsStr::new("derive-work") => {
            dispatch("derive-work", Path::new(path), derive_work)
        }
        [command, path] if command == OsStr::new("check-coordinates") => {
            dispatch_coordinates(Path::new(path))
        }
        [command, path, projection_id] if command == OsStr::new("project") => {
            let projection_id = projection_id.to_str().ok_or_else(|| ExecutionError {
                error: ToolError::invalid("projection ID must be valid UTF-8"),
                operation: Some("project"),
                input_sha256: None,
            })?;
            dispatch("project", Path::new(path), |model| {
                project(model, projection_id)
            })
        }
        _ => Err(ExecutionError {
            error: ToolError::invalid(
                "usage: kerf-model <analyze|derive-work> MODEL.json | kerf-model check-coordinates COORDINATES.json | kerf-model project MODEL.json PROJECTION_ID",
            ),
            operation: None,
            input_sha256: None,
        }),
    }
}

fn dispatch_coordinates(path: &Path) -> Result<(), ExecutionError> {
    const OPERATION: &str = "check-coordinates";
    let bytes = fs::read(path).map_err(|error| ExecutionError {
        error: ToolError::io(format!("could not read {}: {error}", path.display())),
        operation: Some(OPERATION),
        input_sha256: None,
    })?;
    let input_sha256 = sha256_hex(&bytes);
    let model: CoordinateModel = parse_no_duplicates(&bytes).map_err(|error| ExecutionError {
        error,
        operation: Some(OPERATION),
        input_sha256: Some(input_sha256.clone()),
    })?;
    let result = analyze_coordinates(&model).map_err(|error| ExecutionError {
        error,
        operation: Some(OPERATION),
        input_sha256: Some(input_sha256.clone()),
    })?;
    emit(OPERATION, input_sha256.clone(), result).map_err(|error| ExecutionError {
        error,
        operation: Some(OPERATION),
        input_sha256: Some(input_sha256),
    })
}

fn dispatch<T: Serialize>(
    operation: &'static str,
    path: &Path,
    execute: impl FnOnce(&SemanticModel) -> Result<T, ToolError>,
) -> Result<(), ExecutionError> {
    let bytes = fs::read(path).map_err(|error| ExecutionError {
        error: ToolError::io(format!("could not read {}: {error}", path.display())),
        operation: Some(operation),
        input_sha256: None,
    })?;
    let input_sha256 = sha256_hex(&bytes);
    let model: SemanticModel = parse_no_duplicates(&bytes).map_err(|error| ExecutionError {
        error,
        operation: Some(operation),
        input_sha256: Some(input_sha256.clone()),
    })?;
    let result = execute(&model).map_err(|error| ExecutionError {
        error,
        operation: Some(operation),
        input_sha256: Some(input_sha256.clone()),
    })?;
    emit(operation, input_sha256.clone(), result).map_err(|error| ExecutionError {
        error,
        operation: Some(operation),
        input_sha256: Some(input_sha256),
    })
}

fn emit<T: Serialize>(
    operation: &'static str,
    input_sha256: String,
    result: T,
) -> Result<(), ToolError> {
    let envelope = ResultEnvelope {
        schema_version: RESULT_SCHEMA,
        tool_version: TOOL_VERSION,
        operation,
        input_sha256,
        status: "ok",
        result,
    };
    let mut stdout = io::stdout().lock();
    serde_json::to_writer(&mut stdout, &envelope).map_err(|error| {
        if error.is_io() {
            ToolError::io(format!("could not write result: {error}"))
        } else {
            ToolError::internal(format!("could not serialize result: {error}"))
        }
    })?;
    writeln!(&mut stdout)
        .map_err(|error| ToolError::io(format!("could not write result: {error}")))?;
    stdout
        .flush()
        .map_err(|error| ToolError::io(format!("could not flush result: {error}")))
}

fn print_help() {
    println!(
        "kerf-model {TOOL_VERSION}\n\
Evidence-bearing semantic architecture compiler.\n\n\
USAGE:\n  kerf-model analyze MODEL.json\n  kerf-model derive-work MODEL.json\n  kerf-model check-coordinates COORDINATES.json\n  kerf-model project MODEL.json PROJECTION_ID\n\n\
COMMANDS:\n\
  analyze       Validate the typed model and expose conflicts, unknowns, and missing obligations\n\
  derive-work   Compile declared work plus unresolved obligations into a deterministic graph\n\
  check-coordinates  Reject unconverted coordinate mismatches and over-strong closure claims\n\
  project       Emit one exact existing-Kerf input and its source-scope manifest\n\n\
The compiler proves consequences of the supplied model. It does not prove that the model is a complete or accurate account of source code."
    );
}
