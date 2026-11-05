use crate::{
    config::{Project, Review},
    git_snapshot::Snapshot,
    provider::{Provider, Request},
    report::{Finding, Item, MAX_SUMMARY_CHARS, Status, TokenUsage},
};
use anyhow::{Result, ensure};
use serde::Deserialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
    time::Instant,
};
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Response {
    reviews: Vec<Verdict>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Verdict {
    id: String,
    status: Status,
    reason: String,
    findings: Vec<Finding>,
}
pub fn parse(
    project: &Project,
    active: &[&Review],
    snapshot: &Snapshot,
    json: &str,
) -> Result<Vec<Item>> {
    ensure!(json.len() <= 1024 * 1024, "provider JSON exceeds 1 MiB");
    // Options are nullable, but must still be present as required by the output schema.
    let value: serde_json::Value = serde_json::from_str(json)?;
    if let Some(reviews) = value.get("reviews").and_then(|x| x.as_array()) {
        for review in reviews {
            if let Some(findings) = review.get("findings").and_then(|x| x.as_array()) {
                for finding in findings {
                    ensure!(
                        finding.get("path").is_some() && finding.get("line").is_some(),
                        "finding must include path and line (nullable)"
                    );
                }
            }
        }
    }
    let response: Response = serde_json::from_str(json)?;
    let expected: BTreeSet<_> = active.iter().map(|r| r.id.as_str()).collect();
    let mut mapped = BTreeMap::new();
    for verdict in response.reviews {
        ensure!(
            expected.contains(verdict.id.as_str()),
            "unexpected review ID: {}",
            verdict.id
        );
        ensure!(
            matches!(verdict.status, Status::Pass | Status::Fail | Status::Error),
            "invalid review status"
        );
        ensure!(
            !verdict.reason.trim().is_empty()
                && verdict.reason.chars().count() <= MAX_SUMMARY_CHARS
                && !verdict
                    .reason
                    .chars()
                    .any(|c| c.is_control() || matches!(c, '\u{2028}' | '\u{2029}')),
            "review reason must be one nonempty line of at most {MAX_SUMMARY_CHARS} characters"
        );
        ensure!(verdict.findings.len() <= 20, "too many findings");
        let mut grounded = false;
        for finding in &verdict.findings {
            ensure!(
                !finding.message.trim().is_empty() && finding.message.len() <= 2000,
                "invalid finding message"
            );
            ensure!(finding.line != Some(0), "evidence line must be positive");
            if let Some(path) = &finding.path {
                ensure!(
                    snapshot.files.contains_key(path),
                    "evidence path not in verified manifest: {path}"
                );
                let full = crate::config::inside(&project.root, Path::new(path))?;
                ensure!(full.is_file(), "evidence path must be a file");
                if let Some(line) = finding.line {
                    let text = crate::config::read_bounded(&full, 8 * 1024 * 1024)?;
                    ensure!(
                        (line as usize) <= text.lines().count(),
                        "evidence line beyond file: {path}:{line}"
                    );
                    grounded = true;
                }
            } else {
                ensure!(finding.line.is_none(), "line requires evidence path");
            }
        }
        ensure!(
            verdict.status == Status::Error || grounded,
            "PASS/FAIL require at least one evidence path and line"
        );
        let mut item = Item::simple(&verdict.id, "review", verdict.status, verdict.reason);
        item.findings = verdict.findings;
        ensure!(
            mapped.insert(verdict.id.clone(), item).is_none(),
            "duplicate review ID: {}",
            verdict.id
        );
    }
    ensure!(
        mapped.len() == expected.len(),
        "incomplete review ID coverage"
    );
    Ok(mapped.into_values().collect())
}
pub fn execute(
    project: &Project,
    active: &[&Review],
    snapshot: &Snapshot,
    provider: &dyn Provider,
) -> Vec<Item> {
    execute_with_usage(project, active, snapshot, provider).0
}
pub fn execute_with_usage(
    project: &Project,
    active: &[&Review],
    snapshot: &Snapshot,
    provider: &dyn Provider,
) -> (Vec<Item>, Option<TokenUsage>) {
    let start = Instant::now();
    let response = provider.review_with_usage(&Request {
        project,
        reviews: active,
        snapshot,
    });
    let result = response
        .verdict
        .and_then(|json| parse(project, active, snapshot, &json));
    let items = match result {
        Ok(mut items) => {
            for item in &mut items {
                item.duration_ms = start.elapsed().as_millis() as u64;
            }
            items
        }
        Err(error) => active
            .iter()
            .map(|r| {
                let mut item = Item::simple(&r.id, "review", Status::Error, format!("{error:#}"));
                item.duration_ms = start.elapsed().as_millis() as u64;
                item
            })
            .collect(),
    };
    (items, response.usage)
}
