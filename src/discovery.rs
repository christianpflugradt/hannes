//! Fixed filesystem discovery shared by project roots and imported rule-set roots.
use crate::{
    config::source_path,
    config::{self, Check, Config, Review, RuleDefinition, RuleSet},
    limits::*,
};
use anyhow::{Context, Result, ensure};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub(crate) struct Loaded {
    pub config: Config,
    pub checks: Vec<(PathBuf, Check)>,
    pub reviews: Vec<(PathBuf, Review)>,
    pub rule_sets: Vec<(PathBuf, RuleSet)>,
}
pub(crate) fn load(root: &Path) -> Result<Loaded> {
    let directory = config::path("");
    let config = document(root, &directory.join("config.toml"))?;
    let mut sets = Vec::new();
    for path in entries(root, &directory.join("rule-sets"))? {
        let full = source_path(root, &path)?;
        ensure!(
            full.is_file() && path.extension().is_some_and(|e| e == "toml"),
            "{}: expected a rule-set TOML file",
            path.display()
        );
        let set: RuleSet = document(root, &path)?;
        sets.push((path, set));
    }
    load_rules(root, &directory, config, sets)
}
pub(crate) fn load_external(root: &Path, consumer: Config) -> Result<Loaded> {
    let central = PathBuf::from("config.toml");
    let set: RuleSet = document(root, &central)?;
    ensure!(
        root.join("rule-sets")
            .symlink_metadata()
            .is_err_and(|e| e.kind() == std::io::ErrorKind::NotFound),
        "external root must define exactly one rule set in config.toml; rule-sets is unsupported"
    );
    load_rules(root, Path::new("."), consumer, vec![(central, set)])
}
pub(crate) fn document<T: serde::de::DeserializeOwned>(root: &Path, path: &Path) -> Result<T> {
    let full =
        source_path(root, path).with_context(|| format!("missing or unsafe {}", path.display()))?;
    let text = config::read_bounded(&full, MAX_METADATA_BYTES)?;
    toml::from_str(&text).map_err(|e: toml::de::Error| {
        anyhow::anyhow!("invalid {}: {}", path.display(), e.message())
    })
}

fn resource(root: &Path, directory: &Path, filename: &str) -> Result<PathBuf> {
    ensure!(
        !filename.is_empty()
            && filename != "."
            && filename != ".."
            && !filename.contains(['/', '\\', ':', '\0'])
            && !Path::new(filename).is_absolute(),
        "resource must be a filename directly inside its rule directory: {filename:?}"
    );
    let path = source_path(root, &directory.join(filename))?;
    ensure!(
        path.is_file(),
        "resource must be a regular file: {}",
        path.display()
    );
    Ok(path)
}

fn entries(root: &Path, directory: &Path) -> Result<Vec<PathBuf>> {
    match root.join(directory).symlink_metadata() {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
        Err(e) => return Err(e.into()),
        Ok(_) => {}
    }
    let full = source_path(root, directory)?;
    ensure!(full.is_dir(), "{} must be a directory", directory.display());
    let mut entries = fs::read_dir(full)?
        .map(|e| Ok(directory.join(e?.file_name())))
        .collect::<Result<Vec<_>>>()?;
    entries.sort();
    ensure!(
        entries.len() <= MAX_RULES,
        "at most {MAX_RULES} entries in {}",
        directory.display()
    );
    Ok(entries)
}

fn load_rules(
    root: &Path,
    directory: &Path,
    config: Config,
    rule_sets: Vec<(PathBuf, RuleSet)>,
) -> Result<Loaded> {
    let mut loaded = Loaded {
        config,
        rule_sets,
        checks: vec![],
        reviews: vec![],
    };
    for kind in ["checks", "reviews"] {
        for dir in entries(root, &directory.join(kind))? {
            let full = source_path(root, &dir)?;
            ensure!(
                full.is_dir(),
                "{}: a rule must be a directory",
                dir.display()
            );
            let path = dir.join("rule.toml");
            let rule: RuleDefinition = document(root, &path)?;
            match (kind, rule.check, rule.review) {
                ("checks", Some(check), None) => {
                    let entrypoint = resource(root, &dir, &check.entrypoint)?;
                    if let Some(interpreter) = &check.interpreter {
                        ensure!(
                            !interpreter.trim().is_empty() && !interpreter.contains('\0'),
                            "interpreter must name one executable"
                        );
                    }
                    loaded.checks.push((
                        path.clone(),
                        Check {
                            id: rule.id.clone(),
                            description: rule.description,
                            enabled: rule.enabled,
                            blocking: rule.blocking,
                            entrypoint,
                            interpreter: check.interpreter,
                            timeout_seconds: check.timeout_seconds,
                            run_at: rule.run_at.clone(),
                        },
                    ));
                }
                ("reviews", None, Some(review)) => {
                    let prompt = resource(root, &dir, &review.prompt)?;
                    ensure!(
                        Path::new(&review.prompt)
                            .extension()
                            .is_some_and(|e| e == "md"),
                        "review prompt must be Markdown (.md)"
                    );
                    loaded.reviews.push((
                        path.clone(),
                        Review {
                            id: rule.id.clone(),
                            description: rule.description,
                            enabled: rule.enabled,
                            blocking: rule.blocking,
                            files: review.files,
                            context: review.context,
                            run_at: rule.run_at.clone(),
                            text: config::read_bounded(&prompt, MAX_INSTRUCTION_BYTES)?,
                        },
                    ));
                }
                _ => anyhow::bail!(
                    "{}: expected exactly one [{}] implementation",
                    path.display(),
                    if kind == "checks" { "check" } else { "review" }
                ),
            };
            ensure!(
                loaded.checks.len() + loaded.reviews.len() <= MAX_RULES,
                "at most {MAX_RULES} rules"
            );
        }
    }
    Ok(loaded)
}

/// Create missing configuration files without replacing existing files.
pub(crate) fn init(root: &Path) -> Result<usize> {
    use std::{fs::OpenOptions, io::Write};
    let root = root.canonicalize()?;
    crate::git_snapshot::require_root(&root)?;
    for relative in [
        "",
        "checks",
        "checks/always-ok",
        "checks/deliberately-failing",
        "reviews",
        "reviews/product-consistency",
        "reports",
    ] {
        let relative = config::path(relative);
        let full = root.join(&relative);
        if full
            .symlink_metadata()
            .is_err_and(|e| e.kind() == std::io::ErrorKind::NotFound)
        {
            fs::create_dir(&full)?;
        }
        ensure!(
            source_path(&root, &relative)?.is_dir(),
            "expected configuration directory"
        );
    }
    let project = root.file_name().unwrap_or_default().to_string_lossy();
    let config = Config {
        project: project.into_owned(),
        max_display_files: MAX_DISPLAY_FILES,
        execution_points: Default::default(),
        imports: vec![],
    };
    let (entrypoint, pass, fail) = if cfg!(windows) {
        ("index.cmd", "@git --version\r\n", "@exit /b 1\r\n")
    } else {
        (
            "index.sh",
            "#!/bin/sh\nexec git --version\n",
            "#!/bin/sh\nexit 1\n",
        )
    };
    let check = |id: &str, description: &str, enabled: bool| -> Result<String> {
        Ok(toml::to_string(&RuleDefinition {
            id: id.into(),
            description: description.into(),
            enabled,
            blocking: true,
            run_at: vec!["pre-push".into()],
            check: Some(crate::config::CheckDefinition {
                entrypoint: entrypoint.into(),
                interpreter: None,
                timeout_seconds: 30,
            }),
            review: None,
        })?)
    };
    let review = RuleDefinition {
        id: "product-consistency".into(),
        description: "Evaluate behavior against documented expectations".into(),
        enabled: false,
        blocking: true,
        run_at: vec!["pre-push".into()],
        check: None,
        review: Some(crate::config::ReviewDefinition {
            prompt: "prompt.md".into(),
            files: vec![],
            context: vec![],
        }),
    };
    let files = [
        ("config.toml".into(), toml::to_string(&config)?),
        ("provider.toml".into(), toml::to_string(&crate::config::ProviderConfig::scaffold())?),
        ("checks/always-ok/rule.toml".into(), check("always-ok", "Example deterministic pass", true)?),
        (format!("checks/always-ok/{entrypoint}"), pass.into()),
        ("checks/deliberately-failing/rule.toml".into(), check("deliberately-failing", "Disabled example demonstrating failure", false)?),
        (format!("checks/deliberately-failing/{entrypoint}"), fail.into()),
        ("reviews/product-consistency/rule.toml".into(), toml::to_string(&review)?),
        ("reviews/product-consistency/prompt.md".into(), "Evaluate the current working files against documented expectations. Cite concrete file paths and lines. Report uncertainty as ERROR, grounded contradictions as FAIL, and PASS only with evidence. Customize before enabling.\n".into()),
        ("reports/.gitignore".into(), "*\n!.gitignore\n".into()),
        ("README.md".into(), include_str!("../docs/configuration.md").into()),
    ];
    let mut created = 0;
    for (relative, text) in files {
        let full = root.join(config::path(&relative));
        match OpenOptions::new().write(true).create_new(true).open(&full) {
            Ok(mut file) => {
                file.write_all(text.as_bytes())?;
                #[cfg(unix)]
                if relative.ends_with(".sh") {
                    use std::os::unix::fs::PermissionsExt;
                    file.set_permissions(fs::Permissions::from_mode(0o755))?;
                }
                created += 1;
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                let existing = source_path(&root, &config::path(&relative))?;
                ensure!(
                    existing.is_file(),
                    "configuration path must be a regular file"
                );
            }
            Err(e) => return Err(e.into()),
        }
    }
    Ok(created)
}
