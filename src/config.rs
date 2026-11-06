use crate::limits::*;
use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
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
    pub default_profile: Option<String>,
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
    pub command: Vec<String>,
    #[serde(default = "default_directory")]
    pub working_directory: String,
    pub timeout_seconds: u64,
    #[serde(default)]
    pub tags: Vec<String>,
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
    pub profiles: Vec<Profile>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    pub id: String,
    #[serde(default)]
    pub rules: Vec<String>,
    #[serde(default)]
    pub include_tags: Vec<String>,
    #[serde(default)]
    pub gate: bool,
    #[serde(default = "worktree")]
    pub input: String,
}
fn worktree() -> String {
    "worktree".into()
}
impl Profile {
    pub fn selects(&self, id: &str, tags: &[String]) -> bool {
        self.id == "all"
            || self.rules.iter().any(|rule| rule == id)
            || tags.iter().any(|tag| self.include_tags.contains(tag))
    }
    pub fn all() -> Self {
        Self {
            id: "all".into(),
            rules: vec![],
            include_tags: vec![],
            gate: false,
            input: worktree(),
        }
    }
}

fn tag(value: &str) -> Result<()> {
    ensure!(
        !value.is_empty()
            && value.len() <= MAX_TAG_BYTES
            && value.as_bytes()[0].is_ascii_alphanumeric()
            && value
                .bytes()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || b"-_".contains(&c)),
        "invalid tag/profile ID {value:?}: use 1..{MAX_TAG_BYTES} lowercase ASCII letters, digits, - or _, starting with a letter or digit"
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
fn parse_rule<T: serde::de::DeserializeOwned>(root: &Path, path: &Path, version: u32) -> Result<T> {
    let text = read_bounded(&inside(root, path)?, MAX_METADATA)?;
    if version == 1 {
        let table: toml::Table = toml::from_str(&text)?;
        ensure!(
            !table.contains_key("tags"),
            "{}: tags require configuration version 2",
            path.display()
        );
    }
    toml::from_str(&text).with_context(|| format!("invalid {}", path.display()))
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
fn metadata_files(root: &Path, dir: &Path) -> Result<Vec<PathBuf>> {
    let full = inside(root, dir)?;
    let mut paths = Vec::new();
    for entry in fs::read_dir(full)? {
        let entry = entry?;
        if entry.path().extension().is_some_and(|x| x == "toml") {
            paths.push(dir.join(entry.file_name()));
        }
        ensure!(
            paths.len() <= MAX_RULES,
            "at most {MAX_RULES} metadata files per directory"
        );
    }
    paths.sort();
    Ok(paths)
}
pub fn load(root: &Path) -> Result<Project> {
    let root = root.canonicalize()?;
    crate::git_snapshot::require_root(&root)?;
    let config: Config = parse(&root, &self::path("config.toml"))?;
    ensure!(
        matches!(config.version, 1 | 2),
        "unsupported configuration version {}",
        config.version
    );
    ensure!(
        config.version == 2 || config.default_profile.is_none(),
        "default_profile requires configuration version 2"
    );
    nonempty(&config.project, "project")?;
    ensure!(
        (1..=MAX_DISPLAY_FILES).contains(&config.max_display_files),
        "max_display_files must be 1..={MAX_DISPLAY_FILES}"
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
    for path in metadata_files(&root, &self::path("checks"))? {
        let mut check: Check = parse_rule(&root, &path, config.version)?;
        (|| -> Result<()> {
            identity(&check.id, &check.description, &mut ids)?;
            tags(&mut check.tags)?;
            timeout(check.timeout_seconds)?;
            ensure!(
                !check.command.is_empty() && check.command.len() <= MAX_COMMAND_ARGS,
                "command must have 1..={MAX_COMMAND_ARGS} argv elements"
            );
            nonempty(&check.command[0], "command executable")?;
            ensure!(
                check
                    .command
                    .iter()
                    .all(|s| !s.contains('\0') && s.len() <= MAX_ARGUMENT_BYTES),
                "command arguments contain NUL or exceed {MAX_ARGUMENT_BYTES} bytes"
            );
            let exe = Path::new(&check.command[0]);
            if exe.is_absolute()
                || check.command[0].contains('/')
                || check.command[0].contains('\\')
            {
                relative(exe)?;
                let base = Path::new(&check.working_directory).join(exe);
                ensure!(
                    inside(&root, &base)?.is_file(),
                    "command executable must be a file"
                );
            }
            ensure!(
                inside(&root, Path::new(&check.working_directory))?.is_dir(),
                "working_directory must be a directory"
            );
            Ok(())
        })()
        .with_context(|| format!("invalid {}", path.display()))?;
        checks.push(check);
    }
    let mut reviews = Vec::new();
    let mut instruction_bytes = 0;
    for path in metadata_files(&root, &self::path("reviews"))? {
        let mut review: Review = parse_rule(&root, &path, config.version)?;
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
            let full = inside(&root, &self::path("reviews").join(p))?;
            ensure!(
                full.starts_with(root.join(self::path("reviews")).canonicalize()?),
                "instructions must stay inside reviews directory"
            );
            review.text = read_bounded(&full, MAX_INSTRUCTIONS)?;
            nonempty(&review.text, "instructions")?;
            instruction_bytes += review.text.len();
            ensure!(
                instruction_bytes <= MAX_TOTAL_INSTRUCTION_BYTES,
                "total review instructions exceed {} bytes", MAX_TOTAL_INSTRUCTION_BYTES
            );
            Ok(())
        })()
        .with_context(|| format!("invalid {}", path.display()))?;
        reviews.push(review);
    }
    checks.sort_by(|a, b| a.id.cmp(&b.id));
    reviews.sort_by(|a, b| a.id.cmp(&b.id));
    crate::review_context::collect(&root, &reviews.iter().collect::<Vec<_>>())?;
    let mut profiles = Vec::new();
    if root.join(self::path("profiles")).symlink_metadata().is_ok() {
        let paths = metadata_files(&root, &self::path("profiles"))?;
        ensure!(
            config.version == 2 || paths.is_empty(),
            "profiles require configuration version 2"
        );
        let known_tags: BTreeSet<_> = checks
            .iter()
            .flat_map(|r| &r.tags)
            .chain(reviews.iter().flat_map(|r| &r.tags))
            .collect();
        let mut profile_ids = BTreeSet::new();
        for path in paths {
            let mut profile: Profile = parse(&root, &path)?;
            (|| -> Result<()> {
                tag(&profile.id)?;
                ensure!(
                    profile.id != "all" && profile_ids.insert(profile.id.clone()),
                    "reserved or duplicate profile ID: {}",
                    profile.id
                );
                ensure!(
                    profile.input == "worktree",
                    "unsupported profile input {}; only worktree is implemented",
                    profile.input
                );
                unique(&mut profile.rules, "profile rule selector")?;
                unique(&mut profile.include_tags, "profile tag selector")?;
                for id in &profile.rules {
                    ensure!(ids.contains(id), "unknown rule selector: {id}");
                }
                for value in &profile.include_tags {
                    tag(value)?;
                    ensure!(known_tags.contains(value), "unknown tag selector: {value}");
                }
                if profile.gate {
                    let selected: Vec<_> = checks
                        .iter()
                        .map(|r| (&r.id, &r.tags, r.enabled))
                        .chain(reviews.iter().map(|r| (&r.id, &r.tags, r.enabled)))
                        .filter(|(id, tags, _)| profile.selects(id, tags))
                        .collect();
                    ensure!(!selected.is_empty(), "gate {} selects no rules", profile.id);
                    ensure!(
                        selected.iter().all(|(_, _, enabled)| *enabled),
                        "gate {} selects disabled required rules",
                        profile.id
                    );
                }
                Ok(())
            })()
            .with_context(|| format!("invalid {}", path.display()))?;
            profiles.push(profile);
        }
    }
    profiles.sort_by(|a, b| a.id.cmp(&b.id));
    if let Some(default) = &config.default_profile {
        ensure!(
            default == "all" || profiles.iter().any(|p| &p.id == default),
            "unknown default profile: {default}"
        );
    }
    Ok(Project {
        root,
        config,
        provider,
        checks,
        reviews,
        profiles,
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
        version: 1,
        project,
        max_display_files: MAX_DISPLAY_FILES,
        default_profile: None,
    };
    let check = |id: &str, description: &str, enabled: bool, argument: &str| Check {
        id: id.into(),
        description: description.into(),
        enabled,
        command: vec!["git".into(), argument.into()],
        working_directory: default_directory(),
        timeout_seconds: 30,
        tags: vec![],
    };
    let review = Review {
        id: "product-consistency".into(),
        description: "Evaluate behavior against documented expectations".into(),
        enabled: false,
        instructions: "product-consistency.md".into(),
        files: vec![],
        context: vec![],
        tags: vec![],
        text: String::new(),
    };
    let files = [
        ("config.toml", scaffold(&config)?),
        ("provider.toml", scaffold(&ProviderConfig::scaffold())?),
        ("checks/always-ok.toml", scaffold(&check("always-ok", "Example deterministic pass", true, "--version"))?),
        ("checks/deliberately-failing.toml", scaffold(&check("deliberately-failing", "Disabled example demonstrating failure", false, "definitely-not-a-subcommand"))?),
        ("reviews/product-consistency.toml", scaffold(&review)?),
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

/// Serialize real configuration types while keeping version-1 examples minimal.
fn scaffold(value: &impl Serialize) -> Result<String> {
    let mut table = toml::Value::try_from(value)?;
    for key in ["tags", "files", "context"] {
        if table
            .get(key)
            .and_then(toml::Value::as_array)
            .is_some_and(Vec::is_empty)
        {
            table.as_table_mut().unwrap().remove(key);
        }
    }
    Ok(toml::to_string(&table)?)
}
