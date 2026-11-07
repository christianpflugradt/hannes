use crate::review_contract::{ReviewResponse, VERDICTS, valid_summary};
use crate::{
    config::{Project, Review},
    git_snapshot::Snapshot,
    provider::{Provider, Request},
    report::{Item, MAX_SUMMARY_CHARS, Status, TokenUsage},
};
use anyhow::{Result, ensure};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
    time::Instant,
};
pub fn parse(
    project: &Project,
    active: &[&Review],
    snapshot: &Snapshot,
    json: &str,
) -> Result<Vec<Item>> {
    ensure!(
        json.len() as u64 <= crate::limits::MAX_PROVIDER_RESPONSE_BYTES,
        "provider JSON exceeds {} bytes",
        crate::limits::MAX_PROVIDER_RESPONSE_BYTES
    );
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
    let response: ReviewResponse = serde_json::from_str(json)?;
    let expected: BTreeSet<_> = active.iter().map(|r| r.id.as_str()).collect();
    let mut mapped = BTreeMap::new();
    for verdict in response.reviews {
        ensure!(
            expected.contains(verdict.id.as_str()),
            "unexpected review ID: {}",
            verdict.id
        );
        ensure!(VERDICTS.contains(&verdict.status), "invalid review status");
        ensure!(
            valid_summary(&verdict.reason),
            "review reason must be one nonempty line of at most {MAX_SUMMARY_CHARS} characters"
        );
        ensure!(
            verdict.findings.len() <= crate::limits::MAX_FINDINGS,
            "too many findings"
        );
        let mut grounded = false;
        for finding in &verdict.findings {
            ensure!(
                !finding.message.trim().is_empty()
                    && finding.message.len() <= crate::limits::MAX_FINDING_MESSAGE_BYTES,
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
                    let text =
                        crate::config::read_bounded(&full, crate::limits::MAX_SNAPSHOT_FILE_BYTES)?;
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
        item.blocking = active.iter().find(|r| r.id == verdict.id).unwrap().blocking;
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
    match crate::review_input::PreparedReviewInput::prepare(project, active, snapshot) {
        Ok(input) => execute_prepared(project, active, snapshot, &input, provider),
        Err(error) => (
            active
                .iter()
                .map(|r| Item::simple(&r.id, "review", Status::Error, format!("{error:#}")))
                .collect(),
            None,
        ),
    }
}
pub fn execute_prepared(
    project: &Project,
    active: &[&Review],
    snapshot: &Snapshot,
    input: &crate::review_input::PreparedReviewInput,
    provider: &dyn Provider,
) -> (Vec<Item>, Option<TokenUsage>) {
    let start = Instant::now();
    let response = provider.review_with_usage(&Request {
        project,
        reviews: active,
        snapshot,
        input,
    });
    let result = response
        .verdict
        .and_then(|json| parse(project, active, snapshot, &json));
    let mut items = match result {
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
    for item in &mut items {
        item.blocking = active.iter().find(|r| r.id == item.id).unwrap().blocking;
    }
    (items, response.usage)
}
