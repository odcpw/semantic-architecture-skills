use kerf::report::{ToolError, sha256_hex};
use semantic_architecture::TOOL_VERSION;
use semantic_architecture::cycle::PurposeEnvelope;
use semantic_architecture::generate::{
    TransformationBatch, TransformationRequest, environment_sha256, generate_transformations,
    generator_registry, hard_constraints_sha256, metric_policy_sha256, purpose_sha256,
    transformation_policy_sha256, validate_transformation_batch,
};
use semantic_architecture::semantic::parse_no_duplicates;
use serde::{Serialize, de::DeserializeOwned};
use std::collections::BTreeMap;
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::io::{self, Write};
use std::path::Path;

const RESULT_SCHEMA: &str = "kerf.generate-result.v1";
const ERROR_SCHEMA: &str = "kerf.generate-error.v1";

#[derive(Serialize)]
struct ResultEnvelope<T> {
    schema_version: &'static str,
    tool_version: &'static str,
    operation: &'static str,
    input_sha256s: BTreeMap<String, String>,
    status: &'static str,
    result: T,
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
            println!("kerf-generate {TOOL_VERSION}");
            Ok(())
        }
        [flag] if flag == OsStr::new("--help") || flag == OsStr::new("-h") => {
            print_help();
            Ok(())
        }
        [command] if command == OsStr::new("registry") => {
            write_result("registry", BTreeMap::new(), generator_registry())
        }
        [command, request] if command == OsStr::new("bind-envelope") => {
            dispatch_bind_envelope(Path::new(request))
        }
        [command, request] if command == OsStr::new("generate") => {
            dispatch_generate(Path::new(request), false)
        }
        [command, request, flag]
            if command == OsStr::new("generate") && flag == OsStr::new("--require-complete") =>
        {
            dispatch_generate(Path::new(request), true)
        }
        [command, request, batch] if command == OsStr::new("validate") => {
            dispatch_validate(Path::new(request), Path::new(batch))
        }
        _ => Err(usage_error()),
    }
}

fn dispatch_bind_envelope(path: &Path) -> Result<(), ExecutionError> {
    const OPERATION: &str = "bind-envelope";
    let (request, inputs) = read_document::<TransformationRequest>(path, "request", OPERATION)?;
    let envelope = PurposeEnvelope {
        envelope_id: request.envelope.envelope_id,
        purpose_sha256: purpose_sha256(&request.purpose).map_err(|error| ExecutionError {
            error,
            operation: Some(OPERATION),
            input_sha256s: inputs.clone(),
        })?,
        environment_sha256: environment_sha256(&request.environment).map_err(|error| {
            ExecutionError {
                error,
                operation: Some(OPERATION),
                input_sha256s: inputs.clone(),
            }
        })?,
        hard_constraints_sha256: hard_constraints_sha256(&request.hard_constraints).map_err(
            |error| ExecutionError {
                error,
                operation: Some(OPERATION),
                input_sha256s: inputs.clone(),
            },
        )?,
        metric_policy_sha256: metric_policy_sha256(&request.metrics).map_err(|error| {
            ExecutionError {
                error,
                operation: Some(OPERATION),
                input_sha256s: inputs.clone(),
            }
        })?,
        transformation_policy_sha256: transformation_policy_sha256(&request.policy).map_err(
            |error| ExecutionError {
                error,
                operation: Some(OPERATION),
                input_sha256s: inputs.clone(),
            },
        )?,
    };
    write_result(OPERATION, inputs, envelope)
}

fn dispatch_generate(path: &Path, require_complete: bool) -> Result<(), ExecutionError> {
    const OPERATION: &str = "generate";
    let (request, inputs) = read_document::<TransformationRequest>(path, "request", OPERATION)?;
    let result = generate_transformations(&request).map_err(|error| ExecutionError {
        error,
        operation: Some(OPERATION),
        input_sha256s: inputs.clone(),
    })?;
    if require_complete && !result.coverage.complete {
        return Err(ExecutionError {
            error: ToolError {
                code: "gate_unsatisfied",
                message: "transformation discovery was bounded or skipped source files".to_owned(),
                details: Some(Box::new(serde_json::json!({
                    "truncated": result.coverage.truncated,
                    "skipped_files": result.coverage.skipped_files,
                }))),
            },
            operation: Some(OPERATION),
            input_sha256s: inputs,
        });
    }
    write_result(OPERATION, inputs, result)
}

fn dispatch_validate(request: &Path, batch: &Path) -> Result<(), ExecutionError> {
    const OPERATION: &str = "validate";
    let (request, mut inputs) =
        read_document::<TransformationRequest>(request, "request", OPERATION)?;
    let (batch, batch_inputs) = read_document::<TransformationBatch>(batch, "batch", OPERATION)?;
    inputs.extend(batch_inputs);
    validate_transformation_batch(&request, &batch).map_err(|error| ExecutionError {
        error,
        operation: Some(OPERATION),
        input_sha256s: inputs.clone(),
    })?;
    write_result(
        OPERATION,
        inputs,
        serde_json::json!({
            "valid": true,
            "candidate_count": batch.candidates.len(),
            "registry_sha256": batch.registry.registry_sha256,
        }),
    )
}

fn read_document<T: DeserializeOwned>(
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
    let document = parse_no_duplicates(&bytes).map_err(|error| ExecutionError {
        error,
        operation: Some(operation),
        input_sha256s: inputs.clone(),
    })?;
    Ok((document, inputs))
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

fn usage_error() -> ExecutionError {
    ExecutionError {
        error: ToolError::invalid(
            "usage: kerf-generate registry | bind-envelope REQUEST.json | generate REQUEST.json [--require-complete] | validate REQUEST.json BATCH.json",
        ),
        operation: None,
        input_sha256s: BTreeMap::new(),
    }
}

fn print_help() {
    println!(
        "kerf-generate {TOOL_VERSION}\n\
Deterministic whole-system transformation generation.\n\n\
USAGE:\n  kerf-generate registry\n  kerf-generate bind-envelope REQUEST.json\n  kerf-generate generate REQUEST.json [--require-complete]\n  kerf-generate validate REQUEST.json BATCH.json\n\n\
bind-envelope computes the five content identities after editing a request draft. generate accepts greenfield intent or a brownfield repository root and emits semantic witnesses, qualitative predictions for every metric, proof obligations, falsifiers, and nonclaims. It does not apply or approve candidates. Exit codes: 0=operation completed (and any requested gate passed), 2=invalid input, 3=requested completeness gate was not satisfied, 4=internal invariant, 5=I/O failure."
    );
}
