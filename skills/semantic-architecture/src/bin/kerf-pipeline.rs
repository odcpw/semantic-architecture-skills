use kerf::report::{ToolError, sha256_hex};
use semantic_architecture::TOOL_VERSION;
use semantic_architecture::coordinates::CoordinateModel;
use semantic_architecture::execution::{SquareReceipt, verify_square};
use semantic_architecture::semantic::parse_no_duplicates;
use serde::Serialize;
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::io::{self, Write};
use std::path::Path;

const RESULT_SCHEMA: &str = "kerf.pipeline-result.v1";
const ERROR_SCHEMA: &str = "kerf.pipeline-error.v1";

#[derive(Serialize)]
struct ResultEnvelope<T> {
    schema_version: &'static str,
    tool_version: &'static str,
    operation: &'static str,
    coordinate_input_sha256: String,
    receipt_input_sha256: String,
    status: &'static str,
    result: T,
}

#[derive(Debug)]
struct ExecutionError {
    error: ToolError,
    operation: Option<&'static str>,
    coordinate_input_sha256: Option<String>,
    receipt_input_sha256: Option<String>,
}

#[derive(Serialize)]
struct ErrorEnvelope<'a> {
    schema_version: &'static str,
    tool_version: &'static str,
    status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    operation: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    coordinate_input_sha256: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    receipt_input_sha256: Option<&'a str>,
    error: &'a ToolError,
}

fn main() {
    if let Err(error) = run() {
        let envelope = ErrorEnvelope {
            schema_version: ERROR_SCHEMA,
            tool_version: TOOL_VERSION,
            status: "error",
            operation: error.operation,
            coordinate_input_sha256: error.coordinate_input_sha256.as_deref(),
            receipt_input_sha256: error.receipt_input_sha256.as_deref(),
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
            println!("kerf-pipeline {TOOL_VERSION}");
            Ok(())
        }
        [flag] if flag == OsStr::new("--help") || flag == OsStr::new("-h") => {
            print_help();
            Ok(())
        }
        [command, coordinate_path, receipt_path] if command == OsStr::new("verify-square") => {
            dispatch_verify_square(Path::new(coordinate_path), Path::new(receipt_path))
        }
        _ => Err(ExecutionError {
            error: ToolError::invalid(
                "usage: kerf-pipeline verify-square COORDINATES.json RECEIPT.json",
            ),
            operation: None,
            coordinate_input_sha256: None,
            receipt_input_sha256: None,
        }),
    }
}

fn dispatch_verify_square(
    coordinate_path: &Path,
    receipt_path: &Path,
) -> Result<(), ExecutionError> {
    const OPERATION: &str = "verify-square";
    let coordinate_bytes = fs::read(coordinate_path).map_err(|error| ExecutionError {
        error: ToolError::io(format!(
            "could not read {}: {error}",
            coordinate_path.display()
        )),
        operation: Some(OPERATION),
        coordinate_input_sha256: None,
        receipt_input_sha256: None,
    })?;
    let coordinate_sha256 = sha256_hex(&coordinate_bytes);
    let receipt_bytes = fs::read(receipt_path).map_err(|error| ExecutionError {
        error: ToolError::io(format!(
            "could not read {}: {error}",
            receipt_path.display()
        )),
        operation: Some(OPERATION),
        coordinate_input_sha256: Some(coordinate_sha256.clone()),
        receipt_input_sha256: None,
    })?;
    let receipt_sha256 = sha256_hex(&receipt_bytes);
    let coordinates: CoordinateModel =
        parse_no_duplicates(&coordinate_bytes).map_err(|error| ExecutionError {
            error,
            operation: Some(OPERATION),
            coordinate_input_sha256: Some(coordinate_sha256.clone()),
            receipt_input_sha256: Some(receipt_sha256.clone()),
        })?;
    let receipt: SquareReceipt =
        parse_no_duplicates(&receipt_bytes).map_err(|error| ExecutionError {
            error,
            operation: Some(OPERATION),
            coordinate_input_sha256: Some(coordinate_sha256.clone()),
            receipt_input_sha256: Some(receipt_sha256.clone()),
        })?;
    let result = verify_square(&coordinates, &receipt).map_err(|error| ExecutionError {
        error,
        operation: Some(OPERATION),
        coordinate_input_sha256: Some(coordinate_sha256.clone()),
        receipt_input_sha256: Some(receipt_sha256.clone()),
    })?;
    let envelope = ResultEnvelope {
        schema_version: RESULT_SCHEMA,
        tool_version: TOOL_VERSION,
        operation: OPERATION,
        coordinate_input_sha256: coordinate_sha256,
        receipt_input_sha256: receipt_sha256,
        status: "ok",
        result,
    };
    let mut stdout = io::stdout().lock();
    serde_json::to_writer(&mut stdout, &envelope).map_err(|error| ExecutionError {
        error: ToolError::internal(format!("could not serialize result: {error}")),
        operation: Some(OPERATION),
        coordinate_input_sha256: None,
        receipt_input_sha256: None,
    })?;
    writeln!(&mut stdout).map_err(|error| ExecutionError {
        error: ToolError::io(format!("could not write result: {error}")),
        operation: Some(OPERATION),
        coordinate_input_sha256: None,
        receipt_input_sha256: None,
    })?;
    stdout.flush().map_err(|error| ExecutionError {
        error: ToolError::io(format!("could not flush result: {error}")),
        operation: Some(OPERATION),
        coordinate_input_sha256: None,
        receipt_input_sha256: None,
    })
}

fn print_help() {
    println!(
        "kerf-pipeline {TOOL_VERSION}\n\
Artifact-bound semantic pipeline verification.\n\n\
USAGE:\n  kerf-pipeline verify-square COORDINATES.json RECEIPT.json\n\n\
COMMANDS:\n  verify-square  Re-hash declared inputs/outputs and compare every named output pair byte-for-byte\n\n\
The verifier checks existing artifacts. It does not execute the declared commands, attest builds, or prove source completeness."
    );
}
