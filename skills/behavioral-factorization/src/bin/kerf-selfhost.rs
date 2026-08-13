use kerf::TOOL_VERSION;
use kerf::report::{ToolError, sha256_hex};
use kerf::self_host::{SelfHostInput, SelfHostResult, check};
use serde::Serialize;
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::io::{self, Write};
use std::path::Path;

const RESULT_SCHEMA: &str = "kerf.self-host-result.v1";

#[derive(Serialize)]
struct ResultEnvelope {
    schema_version: &'static str,
    tool_version: &'static str,
    status: &'static str,
    result: SelfHostResult,
}

#[derive(Serialize)]
struct ErrorEnvelope<'a> {
    schema_version: &'static str,
    tool_version: &'static str,
    status: &'static str,
    error: &'a ToolError,
}

fn main() {
    match run() {
        Ok(true) => {}
        Ok(false) => std::process::exit(1),
        Err(error) => {
            let envelope = ErrorEnvelope {
                schema_version: "kerf.self-host-error.v1",
                tool_version: TOOL_VERSION,
                status: "error",
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
}

fn run() -> Result<bool, ToolError> {
    let args: Vec<_> = env::args_os().skip(1).collect();
    match args.as_slice() {
        [flag] if flag == OsStr::new("--version") || flag == OsStr::new("-V") => {
            println!("kerf-selfhost {TOOL_VERSION}");
            Ok(true)
        }
        [flag] if flag == OsStr::new("--help") || flag == OsStr::new("-h") => {
            print_help();
            Ok(true)
        }
        [
            kerf_flag,
            executable,
            repo_flag,
            repository,
            head_flag,
            expected_head,
            corpus_flag,
            corpus,
        ] if kerf_flag == OsStr::new("--kerf")
            && repo_flag == OsStr::new("--repo-root")
            && head_flag == OsStr::new("--expect-head")
            && corpus_flag == OsStr::new("--corpus") =>
        {
            run_corpus(
                Path::new(executable),
                Path::new(repository),
                expected_head,
                true,
                Path::new(corpus),
            )
        }
        [
            kerf_flag,
            executable,
            repo_flag,
            repository,
            head_flag,
            expected_head,
            corpus_flag,
            corpus,
            allow_dirty,
        ] if kerf_flag == OsStr::new("--kerf")
            && repo_flag == OsStr::new("--repo-root")
            && head_flag == OsStr::new("--expect-head")
            && corpus_flag == OsStr::new("--corpus")
            && allow_dirty == OsStr::new("--allow-dirty") =>
        {
            run_corpus(
                Path::new(executable),
                Path::new(repository),
                expected_head,
                false,
                Path::new(corpus),
            )
        }
        _ => Err(ToolError::invalid(
            "usage: kerf-selfhost --kerf PATH --repo-root PATH --expect-head HEX --corpus CORPUS.json [--allow-dirty]",
        )),
    }
}

fn run_corpus(
    executable: &Path,
    repository: &Path,
    expected_head: &OsStr,
    require_clean: bool,
    corpus: &Path,
) -> Result<bool, ToolError> {
    let expected_head = expected_head
        .to_str()
        .ok_or_else(|| ToolError::invalid("expected Git HEAD must be valid UTF-8"))?;
    let bytes = fs::read(corpus).map_err(|error| {
        ToolError::io(format!(
            "could not read self-host corpus {}: {error}",
            corpus.display()
        ))
    })?;
    let input: SelfHostInput = serde_json::from_slice(&bytes)
        .map_err(|error| ToolError::invalid(format!("invalid JSON input: {error}")))?;
    let root = corpus.parent().unwrap_or_else(|| Path::new("."));
    let result = check(
        executable,
        repository,
        expected_head,
        require_clean,
        root,
        sha256_hex(&bytes),
        &input,
    )?;
    let matched = result.matched;
    let envelope = ResultEnvelope {
        schema_version: RESULT_SCHEMA,
        tool_version: TOOL_VERSION,
        status: "ok",
        result,
    };
    let mut stdout = io::stdout().lock();
    serde_json::to_writer(&mut stdout, &envelope)
        .map_err(|error| ToolError::internal(format!("could not serialize result: {error}")))?;
    writeln!(&mut stdout)
        .map_err(|error| ToolError::io(format!("could not write result: {error}")))?;
    Ok(matched)
}

fn print_help() {
    println!(
        "kerf-selfhost {TOOL_VERSION}\n\
Run one exact kerf executable against a revision-bound raw-byte process corpus.\n\n\
USAGE:\n  kerf-selfhost --kerf PATH --repo-root PATH --expect-head HEX --corpus CORPUS.json [--allow-dirty]\n\n\
Exit 0 means every process observation matched; exit 1 is a checked mismatch;\n\
exit 2 is invalid input or I/O failure. JSON results use stdout; errors use stderr."
    );
}
