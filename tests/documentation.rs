//! CFG-001: the public reference and executable examples must follow Serde's contract.
use hannes::{
    config, planner,
    provider::{Provider, Request},
    report::Status,
};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;
use std::{collections::BTreeMap, fs, process::Command};
use tempfile::TempDir;

const README: &str = include_str!("../README.md");
fn section<'a>(text: &'a str, title: &str) -> &'a str {
    text.split_once(&format!("### {title}\n"))
        .unwrap()
        .1
        .split("\n### ")
        .next()
        .unwrap()
}
fn rows(text: &str) -> BTreeMap<String, String> {
    text.lines()
        .filter_map(|line| {
            let cells: Vec<_> = line.split('|').map(str::trim).collect();
            let key = cells.get(1)?.strip_prefix('`')?.strip_suffix('`')?;
            Some((key.to_owned(), cells.get(2)?.to_string()))
        })
        .collect()
}
fn field_paths(value: &Value, parent: Vec<String>, paths: &mut Vec<Vec<String>>) {
    for (key, child) in value.as_object().unwrap() {
        let mut path = parent.clone();
        path.push(key.clone());
        paths.push(path.clone());
        if child.is_object() {
            field_paths(child, path, paths);
        }
    }
}
fn contract<T: Serialize + DeserializeOwned>(sample: &T, text: &str, prefix: &str) {
    let value = serde_json::to_value(sample).unwrap();
    let documented = rows(text);
    let fields = value.as_object().unwrap();
    for key in documented.keys().filter_map(|key| key.strip_prefix(prefix)) {
        if !key.contains('.') {
            assert!(
                fields.contains_key(key),
                "obsolete documented field: {prefix}{key}"
            );
        }
    }
    let mut paths = Vec::new();
    field_paths(&value, vec![], &mut paths);
    for path in paths {
        let key = path.last().unwrap();
        let label = format!("{prefix}{}", path.join("."));
        let pointer = format!("/{}", path.join("/"));
        let parent = if path.len() == 1 {
            String::new()
        } else {
            format!("/{}", path[..path.len() - 1].join("/"))
        };
        let cell = documented
            .get(&label)
            .unwrap_or_else(|| panic!("undocumented configuration field: {label}"));
        let mut omitted = value.clone();
        omitted
            .pointer_mut(&parent)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .remove(key);
        match serde_json::from_value::<T>(omitted) {
            Err(_) => assert!(cell.starts_with("Required"), "required field {label}"),
            Ok(parsed) => {
                assert!(cell.starts_with("Optional"), "optional field {label}");
                let defaults = serde_json::to_value(parsed).unwrap();
                let default = defaults.pointer(&pointer).unwrap();
                let expected = match (key.as_str(), default) {
                    ("default_profile", Value::Null) => Some("\"all\"".into()),
                    ("model" | "reasoning_effort", Value::Null) => Some("Codex default".into()),
                    (_, Value::Object(_)) => None, // Child fields are visited recursively.
                    _ => Some(default.to_string()),
                };
                if let Some(expected) = expected {
                    assert!(
                        cell.contains(&expected),
                        "wrong default for {label}: expected {expected}, found {cell}"
                    );
                }
            }
        }
    }
}
fn project() -> TempDir {
    let root = tempfile::tempdir().unwrap();
    assert!(
        Command::new("git")
            .args(["init", "--quiet"])
            .current_dir(root.path())
            .status()
            .unwrap()
            .success()
    );
    config::init(root.path()).unwrap();
    root
}
#[test]
fn every_configuration_field_required_marker_and_default_is_documented() {
    let root = project();
    let p = config::load(root.path()).unwrap();
    contract(&p.config, section(README, "Project settings"), "");
    let shared = section(README, "Fields shared by checks and reviews");
    contract(
        &p.checks[0],
        &format!("{}\n{shared}", section(README, "Add a check")),
        "",
    );
    contract(
        &p.reviews[0],
        &format!("{}\n{shared}", section(README, "Add a review")),
        "",
    );
    contract(
        &p.provider,
        section(README, "Configure the AI provider"),
        "",
    );
    contract(
        &p.provider.review_profile,
        section(README, "Configure the AI provider"),
        "review_profile.",
    );
    contract(
        &config::ReviewContext {
            path: "README.md".into(),
            headings: vec![],
        },
        section(README, "Add a review"),
        "context.",
    );
    contract(
        &config::Profile::all(),
        section(README, "Group rules into profiles"),
        "",
    );
}
#[test]
fn documentation_contract_detects_missing_fields_and_wrong_defaults() {
    let root = project();
    let p = config::load(root.path()).unwrap();
    let original = section(README, "Configure the AI provider");
    for bad in [
        original
            .lines()
            .filter(|line| !line.starts_with("| `model` |"))
            .collect::<Vec<_>>()
            .join("\n"),
        original.replace("Optional; `[]`", "Optional; `[\"all\"]`"),
        original.replace(
            "| `review_profile.tool_output_token_limit` |",
            "| `removed` |",
        ),
    ] {
        assert!(std::panic::catch_unwind(|| contract(&p.provider, &bad, "")).is_err());
    }
}
fn block<'a>(text: &'a str, kind: &str) -> &'a str {
    text.split_once(&format!("```{kind}\n"))
        .unwrap()
        .1
        .split_once("\n```")
        .unwrap()
        .0
}
struct Never;
impl Provider for Never {
    fn review(&self, _: &Request<'_>) -> anyhow::Result<String> {
        panic!("documentation tests must not call a provider")
    }
}
#[test]
fn readme_examples_validate_and_plan_without_ai_or_check_execution() {
    let root = project();
    assert_eq!(
        hannes::verify(root.path(), &Never).unwrap().overall,
        Status::Pass
    );
    let base = root.path().join(".hannes");
    for (title, relative) in [
        ("Project settings", "config.toml"),
        ("Add a check", "checks/unit-tests.toml"),
        ("Configure the AI provider", "provider.toml"),
        ("Add a review", "reviews/cli-help.toml"),
    ] {
        fs::write(base.join(relative), block(section(README, title), "toml")).unwrap();
    }
    fs::write(
        base.join("reviews/cli-help.md"),
        block(section(README, "Add a review"), "markdown"),
    )
    .unwrap();
    fs::create_dir(root.path().join("src")).unwrap();
    fs::write(root.path().join("src/main.rs"), "fn main() {}\n").unwrap();
    fs::write(
        root.path().join("README.md"),
        "# My_App\n\n## Usage\nRun My_App.\n",
    )
    .unwrap();
    let p = config::load(root.path()).unwrap();
    assert!(planner::build(&p, None, false).unwrap().plan.executable());
    let file = base.join("config.toml");
    fs::write(
        &file,
        fs::read_to_string(&file)
            .unwrap()
            .replace("version = 1", "version = 2")
            + "\ndefault_profile = \"local\"\n",
    )
    .unwrap();
    fs::create_dir(base.join("profiles")).unwrap();
    let profile = base.join("profiles/local.toml");
    fs::write(
        &profile,
        block(section(README, "Group rules into profiles"), "toml"),
    )
    .unwrap();
    let p = config::load(root.path()).unwrap();
    assert_eq!(
        planner::build(&p, None, false).unwrap().plan.scheduled[0].id,
        "unit-tests"
    );
    let check = base.join("checks/unit-tests.toml");
    fs::write(
        &check,
        fs::read_to_string(&check).unwrap() + "\ntags = [\"local\"]\n",
    )
    .unwrap();
    fs::write(
        &profile,
        fs::read_to_string(&profile)
            .unwrap()
            .replace("rules = [\"unit-tests\"]", "rules = []")
            .replace("include_tags = []", "include_tags = [\"local\"]"),
    )
    .unwrap();
    let provider = base.join("provider.toml");
    let allowed = section(README, "Configure the AI provider")
        .split("```toml\n")
        .nth(2)
        .unwrap()
        .split("\n```")
        .next()
        .unwrap();
    fs::write(
        &provider,
        fs::read_to_string(&provider)
            .unwrap()
            .replace("mcp_servers = []", allowed),
    )
    .unwrap();
    let p = config::load(root.path()).unwrap();
    let plan = planner::build(&p, None, false).unwrap().plan;
    assert!(plan.executable());
    assert_eq!(plan.provider.mcp_servers, ["browser", "project-memory"]);
    assert_eq!(plan.scheduled[0].id, "unit-tests");
}

#[test]
fn documented_bounds_and_provider_scaffold_match_the_implementation() {
    use hannes::limits::*;
    let text = README.split_whitespace().collect::<Vec<_>>().join(" ");
    let assertions = [
        format!("1..{MAX_DISPLAY_FILES}"),
        format!("1..{MAX_TIMEOUT_SECONDS}"),
        format!("1–{MAX_ID_BYTES} ASCII"),
        format!("up to {MAX_TAGS} unique labels"),
        format!("1–{MAX_TAG_BYTES} lowercase ASCII"),
        format!("1..{MAX_COMMAND_ARGS}"),
        format!("{MAX_ARGUMENT_BYTES} bytes"),
        format!("Up to {MAX_ENTRY_FILES} unique existing source files"),
        format!("Up to {MAX_CONTEXT_ENTRIES} document selections"),
        format!("Up to {MAX_CONTEXT_HEADINGS} nonblank exact Markdown"),
        format!(
            "{MAX_CONTEXT_FILES} distinct files, {} MiB per file and {} KiB",
            MAX_CONTEXT_FILE_BYTES / 1024 / 1024,
            MAX_CONTEXT_BYTES / 1024
        ),
        format!("{MIN_TOOL_OUTPUT_TOKENS}..{MAX_TOOL_OUTPUT_TOKENS}"),
        format!("at most {MAX_MCP_SERVERS} unique names, each 1–{MAX_MCP_NAME_BYTES}"),
        format!("more than {MAX_DISCOVERED_MCP_SERVERS} discovered servers"),
        format!(
            "{} KiB per TOML file; up to {MAX_RULES} checks, {MAX_RULES} reviews and {MAX_RULES} profiles",
            MAX_METADATA_BYTES / 1024
        ),
        format!(
            "{} KiB per instructions file; {} KiB combined",
            MAX_INSTRUCTION_BYTES / 1024,
            MAX_TOTAL_INSTRUCTION_BYTES / 1024
        ),
        format!(
            "Up to {MAX_MANIFEST_HINTS} file-list entries and {} KiB of diff",
            MAX_DIFF_BYTES / 1024
        ),
        format!(
            "{} MiB per file, {} MiB total",
            MAX_SNAPSHOT_FILE_BYTES / 1024 / 1024,
            MAX_SNAPSHOT_BYTES / 1024 / 1024
        ),
        format!("{},000 paths", MAX_SNAPSHOT_FILES / 1000),
        format!("nesting limited to {MAX_CONFIG_DEPTH} levels"),
        format!(
            "Git file enumeration capped at {} KiB",
            MAX_CAPTURE_BYTES / 1024
        ),
        format!(
            "{} MiB final JSON",
            MAX_PROVIDER_RESPONSE_BYTES / 1024 / 1024
        ),
        format!("{} MiB maximum", MAX_REPORT_BYTES / 1024 / 1024),
        format!("at most {MAX_SUMMARY_CHARS} characters"),
        format!("up to {MAX_FINDINGS} detailed findings"),
        format!("{MAX_FINDING_MESSAGE_BYTES} bytes each"),
    ];
    for expected in assertions {
        assert!(
            text.contains(&expected),
            "README bound missing or stale: {expected}"
        );
    }
    let example: config::ProviderConfig =
        toml::from_str(block(section(README, "Configure the AI provider"), "toml")).unwrap();
    assert_eq!(
        serde_json::to_value(example).unwrap(),
        serde_json::to_value(config::ProviderConfig::scaffold()).unwrap()
    );
    let root = project();
    assert_eq!(
        serde_json::to_value(config::load(root.path()).unwrap().provider).unwrap(),
        serde_json::to_value(config::ProviderConfig::scaffold()).unwrap()
    );
}
