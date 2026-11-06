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
    #[serde(default)]
    pub plan: Option<crate::planner::Plan>,
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
    let directory = crate::config::inside(root, Path::new(".hannes/reports"))?;
    // Reports must be local regular files, never an output symlink into another directory.
    ensure!(
        directory == root.join(".hannes/reports"),
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
    let path = crate::config::inside(root, Path::new(".hannes/reports/last.json"))?;
    let report: Report =
        serde_json::from_str(&crate::config::read_bounded(&path, 128 * 1024 * 1024)?)?;
    ensure!(
        matches!(report.schema_version, 1 | 2),
        "unsupported report schema {}; rerun hannes mach mal",
        report.schema_version
    );
    if report.schema_version == 2 {
        let plan = report
            .plan
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("schema 2 report requires a plan"))?;
        ensure!(
            plan.plan_schema_version == 1
                && matches!(plan.config_version, 1 | 2)
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
        let required: std::collections::BTreeSet<_> = plan.required_ids.iter().collect();
        ensure!(
            if plan.gate {
                !required.is_empty()
                    && required.len() == plan.required_ids.len()
                    && required == plan.scheduled.iter().map(|r| &r.id).collect()
                    && plan.omitted.is_empty()
                    && plan.selection_complete
            } else {
                required.is_empty()
            },
            "inconsistent required gate rules"
        );
        ensure!(
            report.overall != Status::Pass || (plan.executable() && plan.input.complete),
            "blocked/incomplete plan cannot pass"
        );
        if plan.gate && report.overall == Status::Pass {
            ensure!(
                plan.executable()
                    && plan.selection_complete
                    && !plan.required_ids.is_empty()
                    && plan.required_ids.iter().all(|id| report
                        .results
                        .iter()
                        .any(|r| &r.id == id && r.status == Status::Pass)),
                "incomplete gate cannot pass"
            );
        }
    } else {
        ensure!(
            report.plan.is_none(),
            "legacy report cannot certify a profile plan"
        );
    }
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
