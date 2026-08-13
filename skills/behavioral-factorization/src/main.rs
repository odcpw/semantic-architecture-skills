use kerf::TOOL_VERSION;
use kerf::candidates::{self, CandidateInput, MigrationGraphInput};
use kerf::cert::{self, CertificateInput};
use kerf::factor::{self, FactorInput, FactorLimits};
use kerf::factor_search::{self, FactorSearchInput, FactorSearchLimits};
use kerf::lts::{self, CompareInput, Limits, LtsInput};
use kerf::morphism::{self, MorphismInput, MorphismLimits};
use kerf::recompose::{self, RecomposeInput};
use kerf::report::{ToolError, sha256_hex};
use kerf::transport::{self, TransportInput};
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::io::{self, Write};
use std::path::Path;

const RESULT_SCHEMA: &str = "kerf.result.v1";

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(untagged)]
enum AppliedLimits {
    Lts(Limits),
    Factor(FactorLimits),
    FactorSearch(FactorSearchLimits),
    Morphism(MorphismLimits),
}

#[derive(Serialize)]
struct ResultEnvelope<T> {
    schema_version: &'static str,
    tool_version: &'static str,
    operation: String,
    input_sha256: String,
    status: &'static str,
    limits: Option<AppliedLimits>,
    result: T,
}

#[derive(Debug)]
struct ExecutionError {
    error: Box<ToolError>,
    operation: Option<String>,
    input_sha256: Option<String>,
    limits: Option<AppliedLimits>,
}

#[derive(Serialize)]
struct ErrorEnvelope<'a> {
    schema_version: &'static str,
    tool_version: &'static str,
    status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    operation: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    input_sha256: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    limits: Option<AppliedLimits>,
    error: &'a ToolError,
}

fn main() {
    if let Err(error) = run() {
        let envelope = ErrorEnvelope {
            schema_version: "kerf.error.v1",
            tool_version: TOOL_VERSION,
            status: if error.error.code == "resource_limit" {
                "unknown"
            } else {
                "error"
            },
            operation: error.operation.as_deref(),
            input_sha256: error.input_sha256.as_deref(),
            limits: error.limits,
            error: &error.error,
        };
        let mut stderr = io::stderr().lock();
        serde_json::to_writer(&mut stderr, &envelope).ok();
        writeln!(&mut stderr).ok();
        std::process::exit(match error.error.code {
            "resource_limit" => 3,
            "internal_invariant" => 4,
            _ => 2,
        });
    }
}

fn run() -> Result<(), ExecutionError> {
    let args: Vec<_> = env::args_os().skip(1).collect();
    match args.as_slice() {
        [flag] if flag == OsStr::new("--version") || flag == OsStr::new("-V") => {
            println!("kerf {TOOL_VERSION}");
            Ok(())
        }
        [flag] if flag == OsStr::new("--help") || flag == OsStr::new("-h") => {
            print_help();
            Ok(())
        }
        [command, path] => {
            let command = command
                .to_str()
                .ok_or_else(|| bare_error(ToolError::invalid("command must be valid UTF-8")))?;
            if !is_command(command) {
                return Err(ExecutionError {
                    error: Box::new(ToolError::invalid(format!(
                        "unknown command {command:?}; run kerf --help"
                    ))),
                    operation: Some(command.to_owned()),
                    input_sha256: None,
                    limits: None,
                });
            }
            dispatch(command, Path::new(path))
        }
        _ => Err(bare_error(ToolError::invalid(
            "usage: kerf <validate-lts|quotient|distinguish|factor|search-factors|recompose|check-morphism|verify-certificate|validate-candidates|validate-migration|pareto|score-routes> <input.json>",
        ))),
    }
}

fn is_command(command: &str) -> bool {
    matches!(
        command,
        "validate-lts"
            | "quotient"
            | "distinguish"
            | "factor"
            | "search-factors"
            | "recompose"
            | "check-morphism"
            | "verify-certificate"
            | "validate-candidates"
            | "validate-migration"
            | "pareto"
            | "score-routes"
    )
}

fn dispatch(command: &str, path: &Path) -> Result<(), ExecutionError> {
    let bytes = fs::read(path).map_err(|error| ExecutionError {
        error: Box::new(ToolError::io(format!(
            "could not read {}: {error}",
            path.display()
        ))),
        operation: Some(command.to_owned()),
        input_sha256: None,
        limits: None,
    })?;
    let input_sha256 = sha256_hex(&bytes);
    match command {
        "validate-lts" => {
            let input: LtsInput =
                parse(&bytes).map_err(|error| contextual(error, command, &input_sha256, None))?;
            let limits = Some(AppliedLimits::Lts(input.limits));
            let result = lts::validate(&input)
                .map_err(|error| contextual(error, command, &input_sha256, limits))?;
            emit(command, input_sha256.clone(), limits, result)
                .map_err(|error| contextual(error, command, &input_sha256, limits))
        }
        "quotient" => {
            let input: LtsInput =
                parse(&bytes).map_err(|error| contextual(error, command, &input_sha256, None))?;
            let limits = Some(AppliedLimits::Lts(input.limits));
            let result = lts::quotient(&input)
                .map_err(|error| contextual(error, command, &input_sha256, limits))?;
            emit(command, input_sha256.clone(), limits, result)
                .map_err(|error| contextual(error, command, &input_sha256, limits))
        }
        "distinguish" => {
            let input: CompareInput =
                parse(&bytes).map_err(|error| contextual(error, command, &input_sha256, None))?;
            let limits = Some(AppliedLimits::Lts(input.limits));
            let result = lts::compare(&input)
                .map_err(|error| contextual(error, command, &input_sha256, limits))?;
            emit(command, input_sha256.clone(), limits, result)
                .map_err(|error| contextual(error, command, &input_sha256, limits))
        }
        "factor" => {
            let input: FactorInput =
                parse(&bytes).map_err(|error| contextual(error, command, &input_sha256, None))?;
            let limits = Some(AppliedLimits::Factor(input.limits));
            let result = factor::check(&input)
                .map_err(|error| contextual(error, command, &input_sha256, limits))?;
            emit(command, input_sha256.clone(), limits, result)
                .map_err(|error| contextual(error, command, &input_sha256, limits))
        }
        "search-factors" => {
            let input: FactorSearchInput =
                parse(&bytes).map_err(|error| contextual(error, command, &input_sha256, None))?;
            let limits = Some(AppliedLimits::FactorSearch(input.limits));
            let result = factor_search::search(&input)
                .map_err(|error| contextual(error, command, &input_sha256, limits))?;
            emit(command, input_sha256.clone(), limits, result)
                .map_err(|error| contextual(error, command, &input_sha256, limits))
        }
        "recompose" => {
            let input: RecomposeInput =
                parse(&bytes).map_err(|error| contextual(error, command, &input_sha256, None))?;
            let limits = Some(AppliedLimits::Lts(input.limits));
            let result = recompose::check(&input)
                .map_err(|error| contextual(error, command, &input_sha256, limits))?;
            emit(command, input_sha256.clone(), limits, result)
                .map_err(|error| contextual(error, command, &input_sha256, limits))
        }
        "check-morphism" => dispatch_morphism(&bytes, command, &input_sha256),
        "verify-certificate" => {
            let input: CertificateInput =
                parse(&bytes).map_err(|error| contextual(error, command, &input_sha256, None))?;
            let result = cert::verify(&input)
                .map_err(|error| contextual(error, command, &input_sha256, None))?;
            emit(command, input_sha256.clone(), None, result)
                .map_err(|error| contextual(error, command, &input_sha256, None))
        }
        "validate-candidates" => {
            let input: CandidateInput =
                parse(&bytes).map_err(|error| contextual(error, command, &input_sha256, None))?;
            let result = candidates::validate(&input)
                .map_err(|error| contextual(error, command, &input_sha256, None))?;
            emit(command, input_sha256.clone(), None, result)
                .map_err(|error| contextual(error, command, &input_sha256, None))
        }
        "validate-migration" => dispatch_migration(&bytes, command, &input_sha256),
        "pareto" => {
            let input: CandidateInput =
                parse(&bytes).map_err(|error| contextual(error, command, &input_sha256, None))?;
            let result = candidates::pareto(&input)
                .map_err(|error| contextual(error, command, &input_sha256, None))?;
            emit(command, input_sha256.clone(), None, result)
                .map_err(|error| contextual(error, command, &input_sha256, None))
        }
        "score-routes" => dispatch_transport(&bytes, command, &input_sha256),
        _ => unreachable!("command validated before input is read"),
    }
}

fn dispatch_morphism(
    bytes: &[u8],
    command: &str,
    input_sha256: &str,
) -> Result<(), ExecutionError> {
    let input: MorphismInput =
        parse(bytes).map_err(|error| contextual(error, command, input_sha256, None))?;
    let limits = morphism::applied_limits(&input).map(AppliedLimits::Morphism);
    let result = morphism::check(&input)
        .map_err(|error| contextual(error, command, input_sha256, limits))?;
    emit(command, input_sha256.to_owned(), limits, result)
        .map_err(|error| contextual(error, command, input_sha256, limits))
}

fn dispatch_migration(
    bytes: &[u8],
    command: &str,
    input_sha256: &str,
) -> Result<(), ExecutionError> {
    let input: MigrationGraphInput =
        parse(bytes).map_err(|error| contextual(error, command, input_sha256, None))?;
    let result = candidates::validate_migration(&input)
        .map_err(|error| contextual(error, command, input_sha256, None))?;
    emit(command, input_sha256.to_owned(), None, result)
        .map_err(|error| contextual(error, command, input_sha256, None))
}

fn dispatch_transport(
    bytes: &[u8],
    command: &str,
    input_sha256: &str,
) -> Result<(), ExecutionError> {
    let input: TransportInput =
        parse(bytes).map_err(|error| contextual(error, command, input_sha256, None))?;
    let result =
        transport::score(&input).map_err(|error| contextual(error, command, input_sha256, None))?;
    emit(command, input_sha256.to_owned(), None, result)
        .map_err(|error| contextual(error, command, input_sha256, None))
}

fn bare_error(error: ToolError) -> ExecutionError {
    ExecutionError {
        error: Box::new(error),
        operation: None,
        input_sha256: None,
        limits: None,
    }
}

fn contextual(
    error: ToolError,
    operation: &str,
    input_sha256: &str,
    limits: Option<AppliedLimits>,
) -> ExecutionError {
    ExecutionError {
        error: Box::new(error),
        operation: Some(operation.to_owned()),
        input_sha256: Some(input_sha256.to_owned()),
        limits,
    }
}

fn parse<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, ToolError> {
    serde_json::from_slice(bytes)
        .map_err(|error| ToolError::invalid(format!("invalid JSON input: {error}")))
}

fn emit<T: Serialize>(
    operation: &str,
    input_sha256: String,
    limits: Option<AppliedLimits>,
    result: T,
) -> Result<(), ToolError> {
    let envelope = ResultEnvelope {
        schema_version: RESULT_SCHEMA,
        tool_version: TOOL_VERSION,
        operation: operation.to_owned(),
        input_sha256,
        status: "ok",
        limits,
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
        .map_err(|error| ToolError::io(format!("could not flush result: {error}")))?;
    Ok(())
}

fn print_help() {
    println!(
        "kerf {TOOL_VERSION}\n\
Exact finite behavioral-factorization checks.\n\n\
USAGE:\n  kerf COMMAND INPUT.json\n\n\
COMMANDS:\n\
  validate-lts          Validate a kerf.lts.v1 model\n\
  quotient              Compute its reachable observational quotient\n\
  distinguish           Compare two kerf.compare.v1 machines and emit a shortest witness\n\
  factor                Check a supplied context-relative boundary-state map\n\
  search-factors        Find an exact bounded minimum lawful factor map\n\
  recompose             Check exact reachable synchronous recomposition\n\
  check-morphism        Check a supplied exact finite machine morphism\n\
  verify-certificate    Locally verify a finite relation, witness, or trace\n\
  validate-candidates   Check explicit assignment and authority constraints\n\
  validate-migration    Check a proof-carrying finite migration graph\n\
  pareto                Compute the exact frontier of the supplied candidate set\n\
  score-routes          Account exact load on supplied static routes\n\n\
All analysis results are versioned JSON on stdout. Errors are typed JSON on stderr."
    );
}
