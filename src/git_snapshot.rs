use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Read,
    path::Path,
    process::{Command, Stdio},
    time::Duration,
};

const MAX_FILE: u64 = 8 * 1024 * 1024;
const MAX_TOTAL: u64 = 128 * 1024 * 1024;
const MAX_FILES: usize = 100_000;
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct FileState {
    pub sha256: String,
    pub bytes: u64,
    pub executable: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    pub head: Option<String>,
    pub fingerprint: String,
    pub files: BTreeMap<String, FileState>,
    pub complete: bool,
    pub limitations: Vec<String>,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Changes {
    pub state: String,
    pub total_files: Option<usize>,
    pub sample_paths: Vec<String>,
    pub head_changed: bool,
    pub line_counts: Option<String>,
}
fn git(root: &Path) -> Command {
    let mut c = Command::new("git");
    c.current_dir(root)
        .env("GIT_OPTIONAL_LOCKS", "0")
        .stdin(Stdio::null());
    c
}
pub fn require_root(root: &Path) -> Result<()> {
    let canonical_root = root.canonicalize()?;
    let root = canonical_root.as_path();
    let output = crate::process::run(
        git(root).args(["rev-parse", "--show-toplevel"]),
        Duration::from_secs(10),
    )?;
    ensure!(
        output.status.success() && !output.timed_out,
        "run Hannes inside a Git repository (git init first)"
    );
    let found = Path::new(output.stdout.text.trim()).canonicalize()?;
    ensure!(
        found == root,
        "run Hannes from the repository root: {}",
        found.display()
    );
    Ok(())
}
fn excluded(path: &Path) -> bool {
    path.starts_with(".git") || path.starts_with(".Hannes/reports")
}
fn add_config(
    root: &Path,
    directory: &Path,
    paths: &mut BTreeSet<String>,
    limitations: &mut Vec<String>,
    depth: usize,
) -> Result<()> {
    if excluded(directory) {
        return Ok(());
    }
    if depth > 16 {
        limitations.push("configuration nesting exceeds 16".into());
        return Ok(());
    }
    let full = root.join(directory);
    if !full.exists() {
        return Ok(());
    }
    if full.symlink_metadata()?.file_type().is_symlink() {
        limitations.push(format!("symlink: {}", directory.display()));
        return Ok(());
    }
    for entry in fs::read_dir(full)? {
        let entry = entry?;
        let path = directory.join(entry.file_name());
        if excluded(&path) {
            continue;
        }
        if paths.len() >= MAX_FILES {
            limitations.push("file count exceeds 100000".into());
            break;
        }
        if entry.file_type()?.is_dir() {
            add_config(root, &path, paths, limitations, depth + 1)?;
        } else if let Some(s) = path.to_str() {
            #[cfg(windows)]
            let path = s.replace('\\', "/");
            #[cfg(not(windows))]
            let path = s.to_owned();
            paths.insert(path);
        } else {
            limitations.push("non-UTF-8 configuration path".into());
        }
    }
    Ok(())
}
pub fn capture(root: &Path) -> Result<Snapshot> {
    let canonical_root = root.canonicalize()?;
    let root = canonical_root.as_path();
    require_root(root)?;
    let mut limitations = Vec::new();
    let head_out = crate::process::run(
        git(root).args(["rev-parse", "--verify", "HEAD"]),
        Duration::from_secs(10),
    )?;
    ensure!(!head_out.timed_out, "Git HEAD lookup timed out");
    let head = if head_out.status.success() {
        Some(head_out.stdout.text.trim().to_owned())
    } else {
        let symbolic = crate::process::run(
            git(root).args(["symbolic-ref", "-q", "HEAD"]),
            Duration::from_secs(10),
        )?;
        ensure!(symbolic.status.success(), "cannot resolve Git HEAD");
        None
    };
    // NUL-separated paths: whitespace and newlines in file names remain unambiguous.
    let list = crate::process::run(
        git(root).args([
            "-c",
            "core.quotepath=false",
            "ls-files",
            "-z",
            "--cached",
            "--others",
            "--exclude-standard",
        ]),
        Duration::from_secs(30),
    )?;
    ensure!(
        list.status.success() && !list.timed_out,
        "Git file enumeration failed"
    );
    // The process capture is deliberately bounded. Never claim complete coverage after truncation.
    if list.stdout.truncated {
        limitations.push("Git file list exceeded 64 KiB".into());
    }
    let mut paths = BTreeSet::new();
    for raw in list.stdout.text.split_terminator('\0') {
        if raw.contains('\u{fffd}') {
            limitations.push("unsupported non-UTF-8 path".into());
            continue;
        }
        if !excluded(Path::new(raw)) {
            paths.insert(raw.to_owned());
        }
    }
    add_config(root, Path::new(".Hannes"), &mut paths, &mut limitations, 0)?;
    let mut files = BTreeMap::new();
    let mut total = 0;
    for path in paths {
        if files.len() >= MAX_FILES {
            limitations.push("file count exceeds 100000".into());
            break;
        }
        let full = root.join(&path);
        crate::config::relative(Path::new(&path))?;
        let meta = match full.symlink_metadata() {
            Ok(m) => m,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => {
                limitations.push(format!("cannot inspect {path}: {e}"));
                continue;
            }
        };
        if meta.file_type().is_symlink() || !meta.is_file() {
            limitations.push(format!(
                "unsupported symlink/submodule/special file: {path}"
            ));
            continue;
        }
        // Canonical containment also prevents traversing a symlink in a parent directory.
        if !full.canonicalize()?.starts_with(root) {
            limitations.push(format!("outside root: {path}"));
            continue;
        }
        if meta.len() > MAX_FILE || total + meta.len() > MAX_TOTAL {
            limitations.push(format!("hash budget exceeded: {path}"));
            continue;
        }
        let mut bytes = Vec::new();
        match fs::File::open(&full).and_then(|f| f.take(MAX_FILE + 1).read_to_end(&mut bytes)) {
            Ok(_) if bytes.len() as u64 <= MAX_FILE => {}
            _ => {
                limitations.push(format!("unreadable/oversized file: {path}"));
                continue;
            }
        }
        total += bytes.len() as u64;
        #[cfg(unix)]
        let executable = {
            use std::os::unix::fs::PermissionsExt;
            meta.permissions().mode() & 0o111 != 0
        };
        #[cfg(not(unix))]
        let executable = false;
        files.insert(
            path,
            FileState {
                sha256: format!("{:x}", Sha256::digest(&bytes)),
                bytes: bytes.len() as u64,
                executable,
            },
        );
    }
    let fingerprint = format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&(&head, &files))?)
    );
    Ok(Snapshot {
        head,
        fingerprint,
        files,
        complete: limitations.is_empty(),
        limitations,
    })
}
pub fn compare(before: &Snapshot, after: &Snapshot, limit: usize) -> Changes {
    let paths: BTreeSet<_> = before.files.keys().chain(after.files.keys()).collect();
    let changed: Vec<_> = paths
        .into_iter()
        .filter(|p| before.files.get(*p) != after.files.get(*p))
        .cloned()
        .collect();
    let head_changed = before.head != after.head;
    let complete = before.complete && after.complete;
    let state = if !complete {
        "UNKNOWN"
    } else if head_changed || !changed.is_empty() {
        "STALE"
    } else {
        "CURRENT"
    };
    Changes {
        state: state.into(),
        total_files: complete.then_some(changed.len()),
        sample_paths: changed.into_iter().take(limit.min(5)).collect(),
        head_changed,
        line_counts: None,
    }
}
pub fn bounded_diff(root: &Path, has_head: bool) -> Result<(String, bool)> {
    bounded_diff_for_paths(root, has_head, None)
}
pub fn bounded_diff_for_paths(
    root: &Path,
    has_head: bool,
    paths: Option<&[String]>,
) -> Result<(String, bool)> {
    let mut cmd = git(root);
    if let Some(paths) = paths {
        ensure!(!paths.is_empty(), "focused diff requires entry files");
        cmd.arg("--literal-pathspecs");
    }
    cmd.args(["diff", "--no-ext-diff", "--no-textconv", "--no-color"]);
    if has_head {
        cmd.arg("HEAD");
    }
    if let Some(paths) = paths {
        cmd.arg("--").args(paths);
    } else {
        cmd.args(["--", ".", ":(exclude).Hannes/reports"]);
    }
    let out = crate::process::run(&mut cmd, Duration::from_secs(20)).context("Git diff failed")?;
    ensure!(
        out.status.success() && !out.timed_out,
        "Git diff failed or timed out"
    );
    let mut text = out.stdout.text;
    let truncated = out.stdout.truncated || text.len() > 32768;
    let mut end = text.len().min(32768);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    text.truncate(end);
    Ok((text, truncated))
}

/// Git status is orientation only; never used for since-verification comparisons.
pub fn working_summary(root: &Path) -> Result<String> {
    let output = crate::process::run(
        git(root).args(["status", "--porcelain=v1", "-z", "--untracked-files=all"]),
        Duration::from_secs(20),
    )?;
    ensure!(
        output.status.success() && !output.timed_out && !output.stdout.truncated,
        "Git status unavailable, timed out or truncated"
    );
    let mut entries = output.stdout.text.split_terminator('\0');
    let (mut staged, mut unstaged, mut untracked) = (0, 0, 0);
    while let Some(entry) = entries.next() {
        let bytes = entry.as_bytes();
        ensure!(
            bytes.len() >= 3 && bytes[2] == b' ',
            "invalid Git status record"
        );
        let rename = b"RC".contains(&bytes[0]) || b"RC".contains(&bytes[1]);
        if rename {
            entries.next();
        }
        if excluded(Path::new(&entry[3..])) {
            continue;
        }
        if &bytes[..2] == b"??" {
            untracked += 1;
        } else {
            if bytes[0] != b' ' {
                staged += 1;
            }
            if bytes[1] != b' ' {
                unstaged += 1;
            }
        }
    }
    Ok(format!(
        "Git status: {staged} staged, {unstaged} unstaged, {untracked} untracked paths (categories may overlap; not changes since verification)"
    ))
}
