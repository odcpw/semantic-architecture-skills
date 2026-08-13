use kerf::report::{ToolError, sha256_hex};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use super::TransformationPolicy;

#[derive(Debug)]
pub(super) struct SourceFile {
    pub path: String,
    pub extension: String,
    pub lines: Vec<String>,
    pub tokens: BTreeSet<String>,
    pub sha256: String,
}

#[derive(Debug)]
pub(super) struct SourceInventory {
    pub files: Vec<SourceFile>,
    pub skipped: Vec<String>,
    pub content_sha256: String,
}

#[derive(Debug)]
pub(super) struct DuplicateBlock {
    pub sha256: String,
    pub normalized_lines: Vec<String>,
    pub locations: Vec<String>,
    pub extension: String,
}

#[derive(Serialize)]
struct InventoryIdentity<'a> {
    algorithm: &'static str,
    files: BTreeMap<&'a str, &'a str>,
}

pub(super) fn scan_repository(
    root: &Path,
    policy: &TransformationPolicy,
) -> Result<SourceInventory, ToolError> {
    let root = root.canonicalize().map_err(|error| {
        ToolError::io(format!(
            "canonicalize brownfield repository {}: {error}",
            root.display()
        ))
    })?;
    if !root.is_dir() {
        return Err(ToolError::invalid(format!(
            "brownfield repository {} is not a directory",
            root.display()
        )));
    }
    let extensions = policy
        .source_extensions
        .iter()
        .map(|extension| extension.trim_start_matches('.').to_ascii_lowercase())
        .collect::<BTreeSet<_>>();
    let mut paths = Vec::new();
    collect_paths(
        &root,
        &root,
        0,
        policy.max_depth,
        policy.max_files,
        &extensions,
        &mut paths,
    )?;
    paths.sort();
    if paths.len() > policy.max_files {
        return Err(ToolError::resource_limit(format!(
            "source discovery found {} files, exceeding max_files {}",
            paths.len(),
            policy.max_files
        )));
    }

    let mut files = Vec::new();
    let mut skipped = Vec::new();
    for path in paths {
        let relative = path
            .strip_prefix(&root)
            .map_err(|error| ToolError::internal(format!("strip repository prefix: {error}")))?
            .to_string_lossy()
            .replace('\\', "/");
        let metadata = fs::symlink_metadata(&path).map_err(|error| {
            ToolError::io(format!("read metadata for {}: {error}", path.display()))
        })?;
        if metadata.file_type().is_symlink() {
            skipped.push(format!("{relative}:symlink"));
            continue;
        }
        if metadata.len() > policy.max_bytes_per_file {
            skipped.push(format!("{relative}:over-byte-bound"));
            continue;
        }
        let bytes = fs::read(&path)
            .map_err(|error| ToolError::io(format!("read {}: {error}", path.display())))?;
        let Ok(text) = std::str::from_utf8(&bytes) else {
            skipped.push(format!("{relative}:non-utf8"));
            continue;
        };
        let extension = path
            .extension()
            .and_then(|extension| extension.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        files.push(SourceFile {
            path: relative,
            extension,
            lines: normalized_lines(text),
            tokens: semantic_tokens(text),
            sha256: sha256_hex(&bytes),
        });
    }
    let identity = InventoryIdentity {
        algorithm: "kerf.source-inventory.v1",
        files: files
            .iter()
            .map(|file| (file.path.as_str(), file.sha256.as_str()))
            .collect(),
    };
    let bytes = serde_json::to_vec(&identity)
        .map_err(|error| ToolError::internal(format!("serialize source inventory: {error}")))?;
    Ok(SourceInventory {
        files,
        skipped,
        content_sha256: sha256_hex(&bytes),
    })
}

fn collect_paths(
    root: &Path,
    directory: &Path,
    depth: usize,
    max_depth: usize,
    max_files: usize,
    extensions: &BTreeSet<String>,
    paths: &mut Vec<PathBuf>,
) -> Result<(), ToolError> {
    if depth > max_depth {
        return Ok(());
    }
    let mut entries = fs::read_dir(directory)
        .map_err(|error| ToolError::io(format!("read directory {}: {error}", directory.display())))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| {
            ToolError::io(format!(
                "read directory entry below {}: {error}",
                directory.display()
            ))
        })?;
    entries.sort_by_key(std::fs::DirEntry::file_name);
    for entry in entries {
        let path = entry.path();
        let relative = path.strip_prefix(root).map_err(|error| {
            ToolError::internal(format!("strip source-discovery root: {error}"))
        })?;
        let first = relative
            .components()
            .next()
            .and_then(|part| part.as_os_str().to_str());
        if first.is_some_and(|name| {
            matches!(
                name,
                ".git" | ".hg" | ".svn" | "target" | "node_modules" | "dist" | "build"
            )
        }) {
            continue;
        }
        let file_type = entry.file_type().map_err(|error| {
            ToolError::io(format!("read file type for {}: {error}", path.display()))
        })?;
        if file_type.is_symlink() {
            continue;
        }
        if file_type.is_dir() {
            collect_paths(
                root,
                &path,
                depth + 1,
                max_depth,
                max_files,
                extensions,
                paths,
            )?;
        } else if file_type.is_file()
            && path
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| extensions.contains(&extension.to_ascii_lowercase()))
        {
            if paths.len() >= max_files {
                return Err(ToolError::resource_limit(format!(
                    "source discovery exceeded max_files {max_files}"
                )));
            }
            paths.push(path);
        }
    }
    Ok(())
}

pub(super) fn duplicate_blocks(
    inventory: &SourceInventory,
    minimum_lines: usize,
) -> Vec<DuplicateBlock> {
    let mut occurrences: BTreeMap<String, (Vec<String>, String, BTreeSet<String>)> =
        BTreeMap::new();
    for file in &inventory.files {
        if file.lines.len() < minimum_lines {
            continue;
        }
        for (start, window) in file.lines.windows(minimum_lines).enumerate() {
            let joined = window.join("\n");
            if joined.len() < 64 || joined.chars().filter(char::is_ascii_alphanumeric).count() < 32
            {
                continue;
            }
            let hash = sha256_hex(joined.as_bytes());
            let location = format!("{}:{}", file.path, start + 1);
            occurrences
                .entry(hash)
                .and_modify(|(_, _, locations)| {
                    locations.insert(location.clone());
                })
                .or_insert_with(|| {
                    (
                        window.to_vec(),
                        file.extension.clone(),
                        BTreeSet::from([location]),
                    )
                });
        }
    }
    occurrences
        .into_iter()
        .filter_map(|(sha256, (normalized_lines, extension, locations))| {
            let distinct_files = locations
                .iter()
                .filter_map(|location| location.split_once(':').map(|(path, _)| path))
                .collect::<BTreeSet<_>>();
            (distinct_files.len() >= 2).then(|| DuplicateBlock {
                sha256,
                normalized_lines,
                locations: locations.into_iter().collect(),
                extension,
            })
        })
        .collect()
}

pub(super) fn anchors(text: &str) -> BTreeSet<String> {
    semantic_tokens(text)
        .into_iter()
        .filter(|token| token.len() >= 3 && !STOP_WORDS.contains(&token.as_str()))
        .collect()
}

fn normalized_lines(text: &str) -> Vec<String> {
    text.lines()
        .map(str::trim)
        .filter(|line| {
            !line.is_empty()
                && !matches!(*line, "{" | "}" | "(" | ")" | "[" | "]")
                && !line.starts_with("//")
                && !line.starts_with('#')
        })
        .map(ToOwned::to_owned)
        .collect()
}

fn semantic_tokens(text: &str) -> BTreeSet<String> {
    text.split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|token| !token.is_empty())
        .map(str::to_ascii_lowercase)
        .collect()
}

const STOP_WORDS: &[&str] = &[
    "and", "are", "const", "crate", "else", "false", "from", "function", "impl", "into", "mod",
    "module", "none", "public", "return", "self", "some", "struct", "that", "the", "this", "true",
    "use", "with",
];
