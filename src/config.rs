use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    fs::{self, OpenOptions},
    io::{Read, Write},
    path::{Component, Path, PathBuf},
};

pub const MAX_METADATA: u64 = 64 * 1024;
pub const MAX_INSTRUCTIONS: u64 = 32 * 1024;
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
            tool_output_token_limit: 2000,
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
            && value.len() <= 32
            && value.as_bytes()[0].is_ascii_alphanumeric()
            && value
                .bytes()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || b"-_".contains(&c)),
        "invalid tag/profile ID {value:?}: use 1..32 lowercase ASCII letters, digits, - or _, starting with a letter or digit"
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
    ensure!(values.len() <= 32, "at most 32 tags per rule");
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
        (1..=3600).contains(&value),
        "timeout_seconds must be 1..=3600"
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
            && id.len() <= 80
            && id
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b"-_".contains(&c)),
        "invalid id {id:?}: use 1..80 ASCII letters, digits, - or _"
    );
    ensure!(
        !id.starts_with("hannes-"),
        "IDs beginning hannes- are reserved"
    );
    ensure!(ids.insert(id.to_owned()), "duplicate id: {id}");
    nonempty(description, "description")
}
fn metadata_files(root: &Path, dir: &str) -> Result<Vec<PathBuf>> {
    let full = inside(root, Path::new(dir))?;
    let mut paths = Vec::new();
    for entry in fs::read_dir(full)? {
        let entry = entry?;
        if entry.path().extension().is_some_and(|x| x == "toml") {
            paths.push(Path::new(dir).join(entry.file_name()));
        }
        ensure!(
            paths.len() <= 256,
            "at most 256 metadata files per directory"
        );
    }
    paths.sort();
    Ok(paths)
}
pub fn load(root: &Path) -> Result<Project> {
    let root = root.canonicalize()?;
    crate::git_snapshot::require_root(&root)?;
    let config: Config = parse(&root, Path::new(".Hannes/config.toml"))?;
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
        (1..=5).contains(&config.max_display_files),
        "max_display_files must be 1..=5"
    );
    let provider: ProviderConfig = parse(&root, Path::new(".Hannes/provider.toml"))?;
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
        (256..=16384).contains(&profile.tool_output_token_limit),
        "tool_output_token_limit must be 256..=16384"
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
        provider.mcp_servers.len() <= 32,
        "at most 32 allowed MCP servers"
    );
    let mut servers = BTreeSet::new();
    for server in &provider.mcp_servers {
        nonempty(server, "MCP server name")?;
        ensure!(
            server.len() <= 256
                && server
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || b"_-".contains(&byte))
                && servers.insert(server),
            "invalid or duplicate MCP server name"
        );
    }
    let mut ids = BTreeSet::new();
    let mut checks = Vec::new();
    for path in metadata_files(&root, ".Hannes/checks")? {
        let mut check: Check = parse_rule(&root, &path, config.version)?;
        (|| -> Result<()> {
            identity(&check.id, &check.description, &mut ids)?;
            tags(&mut check.tags)?;
            timeout(check.timeout_seconds)?;
            ensure!(
                !check.command.is_empty() && check.command.len() <= 256,
                "command must have 1..=256 argv elements"
            );
            nonempty(&check.command[0], "command executable")?;
            ensure!(
                check
                    .command
                    .iter()
                    .all(|s| !s.contains('\0') && s.len() <= 32768),
                "command arguments contain NUL or exceed 32768 bytes"
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
    for path in metadata_files(&root, ".Hannes/reviews")? {
        let mut review: Review = parse_rule(&root, &path, config.version)?;
        (|| -> Result<()> {
            identity(&review.id, &review.description, &mut ids)?;
            tags(&mut review.tags)?;
            ensure!(
                review.files.len() <= 64 && review.context.len() <= 32,
                "review permits at most 64 files and 32 context entries"
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
                    source.headings.len() <= 32,
                    "at most 32 headings per context entry"
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
            let full = inside(&root, &Path::new(".Hannes/reviews").join(p))?;
            ensure!(
                full.starts_with(root.join(".Hannes/reviews").canonicalize()?),
                "instructions must stay inside reviews directory"
            );
            review.text = read_bounded(&full, MAX_INSTRUCTIONS)?;
            nonempty(&review.text, "instructions")?;
            instruction_bytes += review.text.len();
            ensure!(
                instruction_bytes <= 256 * 1024,
                "total review instructions exceed 256 KiB"
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
    if root.join(".Hannes/profiles").symlink_metadata().is_ok() {
        let paths = metadata_files(&root, ".Hannes/profiles")?;
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
    for dir in [
        ".Hannes",
        ".Hannes/checks",
        ".Hannes/reviews",
        ".Hannes/reports",
    ] {
        let p = root.join(dir);
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
    let project = root.file_name().unwrap_or_default().to_string_lossy();
    let name = toml::Value::String(project.into_owned()).to_string();
    let config = format!("version = 1\nproject = {name}\nmax_display_files = 5\n");
    let files = [
        ("config.toml", config.as_str()),
        (
            "provider.toml",
            "provider = \"codex-cli\"\ntimeout_seconds = 180\nmodel = \"gpt-6.1-sol\"\nmcp_servers = []\n\n[review_profile]\nreasoning_effort = \"medium\"\ntool_output_token_limit = 2000\n",
        ),
        (
            "checks/always-ok.toml",
            "id = \"always-ok\"\ndescription = \"Example deterministic pass\"\nenabled = true\ncommand = [\"git\", \"--version\"]\nworking_directory = \".\"\ntimeout_seconds = 30\n",
        ),
        (
            "checks/deliberately-failing.toml",
            "id = \"deliberately-failing\"\ndescription = \"Disabled example demonstrating failure\"\nenabled = false\ncommand = [\"git\", \"definitely-not-a-subcommand\"]\ntimeout_seconds = 30\n",
        ),
        (
            "reviews/product-consistency.toml",
            "id = \"product-consistency\"\ndescription = \"Evaluate behavior against documented expectations\"\nenabled = false\ninstructions = \"product-consistency.md\"\n",
        ),
        (
            "reviews/product-consistency.md",
            "Evaluate the current working files against expectations explicitly documented in this project. Inspect relevant changed code and documentation. Do not invent requirements. Cite concrete file paths and line numbers. State uncertainty as ERROR when necessary evidence is unavailable. Report FAIL for a grounded contradiction, PASS only with grounded reasons. Customize this guidance before enabling.\n",
        ),
        ("reports/.gitignore", "*\n!.gitignore\n"),
        ("README.md", include_str!("../docs/configuration.md")),
    ];
    let mut created = 0;
    for (path, content) in files {
        let full = root.join(".Hannes").join(path);
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
