use crate::{git_snapshot::Snapshot, process::Capture};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::{io::Write, path::Path};

pub use crate::limits::MAX_SUMMARY_CHARS;

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
    pub blocking: bool,
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
            blocking: true,
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
    pub plan: crate::planner::Plan,
}
pub fn overall(items: &[Item]) -> Status {
    if items.iter().any(|x| x.status == Status::Error) {
        Status::Error
    } else if items.iter().any(|x| x.status == Status::Fail && x.blocking) {
        Status::Fail
    } else if items.iter().any(|x| x.status == Status::NotRun) {
        Status::Error
    } else {
        Status::Pass
    }
}
pub fn write(root: &Path, report: &Report) -> Result<()> {
    let target = root.join(crate::config::path("reports"));
    if target
        .symlink_metadata()
        .is_err_and(|e| e.kind() == std::io::ErrorKind::NotFound)
    {
        crate::config::source_path(root, &crate::config::path(""))?;
        std::fs::create_dir(&target)?;
    }
    let directory = crate::config::inside(root, &crate::config::path("reports"))?;
    // Reports must be local regular files, never an output symlink into another directory.
    ensure!(
        directory == root.join(crate::config::path("reports")),
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
    let path = crate::config::inside(root, &crate::config::path("reports/last.json"))?;
    let report: Report = serde_json::from_str(&crate::config::read_bounded(
        &path,
        crate::limits::MAX_REPORT_BYTES,
    )?)?;
    ensure!(
        report.schema_version == 4,
        "unsupported report schema; rerun hannes mach was"
    );
    let plan = &report.plan;
    ensure!(
        plan.plan_schema_version == 3
            && !plan.at.is_empty()
            && matches!(plan.status.as_str(), "planned" | "blocked")
            && plan.executable() == plan.blocking_reasons.is_empty()
            && plan.input.kind == "worktree"
            && plan.plan_id.len() == 64
            && plan.plan_id.bytes().all(|b| b.is_ascii_hexdigit())
            && plan.checks_only == report.checks_only
            && plan.input.fingerprint == report.snapshot.fingerprint
            && plan.input.head == report.snapshot.head
            && plan.input.complete == report.snapshot.complete,
        "inconsistent report plan/input"
    );
    let expected: std::collections::BTreeSet<_> =
        plan.scheduled.iter().map(|r| (&r.id, &r.kind)).collect();
    let actual: std::collections::BTreeSet<_> = report
        .results
        .iter()
        .filter(|r| r.kind != "integrity")
        .map(|r| (&r.id, &r.kind))
        .collect();
    ensure!(
        expected == actual
            && expected.len() == plan.scheduled.len()
            && plan
                .scheduled
                .iter()
                .all(|r| matches!(r.kind.as_str(), "check" | "review"))
            && report
                .results
                .iter()
                .filter(|r| r.kind != "integrity")
                .count()
                == actual.len(),
        "inconsistent report rule coverage"
    );
    ensure!(
        plan.scheduled.iter().all(|rule| rule
            .origin
            .as_ref()
            .is_some_and(|o| o.execution_points.contains(&plan.at))
            && report
                .results
                .iter()
                .any(|item| item.id == rule.id && item.blocking == rule.blocking)),
        "inconsistent report rule metadata"
    );
    let required: std::collections::BTreeSet<_> = plan.required_ids.iter().collect();
    ensure!(
        required.len() == plan.required_ids.len()
            && required == plan.scheduled.iter().map(|r| &r.id).collect()
            && plan.omitted.is_empty()
            && plan.selection_complete,
        "inconsistent required rules"
    );
    ensure!(
        report.overall != Status::Pass || (plan.executable() && plan.input.complete),
        "blocked/incomplete plan cannot pass"
    );
    ensure!(
        report.repository == root.canonicalize()?.to_string_lossy(),
        "report belongs to a different repository; rerun hannes mach was"
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
