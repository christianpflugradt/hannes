use crate::limits::*;
use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, OpenOptions},
    io::{Read, Write},
    path::{Component, Path, PathBuf},
};

pub use crate::limits::{
    MAX_INSTRUCTION_BYTES as MAX_INSTRUCTIONS, MAX_METADATA_BYTES as MAX_METADATA,
};

pub fn path(relative: impl AsRef<Path>) -> PathBuf {
    // A trailing slash would make symlink_metadata follow a directory symlink.
    if relative.as_ref().as_os_str().is_empty() {
        PathBuf::from(CONFIG_DIRECTORY)
    } else {
        Path::new(CONFIG_DIRECTORY).join(relative)
    }
}

impl ProviderConfig {
    /// Explicit initialization choices; omitted optional fields still use Codex defaults.
    pub fn scaffold() -> Self {
        Self {
            provider: "codex-cli".into(),
            timeout_seconds: 180,
            model: Some("gpt-6.1-sol".into()),
            mcp_servers: vec![],
            review_profile: ReviewProfile {
                reasoning_effort: Some("medium".into()),
                ..ReviewProfile::default()
            },
        }
    }
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub version: u32,
    pub project: String,
    pub max_display_files: usize,
    #[serde(default)]
    pub execution_points: ExecutionPoints,
    pub hooks: Option<Hooks>,
}
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Hooks {
    #[serde(default)]
    pub git: BTreeMap<String, String>,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderConfig {
    pub provider: String,
    pub timeout_seconds: u64,
    pub model: Option<String>,
    #[serde(default)]
    pub mcp_servers: Vec<String>,
    #[serde(default)]
    pub review_profile: ReviewProfile,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct ReviewProfile {
    pub reasoning_effort: Option<String>,
    pub tool_output_token_limit: u32,
}
impl Default for ReviewProfile {
    fn default() -> Self {
        Self {
            reasoning_effort: None,
            tool_output_token_limit: DEFAULT_TOOL_OUTPUT_TOKENS,
        }
    }
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewContext {
    pub path: String,
    #[serde(default)]
    pub headings: Vec<String>,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Check {
    pub id: String,
    pub description: String,
    pub enabled: bool,
    pub command: String,
    #[serde(default = "default_directory")]
    pub working_directory: String,
    pub timeout_seconds: u64,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub run_at: Vec<String>,
}
fn default_directory() -> String {
    ".".into()
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Review {
    pub id: String,
    pub description: String,
    pub enabled: bool,
    pub instructions: String,
    #[serde(default)]
    pub files: Vec<String>,
    #[serde(default)]
    pub context: Vec<ReviewContext>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub run_at: Vec<String>,
    #[serde(skip)]
    pub text: String,
}
#[derive(Debug)]
pub struct Project {
    pub root: PathBuf,
    pub config: Config,
    pub provider: ProviderConfig,
    pub checks: Vec<Check>,
    pub reviews: Vec<Review>,
    pub rule_sets: Vec<RuleSet>,
    pub effective: crate::rules::EffectiveConfig,
    pub external_sources: BTreeMap<String, PathBuf>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionPoints {
    #[serde(default)]
    pub custom: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RuleSet {
    pub id: String,
    pub enabled: bool,
    pub rules: Vec<String>,
    #[serde(default)]
    pub run_at: Vec<String>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Definitions {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub checks: Vec<Check>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reviews: Vec<Review>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rule_sets: Vec<RuleSet>,
}

fn tag(value: &str) -> Result<()> {
    ensure!(
        !value.is_empty()
            && value.len() <= MAX_TAG_BYTES
            && value.as_bytes()[0].is_ascii_alphanumeric()
            && value
                .bytes()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || b"-_".contains(&c)),
        "invalid label {value:?}: use 1..{MAX_TAG_BYTES} lowercase ASCII letters, digits, - or _, starting with a letter or digit"
    );
    Ok(())
}
fn unique(values: &mut [String], label: &str) -> Result<()> {
    values.sort();
    ensure!(
        !values.windows(2).any(|pair| pair[0] == pair[1]),
        "duplicate {label}"
    );
    Ok(())
}
fn tags(values: &mut [String]) -> Result<()> {
    ensure!(values.len() <= MAX_TAGS, "at most {MAX_TAGS} tags per rule");
    for value in values.iter() {
        tag(value)?;
    }
    unique(values, "rule tag")
}
pub fn relative(path: &Path) -> Result<()> {
    ensure!(
        !path.as_os_str().is_empty() && !path.is_absolute(),
        "path must be nonempty and relative: {}",
        path.display()
    );
    ensure!(
        path.components()
            .all(|c| matches!(c, Component::Normal(_) | Component::CurDir)),
        "path cannot traverse parents: {}",
        path.display()
    );
    Ok(())
}
pub fn inside(root: &Path, path: &Path) -> Result<PathBuf> {
    relative(path)?;
    let root = root.canonicalize()?;
    let p = root
        .join(path)
        .canonicalize()
        .with_context(|| format!("missing path {}", path.display()))?;
    ensure!(
        p.starts_with(root),
        "path escapes project: {}",
        path.display()
    );
    Ok(p)
}
pub fn read_bounded(path: &Path, limit: u64) -> Result<String> {
    ensure!(
        path.metadata()?.is_file(),
        "{} must be a regular file",
        path.display()
    );
    let mut bytes = Vec::new();
    fs::File::open(path)?
        .take(limit + 1)
        .read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() as u64 <= limit,
        "{} exceeds {} bytes",
        path.display(),
        limit
    );
    String::from_utf8(bytes).context("file must be UTF-8")
}
fn parse<T: serde::de::DeserializeOwned>(root: &Path, path: &Path) -> Result<T> {
    let full = inside(root, path)?;
    let text = read_bounded(&full, MAX_METADATA)?;
    toml::from_str(&text).with_context(|| format!("invalid {}", path.display()))
}
fn timeout(value: u64) -> Result<()> {
    ensure!(
        (1..=MAX_TIMEOUT_SECONDS).contains(&value),
        "timeout_seconds must be 1..={MAX_TIMEOUT_SECONDS}"
    );
    Ok(())
}
fn nonempty(value: &str, name: &str) -> Result<()> {
    ensure!(
        !value.trim().is_empty() && !value.contains('\0'),
        "{name} must be nonempty and contain no NUL"
    );
    Ok(())
}
fn identity(id: &str, description: &str, ids: &mut BTreeSet<String>) -> Result<()> {
    ensure!(
        !id.is_empty()
            && id.len() <= MAX_ID_BYTES
            && id
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b"-_".contains(&c)),
        "invalid id {id:?}: use 1..{MAX_ID_BYTES} ASCII letters, digits, - or _"
    );
    ensure!(
        !id.starts_with("hannes-"),
        "IDs beginning hannes- are reserved"
    );
    ensure!(ids.insert(id.to_owned()), "duplicate id: {id}");
    nonempty(description, "description")
}
pub fn load(root: &Path) -> Result<Project> {
    crate::external::load(root, false, None)
}

/// Refresh external sources and publish resolution state only after full validation.
pub fn refresh(root: &Path) -> Result<Project> {
    crate::external::load(root, true, None)
}

pub(crate) fn validate(
    root: PathBuf,
    loaded: crate::discovery::Loaded,
    source_root: Option<&Path>,
) -> Result<Project> {
    let config = loaded.config;
    ensure!(
        config.version == 3,
        ".hannes/config.toml: unsupported configuration version {}; migrate to version 3 rules and rule sets",
        config.version
    );
    nonempty(&config.project, "project").context(".hannes/config.toml")?;
    ensure!(
        (1..=MAX_DISPLAY_FILES).contains(&config.max_display_files),
        ".hannes/config.toml: max_display_files must be 1..={MAX_DISPLAY_FILES}"
    );
    let provider: ProviderConfig = parse(&root, &self::path("provider.toml"))?;
    ensure!(
        provider.provider == "codex-cli",
        "unsupported provider: {}; expected codex-cli",
        provider.provider
    );
    timeout(provider.timeout_seconds).context("provider.toml")?;
    if let Some(model) = &provider.model {
        nonempty(model, "model")?;
    }
    let profile = &provider.review_profile;
    ensure!(
        (MIN_TOOL_OUTPUT_TOKENS..=MAX_TOOL_OUTPUT_TOKENS)
            .contains(&profile.tool_output_token_limit),
        "tool_output_token_limit must be {MIN_TOOL_OUTPUT_TOKENS}..={MAX_TOOL_OUTPUT_TOKENS}"
    );
    if let Some(effort) = &profile.reasoning_effort {
        ensure!(
            [
                "none", "minimal", "low", "medium", "high", "xhigh", "max", "ultra"
            ]
            .contains(&effort.as_str()),
            "unsupported reasoning_effort"
        );
    }
    ensure!(
        provider.mcp_servers.len() <= MAX_MCP_SERVERS,
        "at most {MAX_MCP_SERVERS} allowed MCP servers"
    );
    let mut servers = BTreeSet::new();
    for server in &provider.mcp_servers {
        nonempty(server, "MCP server name")?;
        ensure!(
            server.len() <= MAX_MCP_NAME_BYTES
                && server
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || b"_-".contains(&byte))
                && servers.insert(server),
            "invalid or duplicate MCP server name"
        );
    }
    let mut ids = BTreeSet::new();
    let mut checks = Vec::new();
    for (path, mut check) in loaded.checks {
        (|| -> Result<()> {
            identity(&check.id, &check.description, &mut ids)?;
            tags(&mut check.tags)?;
            timeout(check.timeout_seconds)?;
            let arguments = crate::check_command::parse(&check.command)
                .with_context(|| format!("Invalid command in check '{}'", check.id))?;
            let exe = Path::new(&arguments[0]);
            if exe.is_absolute() || arguments[0].contains('/') || arguments[0].contains('\\') {
                relative(exe)?;
                let base = Path::new(&check.working_directory).join(exe);
                // Validate containment of existing ancestors without requiring the
                // executable (or its parent directories) to exist in this environment.
                let mut ancestor = base.as_path();
                while root
                    .join(ancestor)
                    .symlink_metadata()
                    .is_err_and(|error| error.kind() == std::io::ErrorKind::NotFound)
                {
                    ancestor = ancestor
                        .parent()
                        .filter(|path| !path.as_os_str().is_empty())
                        .unwrap_or(Path::new("."));
                }
                inside(&root, ancestor)?;
            }
            ensure!(
                inside(&root, Path::new(&check.working_directory))?.is_dir(),
                "working_directory must be a directory"
            );
            Ok(())
        })()
        .with_context(|| format!("invalid check '{}' ({})", check.id, path.display()))?;
        checks.push(check);
    }
    let mut reviews = Vec::new();
    let mut instruction_bytes = 0;
    for (path, mut review) in loaded.reviews {
        (|| -> Result<()> {
            identity(&review.id, &review.description, &mut ids)?;
            tags(&mut review.tags)?;
            ensure!(
                review.files.len() <= MAX_ENTRY_FILES
                    && review.context.len() <= MAX_CONTEXT_ENTRIES,
                "review permits at most {MAX_ENTRY_FILES} files and {MAX_CONTEXT_ENTRIES} context entries"
            );
            let mut files = BTreeSet::new();
            for file in &mut review.files {
                let full = inside(&root, Path::new(file))?;
                ensure!(full.is_file(), "review file must be a regular file: {file}");
                *file = full
                    .strip_prefix(&root)?
                    .components()
                    .map(|part| part.as_os_str().to_string_lossy())
                    .collect::<Vec<_>>()
                    .join("/");
                ensure!(files.insert(file.clone()), "duplicate review file: {file}");
            }
            for source in &mut review.context {
                let full = inside(&root, Path::new(&source.path))?;
                ensure!(
                    full.is_file(),
                    "context must be a regular file: {}",
                    source.path
                );
                source.path = full
                    .strip_prefix(&root)?
                    .components()
                    .map(|part| part.as_os_str().to_string_lossy())
                    .collect::<Vec<_>>()
                    .join("/");
                ensure!(
                    source.headings.len() <= MAX_CONTEXT_HEADINGS,
                    "at most {MAX_CONTEXT_HEADINGS} headings per context entry"
                );
                for heading in &source.headings {
                    nonempty(heading, "context heading")?;
                }
            }
            let p = Path::new(&review.instructions);
            relative(p)?;
            ensure!(
                p.extension().is_some_and(|s| s == "md"),
                "instructions must reference Markdown (.md)"
            );
            let full = if let Some(source_root) = source_root {
                crate::external::source_path(source_root, &path.parent().unwrap().join(p))?
            } else {
                let full = inside(&root, &path.parent().unwrap().join(p))?;
                ensure!(full.starts_with(root.join(self::path("")).canonicalize()?),
                    "instructions must stay inside .hannes");
                full
            };
            review.text = read_bounded(&full, MAX_INSTRUCTIONS)?;
            nonempty(&review.text, "instructions")?;
            instruction_bytes += review.text.len();
            ensure!(
                instruction_bytes <= MAX_TOTAL_INSTRUCTION_BYTES,
                "total review instructions exceed {} bytes", MAX_TOTAL_INSTRUCTION_BYTES
            );
            Ok(())
        })()
        .with_context(|| format!("invalid review '{}' ({})", review.id, path.display()))?;
        reviews.push(review);
    }
    checks.sort_by(|a, b| a.id.cmp(&b.id));
    reviews.sort_by(|a, b| a.id.cmp(&b.id));
    crate::review_context::collect(&root, &reviews.iter().collect::<Vec<_>>())?;
    let effective = crate::rules::resolve(
        &config.execution_points,
        &loaded.assignments,
        &loaded.rule_sets,
    )?;
    if let Some(hooks) = &config.hooks {
        for (event, point) in &hooks.git {
            ensure!(
                crate::reconciliation::GIT_EVENTS.contains(&event.as_str()),
                ".hannes/config.toml: hooks.git: unsupported Git hook: {event}"
            );
            ensure!(
                effective.points.contains(point),
                ".hannes/config.toml: hooks.git.{event}: unknown execution point: {point}"
            );
        }
    }
    let rule_sets = loaded.rule_sets.into_iter().map(|(_, set)| set).collect();
    Ok(Project {
        root,
        config,
        provider,
        checks,
        reviews,
        rule_sets,
        effective,
        external_sources: BTreeMap::new(),
    })
}

pub fn init(root: &Path) -> Result<usize> {
    let root = root.canonicalize()?;
    crate::git_snapshot::require_root(&root)?;
    for relative in ["", "checks", "reviews", "reports"] {
        let p = root.join(path(relative));
        if p.symlink_metadata().is_ok() {
            ensure!(
                !p.symlink_metadata()?.file_type().is_symlink() && p.is_dir(),
                "refusing non-directory or symlink {}",
                p.display()
            );
        } else {
            fs::create_dir(&p)?;
        }
    }
    let project = root
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    let config = Config {
        version: 3,
        project,
        max_display_files: MAX_DISPLAY_FILES,
        execution_points: ExecutionPoints::default(),
        hooks: None,
    };
    let check = |id: &str, description: &str, enabled: bool, argument: &str| Check {
        id: id.into(),
        description: description.into(),
        enabled,
        command: format!("git {argument}"),
        working_directory: default_directory(),
        timeout_seconds: 30,
        tags: vec![],
        run_at: vec!["pre-push".into()],
    };
    let review = Review {
        id: "product-consistency".into(),
        description: "Evaluate behavior against documented expectations".into(),
        enabled: false,
        instructions: "product-consistency.md".into(),
        files: vec![],
        context: vec![],
        tags: vec![],
        run_at: vec!["pre-push".into()],
        text: String::new(),
    };
    let files = [
        ("config.toml", scaffold(&config)?),
        ("provider.toml", scaffold(&ProviderConfig::scaffold())?),
        ("checks/always-ok.toml", check_scaffold(check("always-ok", "Example deterministic pass", true, "--version"))?),
        ("checks/deliberately-failing.toml", check_scaffold(check("deliberately-failing", "Disabled example demonstrating failure", false, "definitely-not-a-subcommand"))?),
        ("reviews/product-consistency.toml", scaffold(&Definitions { reviews: vec![review], ..Definitions::default() })?),
        ("reviews/product-consistency.md", "Evaluate the current working files against expectations explicitly documented in this project. Inspect relevant changed code and documentation. Do not invent requirements. Cite concrete file paths and line numbers. State uncertainty as ERROR when necessary evidence is unavailable. Report FAIL for a grounded contradiction, PASS only with grounded reasons. Customize this guidance before enabling.\n".into()),
        ("reports/.gitignore", "*\n!.gitignore\n".into()),
        ("README.md", include_str!("../docs/configuration.md").into()),
    ];
    let mut created = 0;
    for (path, content) in files {
        let full = root.join(self::path(path));
        match OpenOptions::new().write(true).create_new(true).open(&full) {
            Ok(mut f) => {
                f.write_all(content.as_bytes())?;
                created += 1;
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(e) => bail!("cannot create {}: {e}", full.display()),
        }
    }
    Ok(created)
}

/// Serialize real configuration types while omitting empty optional rule fields.
fn scaffold(value: &impl Serialize) -> Result<String> {
    let mut table = toml::Value::try_from(value)?;
    fn trim(value: &mut toml::Value) {
        match value {
            toml::Value::Table(table) => {
                for key in ["tags", "files", "context"] {
                    if table
                        .get(key)
                        .and_then(toml::Value::as_array)
                        .is_some_and(Vec::is_empty)
                    {
                        table.remove(key);
                    }
                }
                for (_, child) in table.iter_mut() {
                    trim(child);
                }
            }
            toml::Value::Array(array) => {
                for child in array {
                    trim(child);
                }
            }
            _ => {}
        }
    }
    trim(&mut table);
    Ok(toml::to_string(&table)?)
}

fn check_scaffold(check: Check) -> Result<String> {
    scaffold(&Definitions {
        checks: vec![check],
        ..Definitions::default()
    })
}
