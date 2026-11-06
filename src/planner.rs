//! Offline rule selection and bounded preparation, shared by preview and execution.
use crate::{
    config::{Profile, Project},
    git_snapshot::{self, Snapshot},
    review_context,
};
use anyhow::{Result, anyhow};
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
    pub profile: String,
    pub profile_source: String,
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
    let parts: Vec<_> = path.split('/').collect();
    parts.len() == 3
        && parts[0] == crate::limits::CONFIG_DIRECTORY
        && matches!(parts[1], "checks" | "reviews" | "profiles")
        && parts[2].ends_with(".toml")
}

pub fn build(
    project: &Project,
    requested: Option<&str>,
    checks_only: bool,
) -> Result<PreparedPlan> {
    let name = requested
        .or(project.config.default_profile.as_deref())
        .unwrap_or("all");
    let profile = if name == "all" {
        Profile::all()
    } else {
        project
            .profiles
            .iter()
            .find(|p| p.id == name)
            .cloned()
            .ok_or_else(|| anyhow!("unknown profile: {name}"))?
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
        if !profile.selects(id, tags) {
            continue;
        }
        let mut selected_by = Vec::new();
        if profile.id == "all" {
            selected_by.push("all".into());
        }
        if profile.rules.contains(id) {
            selected_by.push(format!("id:{id}"));
        }
        for tag in tags {
            if profile.include_tags.contains(tag) {
                selected_by.push(format!("tag:{tag}"));
            }
        }
        let rule = Rule {
            id: id.clone(),
            kind: kind.into(),
            description: description.clone(),
            tags: tags.clone(),
            selected_by,
        };
        if profile.gate {
            required_ids.push(id.clone());
        }
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
    if profile.gate && (!selection_complete || !omitted.is_empty() || required_ids.is_empty()) {
        blocking_reasons.push(
            "Gate requires every selected rule; checks-only cannot omit required reviews".into(),
        );
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
    let mut excerpts = Vec::new();
    if !active.is_empty() {
        match review_context::collect(&project.root, &active) {
            Ok(found) => {
                context.selected_bytes = found.iter().map(|e| e.text.len()).sum();
                context.sections = found
                    .iter()
                    .map(|e| ContextSection {
                        path: e.path.clone(),
                        start_line: e.start_line,
                        end_line: e.end_line,
                        bytes: e.text.len(),
                    })
                    .collect();
                excerpts = found;
            }
            Err(error) => {
                blocking_reasons.push(format!("Cannot prepare review context: {error:#}"))
            }
        }
        for path in active
            .iter()
            .flat_map(|r| r.files.iter().chain(r.context.iter().map(|c| &c.path)))
        {
            if !snapshot.files.contains_key(path) {
                blocking_reasons.push(format!("Review evidence not in verified manifest: {path}"));
            }
        }
        let focused = active.iter().all(|r| !r.files.is_empty()).then(|| {
            active
                .iter()
                .flat_map(|r| r.files.iter().chain(r.context.iter().map(|c| &c.path)))
                .cloned()
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>()
        });
        match git_snapshot::bounded_diff_for_paths(
            &project.root,
            snapshot.head.is_some(),
            focused.as_deref(),
        ) {
            Ok((diff, truncated)) => {
                context.orientation_diff_bytes = diff.len();
                context.orientation_diff_truncated = truncated;
            }
            Err(error) => {
                blocking_reasons.push(format!("Cannot prepare orientation diff: {error:#}"))
            }
        }
    }
    let mut provider = project.provider.clone();
    provider.mcp_servers.sort();
    let mut plan = Plan {
        plan_schema_version: 1,
        config_version: project.config.version,
        plan_id: String::new(),
        status: if blocking_reasons.is_empty() {
            "planned"
        } else {
            "blocked"
        }
        .into(),
        blocking_reasons,
        profile: profile.id.clone(),
        profile_source: if requested.is_some() {
            "explicit"
        } else if project.config.default_profile.is_some() {
            "default"
        } else {
            "implicit"
        }
        .into(),
        gate: profile.gate,
        input: Input {
            kind: profile.input.clone(),
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
        .filter(|r| profile.selects(&r.id, &r.tags))
        .cloned()
        .collect();
    for check in &mut checks {
        check.tags.sort();
    }
    let mut reviews: Vec<_> = project
        .reviews
        .iter()
        .filter(|r| profile.selects(&r.id, &r.tags))
        .cloned()
        .collect();
    for review in &mut reviews {
        review.files.sort();
        review.tags.sort();
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
    let identity = serde_json::json!({
        "plan_schema_version": 1, "verifier_version": env!("CARGO_PKG_VERSION"),
        "config_version": project.config.version, "project": project.config.project,
        "profile": profile, "checks_only": checks_only, "checks": checks, "reviews": reviews,
        "provider": plan.provider, "head": snapshot.head, "files": files, "context": excerpts,
        "complete": snapshot.complete,
    });
    plan.plan_id = format!("{:x}", Sha256::digest(serde_json::to_vec(&identity)?));
    Ok(PreparedPlan { plan, snapshot })
}

pub fn apply(project: &mut Project, plan: &Plan) {
    project.checks.retain(|r| plan.schedules(&r.id));
    project.reviews.retain(|r| plan.schedules(&r.id));
}
