use kerf::report::{ToolError, sha256_hex};
use semantic_architecture::TOOL_VERSION;
use semantic_architecture::observe::{ObservationPlan, observe};
use semantic_architecture::semantic::parse_no_duplicates;
use serde::Serialize;
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::io::{self, Write};
use std::path::Path;

#[derive(Serialize)]
struct ResultEnvelope<T> {
    schema_version: &'static str,
    tool_version: &'static str,
    operation: &'static str,
    input_sha256: String,
    status: &'static str,
    result: T,
}

#[derive(Serialize)]
struct ErrorEnvelope<'a> {
    schema_version: &'static str,
    tool_version: &'static str,
    status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    input_sha256: Option<&'a str>,
    error: &'a ToolError,
}

fn main() {
    if let Err((error, input_sha256)) = run() {
        let envelope = ErrorEnvelope {
            schema_version: "kerf.observe-error.v1",
            tool_version: TOOL_VERSION,
            status: "error",
            input_sha256: input_sha256.as_deref(),
            error: &error,
        };
        let mut stderr = io::stderr().lock();
        serde_json::to_writer(&mut stderr, &envelope).ok();
        writeln!(&mut stderr).ok();
        std::process::exit(if error.code == "internal_invariant" {
            4
        } else {
            2
        });
    }
}

fn run() -> Result<(), (ToolError, Option<String>)> {
    let args: Vec<_> = env::args_os().skip(1).collect();
    match args.as_slice() {
        [flag] if flag == OsStr::new("--version") || flag == OsStr::new("-V") => {
            println!("kerf-observe {TOOL_VERSION}");
            Ok(())
        }
        [flag] if flag == OsStr::new("--help") || flag == OsStr::new("-h") => {
            print_help();
            Ok(())
        }
        [path] => dispatch(Path::new(path)),
        _ => Err((
            ToolError::invalid("usage: kerf-observe OBSERVATION_PLAN.json"),
            None,
        )),
    }
}

fn dispatch(path: &Path) -> Result<(), (ToolError, Option<String>)> {
    let bytes = fs::read(path).map_err(|error| {
        (
            ToolError::io(format!("could not read {}: {error}", path.display())),
            None,
        )
    })?;
    let input_sha256 = sha256_hex(&bytes);
    let plan: ObservationPlan =
        parse_no_duplicates(&bytes).map_err(|error| (error, Some(input_sha256.clone())))?;
    let result = observe(&plan).map_err(|error| (error, Some(input_sha256.clone())))?;
    let envelope = ResultEnvelope {
        schema_version: "kerf.observe-result.v1",
        tool_version: TOOL_VERSION,
        operation: "observe",
        input_sha256: input_sha256.clone(),
        status: "ok",
        result,
    };
    let mut stdout = io::stdout().lock();
    serde_json::to_writer(&mut stdout, &envelope).map_err(|error| {
        (
            ToolError::internal(format!("could not serialize observation result: {error}")),
            Some(input_sha256.clone()),
        )
    })?;
    writeln!(&mut stdout).map_err(|error| {
        (
            ToolError::io(format!("could not write observation result: {error}")),
            Some(input_sha256.clone()),
        )
    })?;
    stdout.flush().map_err(|error| {
        (
            ToolError::io(format!("could not flush observation result: {error}")),
            Some(input_sha256),
        )
    })
}

fn print_help() {
    println!(
        "kerf-observe {TOOL_VERSION}\n\
Capture repeated, environment-cleared command observations and emit a locally checked finite trace certificate.\n\n\
USAGE:\n  kerf-observe OBSERVATION_PLAN.json\n\n\
The plan runs user-supplied commands. It does not sandbox, time-limit, or constrain their external effects."
    );
}
