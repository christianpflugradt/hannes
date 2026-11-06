//! Offline rule selection and bounded preparation, shared by preview and execution.
use crate::{
    config::Project,
    git_snapshot::{self, Snapshot},
    review_context,
};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rule {
    pub id: String,
    pub kind: String,
    pub description: String,
    pub tags: Vec<String>,
    pub selected_by: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origin: Option<crate::rules::EffectiveRule>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Omission {
    pub rule: Rule,
    pub reason: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Input {
    pub kind: String,
    pub head: Option<String>,
    pub fingerprint: String,
    pub complete: bool,
    pub limitations: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContextSection {
    pub path: String,
    pub start_line: usize,
    pub end_line: usize,
    pub bytes: usize,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContextPlan {
    pub entry_files: Vec<String>,
    pub selections: Vec<crate::config::ReviewContext>,
    pub sections: Vec<ContextSection>,
    pub selected_bytes: usize,
    pub byte_limit: usize,
    pub instruction_bytes: usize,
    pub instruction_byte_limit: usize,
    pub manifest_hint_limit: usize,
    pub orientation_diff_bytes: usize,
    pub orientation_diff_truncated: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Plan {
    pub plan_schema_version: u32,
    pub config_version: u32,
    pub plan_id: String,
    pub status: String,
    pub blocking_reasons: Vec<String>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub at: String,
    // Historical report metadata only; never used for current selection.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub profile: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub profile_source: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub gate: bool,
    pub input: Input,
    pub checks_only: bool,
    pub selection_complete: bool,
    pub scheduled: Vec<Rule>,
    pub omitted: Vec<Omission>,
    pub required_ids: Vec<String>,
    pub provider: crate::config::ProviderConfig,
    pub provider_availability_checked: bool,
    pub planned_model_calls: usize,
    pub context: ContextPlan,
}
impl Plan {
    pub fn executable(&self) -> bool {
        self.status == "planned"
    }
    pub fn schedules(&self, id: &str) -> bool {
        self.scheduled.iter().any(|r| r.id == id)
    }
}
pub struct PreparedPlan {
    pub plan: Plan,
    pub snapshot: Snapshot,
    pub review_input: Option<crate::review_input::PreparedReviewInput>,
}

fn metadata_path(path: &str) -> bool {
    if [
        crate::config::path("config.toml"),
        crate::config::path("provider.toml"),
    ]
    .iter()
    .any(|p| p == std::path::Path::new(path))
    {
        return true;
    }
    path.starts_with(".hannes/") && path.ends_with(".toml")
}

pub fn build(
    project: &Project,
    requested: Option<&str>,
    checks_only: bool,
) -> Result<PreparedPlan> {
    let name = requested.unwrap_or("pre-push");
    anyhow::ensure!(
        project.effective.points.contains(name),
        "unknown execution point: {name}"
    );
    let selects = |id: &str| {
        project
            .effective
            .rules
            .get(id)
            .is_some_and(|r| r.execution_points.contains(name))
    };
    let mut scheduled = Vec::new();
    let mut omitted = Vec::new();
    let mut required_ids = Vec::new();
    let mut blocking_reasons = Vec::new();
    for (id, kind, description, tags, enabled) in project
        .checks
        .iter()
        .map(|r| (&r.id, "check", &r.description, &r.tags, r.enabled))
        .chain(
            project
                .reviews
                .iter()
                .map(|r| (&r.id, "review", &r.description, &r.tags, r.enabled)),
        )
    {
        if !selects(id) {
            continue;
        }
        let origin = project.effective.rules.get(id).cloned();
        let selected_by = vec![format!("at:{name}")];
        let rule = Rule {
            id: id.clone(),
            kind: kind.into(),
            description: description.clone(),
            tags: tags.clone(),
            selected_by,
            origin,
        };
        required_ids.push(id.clone());
        if !enabled {
            omitted.push(Omission {
                rule,
                reason: "disabled".into(),
            });
        } else if checks_only && kind == "review" {
            omitted.push(Omission {
                rule,
                reason: "checks_only".into(),
            });
        } else {
            scheduled.push(rule);
        }
    }
    let selection_complete = !omitted.iter().any(|r| r.reason == "checks_only");
    if !selection_complete {
        blocking_reasons.push("Execution point requires every selected rule; checks-only cannot omit required reviews".into());
    }
    let snapshot = git_snapshot::capture(&project.root).unwrap_or_else(|error| Snapshot {
        head: None,
        fingerprint: String::new(),
        files: Default::default(),
        complete: false,
        limitations: vec![format!("{error:#}")],
    });
    if !snapshot.complete {
        blocking_reasons.push("Input snapshot is incomplete; no verification can start".into());
    }
    let active: Vec<_> = project
        .reviews
        .iter()
        .filter(|r| scheduled.iter().any(|p| p.id == r.id))
        .collect();
    let mut context = ContextPlan {
        entry_files: active
            .iter()
            .flat_map(|r| r.files.iter().cloned())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect(),
        selections: active
            .iter()
            .flat_map(|r| r.context.iter().cloned())
            .collect(),
        sections: vec![],
        selected_bytes: 0,
        byte_limit: review_context::MAX_CONTEXT_BYTES,
        instruction_bytes: active.iter().map(|r| r.text.len()).sum(),
        instruction_byte_limit: crate::limits::MAX_TOTAL_INSTRUCTION_BYTES,
        manifest_hint_limit: crate::limits::MAX_MANIFEST_HINTS,
        orientation_diff_bytes: 0,
        orientation_diff_truncated: false,
    };
    for section in &mut context.selections {
        section.headings.sort();
        section.headings.dedup();
    }
    context
        .selections
        .sort_by(|a, b| (&a.path, &a.headings).cmp(&(&b.path, &b.headings)));
    context
        .selections
        .dedup_by(|a, b| a.path == b.path && a.headings == b.headings);
    let mut review_input = None;
    if !active.is_empty() {
        match crate::review_input::PreparedReviewInput::prepare(project, &active, &snapshot) {
            Ok(input) => {
                context.selected_bytes = input.excerpts.iter().map(|e| e.text.len()).sum();
                context.sections = input
                    .excerpts
                    .iter()
                    .map(|e| ContextSection {
                        path: e.path.clone(),
                        start_line: e.start_line,
                        end_line: e.end_line,
                        bytes: e.text.len(),
                    })
                    .collect();
                context.orientation_diff_bytes = input.diff.len();
                context.orientation_diff_truncated = input.diff_truncated;
                review_input = Some(input);
            }
            Err(error) => blocking_reasons.push(format!("Cannot prepare review input: {error:#}")),
        }
    }
    let mut provider = project.provider.clone();
    provider.mcp_servers.sort();
    let mut plan = Plan {
        plan_schema_version: 2,
        config_version: project.config.version,
        plan_id: String::new(),
        status: if blocking_reasons.is_empty() {
            "planned"
        } else {
            "blocked"
        }
        .into(),
        blocking_reasons,
        at: name.into(),
        profile: String::new(),
        profile_source: String::new(),
        gate: false,
        input: Input {
            kind: "worktree".into(),
            head: snapshot.head.clone(),
            fingerprint: snapshot.fingerprint.clone(),
            complete: snapshot.complete,
            limitations: snapshot.limitations.clone(),
        },
        checks_only,
        selection_complete,
        scheduled,
        omitted,
        required_ids,
        provider,
        provider_availability_checked: false,
        planned_model_calls: usize::from(!active.is_empty()),
        context,
    };
    // Normalize metadata separately from raw file hashes so TOML formatting, file
    // names and selector order do not change semantic identity. The report still
    // records the exact raw snapshot for stability/freshness comparisons.
    let mut checks: Vec<_> = project
        .checks
        .iter()
        .filter(|r| selects(&r.id))
        .cloned()
        .collect();
    for check in &mut checks {
        check.tags.sort();
        check.run_at.sort();
        check.command = shell_words::join(crate::check_command::parse(&check.command)?);
    }
    let mut reviews: Vec<_> = project
        .reviews
        .iter()
        .filter(|r| selects(&r.id))
        .cloned()
        .collect();
    for review in &mut reviews {
        review.files.sort();
        review.tags.sort();
        review.run_at.sort();
        for context in &mut review.context {
            context.headings.sort();
            context.headings.dedup();
        }
        review
            .context
            .sort_by(|a, b| (&a.path, &a.headings).cmp(&(&b.path, &b.headings)));
    }
    let files: std::collections::BTreeMap<_, _> = snapshot
        .files
        .iter()
        .filter(|(p, _)| !metadata_path(p))
        .collect();
    let excerpts = review_input
        .as_ref()
        .map_or(&[][..], |input| input.excerpts.as_slice());
    let identity = serde_json::json!({
        "plan_schema_version": 2, "verifier_version": env!("CARGO_PKG_VERSION"),
        "config_version": project.config.version, "project": project.config.project,
        "at": name, "checks_only": checks_only, "checks": checks, "reviews": reviews,
        "provider": plan.provider, "head": snapshot.head, "files": files, "context": excerpts,
        "complete": snapshot.complete,
    });
    plan.plan_id = format!("{:x}", Sha256::digest(serde_json::to_vec(&identity)?));
    Ok(PreparedPlan {
        plan,
        snapshot,
        review_input,
    })
}

pub fn apply(project: &mut Project, plan: &Plan) {
    project.checks.retain(|r| plan.schedules(&r.id));
    project.reviews.retain(|r| plan.schedules(&r.id));
}
