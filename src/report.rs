use crate::{git_snapshot::Snapshot, process::Capture};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::{io::Write, path::Path};

pub const MAX_SUMMARY_CHARS: usize = 120;

/// Provider-reported counters for the single bundled review invocation.
/// Cache and reasoning counters are subsets, never added to the total again.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TokenUsage {
    pub input_tokens: u64,
    pub cached_input_tokens: u64,
    pub output_tokens: u64,
    pub reasoning_output_tokens: Option<u64>,
    pub cache_write_input_tokens: Option<u64>,
}
impl TokenUsage {
    pub fn total(self) -> Option<u64> {
        self.input_tokens.checked_add(self.output_tokens)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Status {
    Pass,
    Fail,
    Error,
    NotRun,
}
impl Status {
    pub fn label(self) -> &'static str {
        match self {
            Self::Pass => "PASS",
            Self::Fail => "FAIL",
            Self::Error => "ERROR",
            Self::NotRun => "NOT_RUN",
        }
    }
    pub fn exit_code(self) -> i32 {
        match self {
            Self::Pass => 0,
            Self::Fail => 1,
            _ => 2,
        }
    }
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Finding {
    pub message: String,
    pub path: Option<String>,
    pub line: Option<u32>,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Item {
    pub id: String,
    pub kind: String,
    pub status: Status,
    pub duration_ms: u64,
    pub reason: String,
    pub findings: Vec<Finding>,
    pub exit_code: Option<i32>,
    pub stdout: Option<Capture>,
    pub stderr: Option<Capture>,
}
impl Item {
    pub fn simple(id: &str, kind: &str, status: Status, reason: String) -> Self {
        Self {
            id: id.into(),
            kind: kind.into(),
            status,
            duration_ms: 0,
            reason,
            findings: vec![],
            exit_code: None,
            stdout: None,
            stderr: None,
        }
    }
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Report {
    pub schema_version: u32,
    pub timestamp: String,
    pub repository: String,
    pub project: String,
    pub provider: String,
    pub overall: Status,
    pub snapshot: Snapshot,
    pub results: Vec<Item>,
    pub git_summary: String,
    pub max_display_files: usize,
    pub token_usage: Option<u64>,
    #[serde(default)]
    pub provider_usage: Option<TokenUsage>,
    #[serde(default)]
    pub checks_only: bool,
}
pub fn overall(items: &[Item]) -> Status {
    if items.iter().any(|x| x.status == Status::Error) {
        Status::Error
    } else if items.iter().any(|x| x.status == Status::Fail) {
        Status::Fail
    } else if items.iter().any(|x| x.status == Status::NotRun) {
        Status::Error
    } else {
        Status::Pass
    }
}
pub fn write(root: &Path, report: &Report) -> Result<()> {
    let directory = crate::config::inside(root, Path::new(".Hannes/reports"))?;
    // Reports must be local regular files, never an output symlink into another directory.
    ensure!(
        directory == root.join(".Hannes/reports"),
        "reports directory cannot be a symlink"
    );
    let mut temporary = tempfile::NamedTempFile::new_in(&directory)?;
    serde_json::to_writer_pretty(&mut temporary, report)?;
    temporary.write_all(b"\n")?;
    temporary.as_file().sync_all()?;
    temporary.persist(directory.join("last.json"))?;
    #[cfg(unix)]
    std::fs::File::open(&directory)?.sync_all()?;
    Ok(())
}
pub fn read(root: &Path) -> Result<Report> {
    let path = crate::config::inside(root, Path::new(".Hannes/reports/last.json"))?;
    let report: Report =
        serde_json::from_str(&crate::config::read_bounded(&path, 128 * 1024 * 1024)?)?;
    ensure!(
        report.schema_version == 1,
        "unsupported report schema {}; rerun hannes mach mal",
        report.schema_version
    );
    ensure!(
        report.repository == root.canonicalize()?.to_string_lossy(),
        "report belongs to a different repository; rerun hannes mach mal"
    );
    ensure!(
        report.overall == overall(&report.results),
        "inconsistent report overall status"
    );
    ensure!(
        !report.checks_only
            || (report.provider_usage.is_none()
                && report.token_usage.is_none()
                && !report.results.iter().any(|item| item.kind == "review")),
        "inconsistent checks-only report scope"
    );
    Ok(report)
}
