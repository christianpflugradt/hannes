//! Local filesystem loading. Domain resolution lives in `rules`.
use crate::{
    config::{self, Check, Config, Definitions, Review, RuleSet},
    limits::*,
    rules::Assignment,
};
use anyhow::{Context, Result, ensure};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub struct Loaded {
    pub config: Config,
    pub checks: Vec<(PathBuf, Check)>,
    pub reviews: Vec<(PathBuf, Review)>,
    pub rule_sets: Vec<(PathBuf, RuleSet)>,
    pub assignments: Vec<Assignment>,
    pub imports: Vec<crate::external::Import>,
}
fn discover(root: &Path, directory: &Path, depth: usize, files: &mut Vec<PathBuf>) -> Result<()> {
    ensure!(
        depth <= MAX_CONFIG_DEPTH,
        "{}: configuration nesting exceeds {MAX_CONFIG_DEPTH}",
        directory.display()
    );
    let metadata = root.join(directory).symlink_metadata()?;
    ensure!(
        metadata.is_dir() && !metadata.file_type().is_symlink(),
        "{}: configuration directory must not be a symlink",
        directory.display()
    );
    let mut entries = fs::read_dir(root.join(directory))?.collect::<std::io::Result<Vec<_>>>()?;
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let name = entry.file_name();
        let name = name.to_str().context("configuration path must be UTF-8")?;
        if name.starts_with('.') || ["reports", "cache", "caches"].contains(&name) {
            continue;
        }
        let path = directory.join(name);
        let kind = entry.file_type()?;
        ensure!(
            !kind.is_symlink(),
            "{}: configuration symlinks are unsupported",
            path.display()
        );
        if kind.is_dir() {
            discover(root, &path, depth + 1, files)?;
        } else {
            ensure!(
                kind.is_file(),
                "{}: configuration entry must be a regular file",
                path.display()
            );
            if path.extension().is_some_and(|e| e == "toml")
                && path != config::path("provider.toml")
            {
                files.push(path);
                ensure!(
                    files.len() <= MAX_RULES,
                    "at most {MAX_RULES} configuration documents"
                );
            }
        }
    }
    Ok(())
}
pub fn load(root: &Path) -> Result<Loaded> {
    load_directory(root, &config::path(""), None)
}

pub(crate) fn load_external(root: &Path, config: Config) -> Result<Loaded> {
    load_directory(root, Path::new("."), Some(config))
}

fn load_directory(root: &Path, directory: &Path, fallback: Option<Config>) -> Result<Loaded> {
    let external = fallback.is_some();
    let mut files = Vec::new();
    discover(root, directory, 0, &mut files)?;
    files.sort();
    let central = config::path("config.toml");
    ensure!(
        external || files.contains(&central),
        "missing {}",
        central.display()
    );
    let mut loaded = fallback;
    let mut imports = Vec::new();
    let mut documents = Vec::new();
    for path in files {
        let text = config::read_bounded(&root.join(&path), MAX_METADATA_BYTES)?;
        // TOML source excerpts could expose credentials in a malformed Git URL.
        let mut table: toml::Table = toml::from_str(&text).map_err(|error: toml::de::Error| {
            anyhow::anyhow!("invalid {}: {}", path.display(), error.message())
        })?;
        let mut definitions = toml::Table::new();
        for key in ["checks", "reviews", "rule_sets"] {
            if let Some(value) = table.remove(key) {
                definitions.insert(key.into(), value);
            }
        }
        if !external && path == central {
            loaded = Some(
                toml::Value::Table(table)
                    .try_into::<Config>()
                    .with_context(|| format!("invalid {}", path.display()))?,
            );
        } else {
            ensure!(
                table.is_empty(),
                "{}: unknown schema fields {}; use [[checks]], [[reviews]] or [[rule_sets]]",
                path.display(),
                table.keys().cloned().collect::<Vec<_>>().join(", ")
            );
            ensure!(
                !definitions.is_empty(),
                "{}: expected rule or rule-set tables",
                path.display()
            );
        }
        fn entries<T: serde::de::DeserializeOwned>(
            table: &mut toml::Table,
            key: &str,
            path: &Path,
        ) -> Result<Vec<T>> {
            let Some(value) = table.remove(key) else {
                return Ok(vec![]);
            };
            let toml::Value::Array(values) = value else {
                anyhow::bail!("{}: {key}: expected array tables [[{key}]]", path.display());
            };
            values
                .into_iter()
                .enumerate()
                .map(|(index, value)| {
                    let id = value
                        .get("id")
                        .and_then(toml::Value::as_str)
                        .map(String::from)
                        .unwrap_or_else(|| format!("entry {}", index + 1));
                    value
                        .try_into()
                        .with_context(|| format!("invalid {}: {key} {id}", path.display()))
                })
                .collect()
        }
        let mut sets = Vec::new();
        if let Some(value) = definitions.remove("rule_sets") {
            let toml::Value::Array(values) = value else {
                anyhow::bail!("{}: expected array tables [[rule_sets]]", path.display());
            };
            for (index, value) in values.into_iter().enumerate() {
                if value.get("git").is_some() {
                    ensure!(
                        !external,
                        "nested external rule-set imports are unsupported"
                    );
                    let import = value.try_into::<crate::external::Import>().with_context(|| format!(
                        "invalid {}: external rule-set entry {} requires exactly git, path and ref; overrides are unsupported",
                        path.display(), index + 1))?;
                    imports.push(import);
                } else {
                    let id = value
                        .get("id")
                        .and_then(toml::Value::as_str)
                        .map(String::from)
                        .unwrap_or_else(|| format!("entry {}", index + 1));
                    sets.push(
                        value.try_into::<RuleSet>().with_context(|| {
                            format!("invalid {}: rule set {id}", path.display())
                        })?,
                    );
                }
            }
        }
        let defs = Definitions {
            checks: entries(&mut definitions, "checks", &path)?,
            reviews: entries(&mut definitions, "reviews", &path)?,
            rule_sets: sets,
        };
        documents.push((path, defs));
    }
    let mut result = Loaded {
        config: loaded.context("missing config.toml")?,
        checks: vec![],
        reviews: vec![],
        rule_sets: vec![],
        assignments: vec![],
        imports,
    };
    for (source, defs) in documents {
        for check in defs.checks {
            result.assignments.push(Assignment {
                id: check.id.clone(),
                enabled: check.enabled,
                run_at: check.run_at.clone(),
                source: source.clone(),
            });
            result.checks.push((source.clone(), check));
        }
        for review in defs.reviews {
            result.assignments.push(Assignment {
                id: review.id.clone(),
                enabled: review.enabled,
                run_at: review.run_at.clone(),
                source: source.clone(),
            });
            result.reviews.push((source.clone(), review));
        }
        result
            .rule_sets
            .extend(defs.rule_sets.into_iter().map(|set| (source.clone(), set)));
    }
    ensure!(
        result.assignments.len() <= MAX_RULES && result.rule_sets.len() <= MAX_RULES,
        "at most {MAX_RULES} rules and {MAX_RULES} rule sets"
    );
    Ok(result)
}
