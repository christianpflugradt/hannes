use crate::limits::*;
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Read,
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
    pub project: String,
    #[serde(default = "default_display")]
    pub max_display_files: usize,
    #[serde(default)]
    pub execution_points: ExecutionPoints,
    #[serde(default)]
    pub imports: Vec<crate::external::Import>,
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
/// A loaded check with an absolute entrypoint, checked for existence before execution.
#[derive(Debug, Clone, Serialize)]
pub struct Check {
    pub id: String,
    pub name: String,
    pub description: String,
    pub enabled: bool,
    pub blocking: bool,
    pub entrypoint: PathBuf,
    pub interpreter: Option<String>,
    pub timeout_seconds: u64,
    pub run_at: Vec<String>,
    pub parameters: crate::parameters::Binding,
}
#[derive(Debug, Clone, Serialize)]
pub struct Review {
    pub id: String,
    pub name: String,
    pub description: String,
    pub enabled: bool,
    pub blocking: bool,
    pub files: Vec<String>,
    pub context: Vec<ReviewContext>,
    pub run_at: Vec<String>,
    pub text: String,
    pub parameters: crate::parameters::Binding,
    pub execution_error: Option<String>,
}
pub(crate) fn default_true() -> bool {
    true
}
fn default_display() -> usize {
    MAX_DISPLAY_FILES
}
fn default_timeout() -> u64 {
    30
}

/// The single rule document format used by local and imported rules.
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RuleDefinition {
    pub id: String,
    pub name: Option<String>,
    pub description: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_true")]
    pub blocking: bool,
    #[serde(default)]
    pub run_at: Vec<String>,
    #[serde(default)]
    pub parameters: BTreeMap<String, crate::parameters::Declaration>,
    pub check: Option<CheckDefinition>,
    pub review: Option<ReviewDefinition>,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CheckDefinition {
    pub entrypoint: String,
    pub interpreter: Option<String>,
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewDefinition {
    pub prompt: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub files: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub context: Vec<ReviewContext>,
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
    pub description: String,
    pub rules: Vec<String>,
    pub run_at: Vec<String>,
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
/// Resolve a source resource without following any symlink component.
pub(crate) fn source_path(root: &Path, relative: &Path) -> Result<PathBuf> {
    self::relative(relative)?;
    let mut path = root.to_path_buf();
    ensure!(
        path.symlink_metadata()?.is_dir() && !path.symlink_metadata()?.file_type().is_symlink(),
        "source boundary must be a directory, not a symlink"
    );
    for part in relative.components() {
        if let Component::Normal(part) = part {
            path.push(part);
            let metadata = path.symlink_metadata().context("missing source path")?;
            ensure!(
                !metadata.file_type().is_symlink(),
                "source symlink traversal is unsupported"
            );
        }
    }
    ensure!(
        path.canonicalize()?.starts_with(root.canonicalize()?),
        "source escapes selected directory"
    );
    Ok(path)
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

pub(crate) fn validate(root: PathBuf, loaded: crate::discovery::Loaded) -> Result<Project> {
    let assignments: Vec<_> = loaded
        .checks
        .iter()
        .map(|(source, rule)| crate::rules::Assignment {
            id: rule.id.clone(),
            enabled: rule.enabled,
            run_at: rule.run_at.clone(),
            source: source.clone(),
        })
        .chain(
            loaded
                .reviews
                .iter()
                .map(|(source, rule)| crate::rules::Assignment {
                    id: rule.id.clone(),
                    enabled: rule.enabled,
                    run_at: rule.run_at.clone(),
                    source: source.clone(),
                }),
        )
        .collect();
    let config = loaded.config;
    nonempty(&config.project, "project").context(".hannes/config.toml")?;
    ensure!(
        (1..=MAX_DISPLAY_FILES).contains(&config.max_display_files),
        ".hannes/config.toml: max_display_files must be 1..={MAX_DISPLAY_FILES}"
    );
    let provider = match root.join(self::path("provider.toml")).symlink_metadata() {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => ProviderConfig::scaffold(),
        Err(e) => return Err(e.into()),
        Ok(_) => crate::discovery::document(&root, &self::path("provider.toml"))?,
    };
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
    for (path, check) in loaded.checks {
        (|| -> Result<()> {
            identity(&check.id, &check.description, &mut ids)?;
            timeout(check.timeout_seconds)?;
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
            if review.execution_error.is_none() {
                nonempty(&review.text, "instructions")?;
            }
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
    let effective =
        crate::rules::resolve(&config.execution_points, &assignments, &loaded.rule_sets)?;
    let rule_sets = loaded.rule_sets.into_iter().map(|(_, set)| set).collect();
    Ok(Project {
        root,
        config,
        provider,
        checks,
        reviews,
        rule_sets,
        effective,
    })
}

pub fn init(root: &Path) -> Result<usize> {
    crate::discovery::init(root)
}
