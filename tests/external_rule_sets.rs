mod support;
use hannes::{
    config, external, planner,
    provider::{Provider, Request},
    report::Status,
};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};
use tempfile::TempDir;

fn git(root: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .current_dir(root)
        .args([
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.invalid",
            "-c",
            "commit.gpgsign=false",
            "-c",
            "core.hooksPath=/dev/null",
        ])
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().into()
}
fn write(root: &Path, path: &str, text: &str) {
    let path = root.join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}
fn commit(root: &Path) -> String {
    git(root, &["add", "."]);
    git(root, &["commit", "--quiet", "-m", "fixture"]);
    git(root, &["rev-parse", "HEAD"])
}
fn definitions(root: &Path, path: &str, script: &str) {
    write(
        root,
        &format!("{path}/config.toml"),
        "id = 'quality'\ndescription = 'Reusable rules'\nrules = ['required-files']\nrun_at = ['check']\n",
    );
    write(
        root,
        &format!("{path}/checks/required-files/rule.toml"),
        "id = 'required-files'\ndescription = 'Inspect consuming project'\nenabled = true\n[check]\nentrypoint = 'index.sh'\ninterpreter = 'sh'\ntimeout_seconds = 5\n",
    );
    write(
        root,
        &format!("{path}/checks/required-files/index.sh"),
        script,
    );
}
struct Fixture {
    _directory: TempDir,
    root: PathBuf,
    remote: PathBuf,
    cache: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let base = directory.path().canonicalize().unwrap();
        let root = base.join("project");
        let remote = base.join("remote");
        let cache = base.join("cache");
        fs::create_dir(&root).unwrap();
        fs::create_dir(&remote).unwrap();
        fs::create_dir(&cache).unwrap();
        git(&root, &["init", "--quiet", "--initial-branch=main"]);
        git(&remote, &["init", "--quiet", "--initial-branch=main"]);
        config::init(&root).unwrap();
        write(
            &root,
            ".hannes/config.toml",
            r#"project = "My_App"
max_display_files = 5
[execution_points]
custom = []
"#,
        );
        definitions(&remote, "repository", "git --version");
        definitions(&remote, "software", "git --version");
        commit(&remote);
        Self {
            _directory: directory,
            root,
            remote,
            cache,
        }
    }
    fn imports(&self, selections: &[(&str, &str)]) {
        let base = r#"project = "My_App"
max_display_files = 5
[execution_points]
custom = []
"#;
        let mut text = base.to_owned();
        for (path, reference) in selections {
            text += &format!(
                "\n[[imports]]\ngit = {:?}\npath = {path:?}\nref = {reference:?}\n",
                self.remote.to_str().unwrap()
            );
        }
        write(&self.root, ".hannes/config.toml", &text);
    }
    fn load(&self, refresh: bool) -> anyhow::Result<config::Project> {
        external::load(&self.root, refresh, Some(&self.cache))
    }
    fn state(&self) -> Vec<u8> {
        fs::read(self.root.join(".git/hannes/external-rule-sets.json")).unwrap()
    }
    fn origin(&self) -> hannes::external::Origin {
        self.load(false)
            .unwrap()
            .effective
            .rules
            .values()
            .find_map(|r| r.external.clone())
            .unwrap()
    }
    fn contents(&self) -> PathBuf {
        let origin = self.origin();
        self.cache
            .join("git")
            .join(origin.repository)
            .join(origin.commit)
            .join("contents")
    }
}
struct Never;
impl Provider for Never {
    fn review(&self, _: &Request<'_>) -> anyhow::Result<String> {
        panic!("unexpected provider call")
    }
}

#[test]
fn branch_tag_and_full_commit_share_one_immutable_revision() {
    let f = Fixture::new();
    git(&f.remote, &["tag", "-a", "v1.0.0", "-m", "tag"]);
    let commit = git(&f.remote, &["rev-parse", "HEAD"]);
    f.imports(&[("repository", "main"), ("software", "v1.0.0")]);
    let project = f.load(true).unwrap();
    let origins: Vec<_> = project
        .effective
        .rules
        .values()
        .filter_map(|r| r.external.as_ref())
        .collect();
    assert_eq!(origins.len(), 2);
    assert!(origins.iter().all(|o| o.commit == commit));
    let repository = f.cache.join("git").join(&origins[0].repository);
    assert_eq!(
        fs::read_dir(repository)
            .unwrap()
            .filter(|e| e.as_ref().unwrap().path().join("receipt.json").exists())
            .count(),
        1
    );
    f.imports(&[("repository", &commit)]);
    f.load(true).unwrap();
    assert_eq!(f.origin().commit, commit);
}

#[test]
fn local_and_identical_external_ids_are_isolated_and_order_is_deterministic() {
    let f = Fixture::new();
    support::check(&f.root, "required-files", &["check"], "git --version");
    f.imports(&[("repository", "main"), ("software", "main")]);
    let a = f.load(true).unwrap();
    let plan = planner::build(&a, Some("check"), false).unwrap().plan;
    assert_eq!(plan.scheduled.len(), 3);
    assert!(plan.scheduled.iter().any(|r| r.id == "required-files"));
    assert!(plan.scheduled.iter().all(|r| {
        !r.origin
            .as_ref()
            .unwrap()
            .source
            .to_string_lossy()
            .contains("cache/")
    }));
    f.imports(&[("software", "main"), ("repository", "main")]);
    let b = f.load(true).unwrap();
    assert_eq!(
        serde_json::to_value(&a.effective.rules).unwrap(),
        serde_json::to_value(&b.effective.rules).unwrap()
    );
    let report = hannes::verify_project_at(
        b,
        &Never,
        hannes::VerificationMode::AllRules,
        Some("check"),
        &mut |_| {},
    )
    .unwrap();
    assert_eq!(report.overall, Status::Pass);
    assert_eq!(report.results.len(), 3);
    hannes::report::read(&f.root).unwrap();
}

#[test]
fn execution_targets_consumer_and_source_resources_are_explicit() {
    let f = Fixture::new();
    write(&f.root, "README.md", "consumer");
    // Explicit shell composition remains supported and receives the source environment.
    definitions(
        &f.remote,
        "repository",
        "sh -c 'test -f README.md && test -f \"$HANNES_RULE_DIRECTORY/resource.txt\"'",
    );
    write(
        &f.remote,
        "repository/checks/required-files/resource.txt",
        "source",
    );
    commit(&f.remote);
    f.imports(&[("repository", "main")]);
    let project = f.load(true).unwrap();
    let report = hannes::verify_project_at(
        project,
        &Never,
        hannes::VerificationMode::AllRules,
        Some("check"),
        &mut |_| {},
    )
    .unwrap();
    assert_eq!(report.overall, Status::Pass);
    assert_eq!(report.results.len(), 1);
}

#[test]
fn offline_execution_uses_last_resolution_and_refresh_moves_only_mutable_refs() {
    let f = Fixture::new();
    let original = git(&f.remote, &["rev-parse", "HEAD"]);
    f.imports(&[("repository", "main")]);
    f.load(true).unwrap();
    let contents = f.contents();
    let timestamp = contents.metadata().unwrap().modified().unwrap();
    write(&f.remote, "repository/note.md", "new");
    let updated = commit(&f.remote);
    assert_eq!(f.origin().commit, original);
    f.load(true).unwrap();
    assert_eq!(f.origin().commit, updated);
    assert_eq!(contents.metadata().unwrap().modified().unwrap(), timestamp);
    f.imports(&[("repository", &original)]);
    f.load(true).unwrap();
    assert_eq!(f.origin().commit, original);
    fs::rename(&f.remote, f.remote.with_extension("offline")).unwrap();
    let project = f.load(false).unwrap();
    let report = hannes::verify_project_at(
        project,
        &Never,
        hannes::VerificationMode::AllRules,
        Some("check"),
        &mut |_| {},
    )
    .unwrap();
    assert_eq!(report.overall, Status::Pass);
    let previous = f.state();
    f.imports(&[("repository", "main")]);
    assert!(f.load(true).is_err());
    assert_eq!(f.state(), previous);
}

#[test]
fn invalid_refresh_preserves_previous_valid_state_without_claiming_success() {
    let f = Fixture::new();
    f.imports(&[("repository", "main")]);
    f.load(true).unwrap();
    let previous = f.state();
    write(
        &f.remote,
        "repository/config.toml",
        "id = \"quality\"\ndescription = \"Reusable rules\"\nrules = [\"missing\"]\nrun_at = [\"check\"]\n",
    );
    commit(&f.remote);
    let error = format!("{:#}", f.load(true).unwrap_err());
    assert!(error.contains("unknown rule"), "{error}");
    assert_eq!(f.state(), previous);
    f.load(false).unwrap();
}

#[test]
fn missing_state_missing_cache_and_corruption_fail_closed() {
    let f = Fixture::new();
    f.imports(&[("repository", "main")]);
    assert!(format!("{:#}", f.load(false).unwrap_err()).contains("hannes pass auf"));
    f.load(true).unwrap();
    let contents = f.contents();
    fs::write(
        contents.join("repository/checks/required-files/rule.toml"),
        "corrupted",
    )
    .unwrap();
    assert!(format!("{:#}", f.load(false).unwrap_err()).contains("integrity"));
    assert!(f.load(true).is_err());
    fs::remove_dir_all(contents.parent().unwrap()).unwrap();
    assert!(format!("{:#}", f.load(false).unwrap_err()).contains("hannes pass auf"));
    f.load(true).unwrap();
}

#[test]
fn unavailable_references_and_repository_are_actionable() {
    let f = Fixture::new();
    f.imports(&[("repository", "not-a-reference")]);
    assert!(format!("{:#}", f.load(true).unwrap_err()).contains("reference"));
    f.imports(&[("repository", "main")]);
    fs::remove_dir_all(&f.remote).unwrap();
    assert!(format!("{:#}", f.load(true).unwrap_err()).contains("repository unavailable"));
}

#[test]
fn repository_root_is_a_governance_root() {
    let f = Fixture::new();
    fs::remove_dir_all(f.remote.join("repository")).unwrap();
    fs::remove_dir_all(f.remote.join("software")).unwrap();
    definitions(&f.remote, ".", "git --version");
    commit(&f.remote);
    f.imports(&[(".", "main")]);
    assert_eq!(f.load(true).unwrap().rule_sets.len(), 1);
}

#[test]
fn duplicate_imports_and_consumer_overrides_are_rejected() {
    let f = Fixture::new();
    f.imports(&[("repository", "main"), ("./repository/", "main")]);
    assert!(format!("{:#}", f.load(true).unwrap_err()).contains("duplicate"));
    f.imports(&[("repository", "main")]);
    let path = f.root.join(".hannes/config.toml");
    let text = fs::read_to_string(&path).unwrap();
    for field in [
        "enabled = false",
        "rules = []",
        "run_at = []",
        "id = \"override\"",
    ] {
        fs::write(&path, format!("{text}\n{field}\n")).unwrap();
        assert!(f.load(true).is_err(), "{field}");
    }
}

#[test]
fn traversal_and_symlink_escapes_are_rejected() {
    let f = Fixture::new();
    for path in ["../software", "/tmp", "repository/../software"] {
        f.imports(&[(path, "main")]);
        assert!(format!("{:#}", f.load(true).unwrap_err()).contains("path"));
    }
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink("../software", f.remote.join("link")).unwrap();
        fs::remove_file(f.remote.join("repository/checks/required-files/index.sh")).unwrap();
        std::os::unix::fs::symlink(
            "../../../software/config.toml",
            f.remote.join("repository/checks/required-files/index.sh"),
        )
        .unwrap();
        commit(&f.remote);
        f.imports(&[("link", "main")]);
        assert!(format!("{:#}", f.load(true).unwrap_err()).contains("symlink"));
        f.imports(&[("repository", "main")]);
        assert!(format!("{:#}", f.load(true).unwrap_err()).contains("symlink"));
    }
}

#[test]
fn concurrent_refreshes_publish_complete_state_and_share_checkout() {
    let f = Fixture::new();
    f.imports(&[("repository", "main"), ("software", "main")]);
    std::thread::scope(|scope| {
        let a = scope.spawn(|| f.load(true).unwrap());
        let b = scope.spawn(|| f.load(true).unwrap());
        assert_eq!(
            serde_json::to_value(a.join().unwrap().effective.rules).unwrap(),
            serde_json::to_value(b.join().unwrap().effective.rules).unwrap()
        );
    });
    assert_eq!(f.load(false).unwrap().rule_sets.len(), 2);
}

#[test]
fn retrieval_never_executes_rules_or_remote_hooks_and_errors_hide_credentials() {
    let f = Fixture::new();
    definitions(&f.remote, "repository", "sh -c 'touch SHOULD_NOT_EXIST'");
    commit(&f.remote);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let hook = f.remote.join(".git/hooks/post-checkout");
        fs::write(
            &hook,
            format!("#!/bin/sh\ntouch {}/HOOK_RAN\n", f.root.display()),
        )
        .unwrap();
        fs::set_permissions(hook, fs::Permissions::from_mode(0o755)).unwrap();
    }
    f.imports(&[("repository", "main")]);
    f.load(true).unwrap();
    assert!(!f.root.join("SHOULD_NOT_EXIST").exists());
    assert!(!f.root.join("HOOK_RAN").exists());
    let path = f.root.join(".hannes/config.toml");
    let text = fs::read_to_string(&path).unwrap().replace(
        f.remote.to_str().unwrap(),
        "https://user:SECRET@example.invalid/rules",
    );
    fs::write(path, text).unwrap();
    let error = format!("{:#}", f.load(true).unwrap_err());
    assert!(!error.contains("SECRET"), "{error}");
    assert!(error.contains("credentials"), "{error}");
}

struct ReviewMock;
impl Provider for ReviewMock {
    fn review(&self, request: &Request<'_>) -> anyhow::Result<String> {
        assert_eq!(request.reviews.len(), 1);
        assert_eq!(request.reviews[0].text, "Judge consumer documentation.\n");
        assert!(request.project.root.join("README.md").is_file());
        assert_eq!(request.reviews[0].files, ["README.md"]);
        Ok(serde_json::json!({"reviews":[{
            "id":request.reviews[0].id, "status":"PASS", "reason":"Documentation is present",
            "findings":[{"message":"Consumer README exists","path":"README.md","line":1}]
        }]})
        .to_string())
    }
}
#[test]
fn imported_reviews_use_source_instructions_and_consumer_evidence() {
    let f = Fixture::new();
    write(&f.root, "README.md", "# Consumer\n");
    write(
        &f.remote,
        "repository/config.toml",
        r#"id = "reviews"
description = "Reusable reviews"
rules = ["documentation"]
run_at = ["check"]
"#,
    );
    fs::remove_dir_all(f.remote.join("repository/checks")).unwrap();
    write(
        &f.remote,
        "repository/reviews/documentation/rule.toml",
        r#"id = "documentation"
description = "Review documentation"
enabled = true
[review]
prompt = "criteria.md"
files = ["README.md"]
context = [{ path = "README.md", headings = ["Consumer"] }]
"#,
    );
    write(
        &f.remote,
        "repository/reviews/documentation/criteria.md",
        "Judge consumer documentation.\n",
    );
    commit(&f.remote);
    f.imports(&[("repository", "main")]);
    let project = f.load(true).unwrap();
    let report = hannes::verify_project_at(
        project,
        &ReviewMock,
        hannes::VerificationMode::AllRules,
        Some("check"),
        &mut |_| {},
    )
    .unwrap();
    assert_eq!(report.overall, Status::Pass);
    assert!(report.results[0].id.starts_with("git:"));
    hannes::report::read(&f.root).unwrap();
}

#[test]
fn imported_check_failure_propagates_and_no_unselected_rule_executes() {
    let f = Fixture::new();
    definitions(&f.remote, "repository", "sh -c 'exit 7'");
    commit(&f.remote);
    f.imports(&[("repository", "main")]);
    let project = f.load(true).unwrap();
    assert!(
        planner::build(&project, Some("pre-commit"), false)
            .unwrap()
            .plan
            .scheduled
            .is_empty()
    );
    let report = hannes::verify_project_at(
        project,
        &Never,
        hannes::VerificationMode::AllRules,
        Some("check"),
        &mut |_| {},
    )
    .unwrap();
    assert_eq!(report.overall, Status::Fail);
    assert_eq!(report.results[0].exit_code, Some(7));
}

#[test]
fn source_instruction_paths_cannot_escape_the_selected_directory() {
    for instructions in [
        "../software/criteria.md",
        "/tmp/criteria.md",
        "escape/criteria.md",
    ] {
        let f = Fixture::new();
        write(
            &f.remote,
            "repository/reviews/review/rule.toml",
            &format!(
                r#"id = "review"
description = "Disabled review still validates"
enabled = false
[review]
prompt = {instructions:?}
"#
            ),
        );
        write(&f.remote, "software/criteria.md", "Outside boundary");
        #[cfg(unix)]
        std::os::unix::fs::symlink("../software", f.remote.join("repository/escape")).unwrap();
        commit(&f.remote);
        f.imports(&[("repository", "main")]);
        assert!(f.load(true).is_err(), "{instructions}");
    }
}

fn cli(f: &Fixture, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_hannes"))
        .current_dir(&f.root)
        .env("XDG_CACHE_HOME", &f.cache)
        .env("NO_COLOR", "1")
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn cli_refresh_cached_plan_and_offline_hook_keep_existing_lifecycle() {
    let f = Fixture::new();
    f.imports(&[("repository", "main")]);
    let path = f.root.join(".hannes/config.toml");
    let text = fs::read_to_string(&path)
        .unwrap()
        .replace("custom = []", "custom = ['quality']");
    let source = f.remote.join("repository/config.toml");
    fs::write(
        &source,
        fs::read_to_string(&source)
            .unwrap()
            .replace("check", "quality"),
    )
    .unwrap();
    commit(&f.remote);
    fs::write(
        &path,
        format!("{text}\n[hooks.git]\npre-commit = \"quality\"\n"),
    )
    .unwrap();
    let plan = cli(&f, &["mach", "was", "--at", "quality", "--plan", "--json"]);
    assert_eq!(plan.status.code(), Some(2));
    assert!(
        !f.root.join(".git/hannes").exists(),
        "missing-state preview must remain read-only"
    );
    let refresh = cli(&f, &["pass", "auf", "--auto-approve"]);
    assert!(
        refresh.status.success(),
        "{}",
        String::from_utf8_lossy(&refresh.stderr)
    );
    assert!(!f.root.join(".hannes/reports/last.json").exists());
    fs::rename(&f.remote, f.remote.with_extension("offline")).unwrap();
    let plan = cli(&f, &["mach", "was", "--at", "quality", "--plan", "--json"]);
    assert!(
        plan.status.success(),
        "{}",
        String::from_utf8_lossy(&plan.stderr)
    );
    let run = cli(&f, &["mach", "was", "--at", "quality"]);
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    let mut paths = vec![
        Path::new(env!("CARGO_BIN_EXE_hannes"))
            .parent()
            .unwrap()
            .to_path_buf(),
    ];
    paths.extend(std::env::split_paths(&std::env::var_os("PATH").unwrap()));
    let hook = Command::new(f.root.join(".git/hooks/pre-commit"))
        .current_dir(&f.root)
        .env("XDG_CACHE_HOME", &f.cache)
        .env("PATH", std::env::join_paths(paths).unwrap())
        .output()
        .unwrap();
    assert!(
        hook.status.success(),
        "{}",
        String::from_utf8_lossy(&hook.stderr)
    );
}

#[test]
fn removed_and_changed_imports_require_their_own_resolution() {
    let f = Fixture::new();
    f.imports(&[("repository", "main")]);
    f.load(true).unwrap();
    f.imports(&[("software", "main")]);
    assert!(format!("{:#}", f.load(false).unwrap_err()).contains("no validated revision"));
    f.load(true).unwrap();
    f.imports(&[]);
    f.load(true).unwrap();
    f.imports(&[("software", "main")]);
    assert!(f.load(false).is_err());
}

#[test]
fn separate_projects_resolving_concurrently_share_one_cache() {
    let f = Fixture::new();
    f.imports(&[("repository", "main")]);
    let second = f.root.with_file_name("second");
    fs::create_dir(&second).unwrap();
    git(&second, &["init", "--quiet"]);
    config::init(&second).unwrap();
    fs::copy(
        f.root.join(".hannes/config.toml"),
        second.join(".hannes/config.toml"),
    )
    .unwrap();
    std::thread::scope(|scope| {
        let a = scope.spawn(|| f.load(true).unwrap());
        let b = scope.spawn(|| external::load(&second, true, Some(&f.cache)).unwrap());
        let a = a.join().unwrap();
        let b = b.join().unwrap();
        assert_eq!(
            serde_json::to_value(a.effective.rules).unwrap(),
            serde_json::to_value(b.effective.rules).unwrap()
        );
    });
}

#[test]
fn sha256_repositories_and_file_urls_use_native_git_resolution() {
    let f = Fixture::new();
    // Recreate only the temporary source using Git's alternate object format.
    fs::remove_dir_all(&f.remote).unwrap();
    fs::create_dir(&f.remote).unwrap();
    git(
        &f.remote,
        &[
            "init",
            "--quiet",
            "--initial-branch=main",
            "--object-format=sha256",
        ],
    );
    definitions(&f.remote, "repository", "git --version");
    let commit = commit(&f.remote);
    assert_eq!(commit.len(), 64);
    f.imports(&[("repository", &commit)]);
    let path = f.root.join(".hannes/config.toml");
    let text = fs::read_to_string(&path).unwrap().replace(
        f.remote.to_str().unwrap(),
        &format!("file://{}", f.remote.display()),
    );
    fs::write(path, text).unwrap();
    f.load(true).unwrap();
    assert_eq!(f.origin().commit, commit);
}

#[test]
fn malformed_configuration_does_not_echo_url_credentials() {
    let f = Fixture::new();
    let path = f.root.join(".hannes/config.toml");
    for text in [
        "[[imports]]\ngit = \"https://user:SECRET@example.invalid/rules\" INVALID\n",
        "[[imports]]\ngit = \"https://user:SECRET@example.invalid/rules\"\npath = \".\"\nref = \"main\"\nenabled = false\n",
    ] {
        fs::write(&path, text).unwrap();
        let error = format!("{:#}", f.load(true).unwrap_err());
        assert!(!error.contains("SECRET"), "{error}");
    }
}

#[cfg(unix)]
#[test]
fn authentication_failure_hides_helper_diagnostics() {
    use std::os::unix::fs::PermissionsExt;
    let f = Fixture::new();
    f.imports(&[("repository", "main")]);
    let path = f.root.join(".hannes/config.toml");
    let text = fs::read_to_string(&path).unwrap().replace(
        f.remote.to_str().unwrap(),
        "ssh://git@example.invalid/rules.git",
    );
    fs::write(path, text).unwrap();
    let ssh = f.root.with_file_name("fake-ssh");
    fs::write(
        &ssh,
        "#!/bin/sh\necho 'Permission denied: SECRET' >&2\nexit 1\n",
    )
    .unwrap();
    fs::set_permissions(&ssh, fs::Permissions::from_mode(0o755)).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_hannes"))
        .current_dir(&f.root)
        .env("XDG_CACHE_HOME", &f.cache)
        .env("GIT_SSH_COMMAND", ssh)
        .env("GIT_SSH_VARIANT", "ssh")
        .args(["pass", "auf", "--auto-approve"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    let output = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.contains("authentication failed"), "{output}");
    assert!(!output.contains("SECRET"), "{output}");
}

#[test]
fn unselected_directories_are_not_interpreted_as_configuration() {
    let f = Fixture::new();
    write(&f.remote, "unselected/broken.toml", "INVALID TOML");
    write(&f.remote, "software/arbitrary.toml", "INVALID TOML");
    commit(&f.remote);
    f.imports(&[("repository", "main")]);
    f.load(true).unwrap();
}

#[test]
fn unsafe_repository_selectors_are_rejected_before_retrieval() {
    let f = Fixture::new();
    f.imports(&[("repository", "main")]);
    let path = f.root.join(".hannes/config.toml");
    let text = fs::read_to_string(&path).unwrap();
    for url in [
        "ext::sh -c touch-UNEXPECTED",
        "custom-helper::somewhere",
        "-h",
        "https://user:SECRET@example.invalid/rules",
        "ssh://user:SECRET@example.invalid/rules",
    ] {
        fs::write(&path, text.replace(f.remote.to_str().unwrap(), url)).unwrap();
        let error = format!("{:#}", f.load(true).unwrap_err());
        assert!(!error.contains("SECRET"), "{error}");
        assert!(!f.cache.join("git").exists(), "{url}");
    }
}

#[cfg(unix)]
#[test]
fn ssh_ports_and_ipv6_authorities_reach_host_git_without_password_confusion() {
    use std::os::unix::fs::PermissionsExt;
    let f = Fixture::new();
    f.imports(&[("repository", "main")]);
    let path = f.root.join(".hannes/config.toml");
    let text = fs::read_to_string(&path).unwrap();
    let ssh = f.root.with_file_name("fake-ssh");
    fs::write(&ssh, "#!/bin/sh\necho 'Permission denied' >&2\nexit 1\n").unwrap();
    fs::set_permissions(&ssh, fs::Permissions::from_mode(0o755)).unwrap();
    for url in [
        "ssh://git@example.invalid:2222/rules.git",
        "ssh://example.invalid:2222/rules.git",
        "ssh://[::1]:2222/rules.git",
    ] {
        fs::write(&path, text.replace(f.remote.to_str().unwrap(), url)).unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_hannes"))
            .current_dir(&f.root)
            .env("XDG_CACHE_HOME", &f.cache)
            .env("GIT_SSH_COMMAND", &ssh)
            .env("GIT_SSH_VARIANT", "ssh")
            .args(["pass", "auf", "--auto-approve"])
            .output()
            .unwrap();
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("authentication failed"),
            "{url}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
fn separate_repositories_with_identical_paths_and_ids_remain_distinct() {
    let f = Fixture::new();
    let other = f.remote.with_file_name("other");
    fs::create_dir(&other).unwrap();
    git(&other, &["init", "--quiet", "--initial-branch=main"]);
    definitions(&other, "repository", "git --version");
    commit(&other);
    f.imports(&[("repository", "main")]);
    let path = f.root.join(".hannes/config.toml");
    let text = fs::read_to_string(&path).unwrap();
    fs::write(
        path,
        format!(
            "{text}\n[[imports]]\ngit = {:?}\npath = \"repository\"\nref = \"main\"\n",
            other.to_str().unwrap()
        ),
    )
    .unwrap();
    let project = f.load(true).unwrap();
    let selected = planner::build(&project, Some("check"), false)
        .unwrap()
        .plan
        .scheduled;
    assert_eq!(selected.len(), 2);
    assert_ne!(selected[0].id, selected[1].id);
    assert_ne!(
        selected[0]
            .origin
            .as_ref()
            .unwrap()
            .external
            .as_ref()
            .unwrap()
            .repository,
        selected[1]
            .origin
            .as_ref()
            .unwrap()
            .external
            .as_ref()
            .unwrap()
            .repository
    );
}

#[test]
fn moved_tags_refresh_but_cached_execution_remains_on_the_validated_commit() {
    let f = Fixture::new();
    git(&f.remote, &["tag", "shared"]);
    f.imports(&[("repository", "shared")]);
    f.load(true).unwrap();
    let original = f.origin().commit;
    write(&f.remote, "repository/change.md", "new tag target");
    let updated = commit(&f.remote);
    git(&f.remote, &["tag", "--force", "shared"]);
    assert_eq!(f.origin().commit, original);
    f.load(true).unwrap();
    assert_eq!(f.origin().commit, updated);
}

fn mapped_imports(f: &Fixture, mappings: &[(&str, &str)], custom: &[&str]) {
    let mut text = format!("project = 'My_App'\n[execution_points]\ncustom = {custom:?}\n");
    for (path, mapping) in mappings {
        text += &format!(
            "[[imports]]\ngit = {:?}\npath = {path:?}\nref = 'main'\n{mapping}\n",
            f.remote.to_str().unwrap()
        );
    }
    write(&f.root, ".hannes/config.toml", &text);
}
fn source_points(f: &Fixture, path: &str, points: &[&str]) {
    let file = f.remote.join(path).join("config.toml");
    let text = fs::read_to_string(&file)
        .unwrap()
        .replace("['check']", &format!("{points:?}"));
    fs::write(file, text).unwrap();
}

#[test]
fn independent_mappings_resolve_inheritance_many_to_one_and_offline_execution() {
    let f = Fixture::new();
    source_points(&f, "repository", &["lint", "test"]);
    source_points(&f, "software", &["lint"]);
    let member = f.remote.join("repository/checks/required-files/rule.toml");
    fs::write(
        &member,
        fs::read_to_string(&member)
            .unwrap()
            .replace("[check]", "run_at = ['own']\n[check]"),
    )
    .unwrap();
    // A standalone Rule's own assignments must also be mapped.
    write(
        &f.remote,
        "repository/checks/standalone/rule.toml",
        "id = 'standalone'\ndescription = 'Standalone'\nrun_at = ['lint', 'test']\nblocking = false\n[check]\nentrypoint = 'index.sh'\ninterpreter = 'sh'\n",
    );
    write(
        &f.remote,
        "repository/checks/standalone/index.sh",
        "git --version\n",
    );
    // Disabled Rules validate, keep metadata and never enter execution results.
    write(
        &f.remote,
        "repository/checks/disabled/rule.toml",
        "id = 'disabled'\ndescription = 'Disabled'\nenabled = false\nrun_at = ['test']\n[check]\nentrypoint = 'index.sh'\ninterpreter = 'sh'\n",
    );
    write(&f.remote, "repository/checks/disabled/index.sh", "exit 1\n");
    commit(&f.remote);
    mapped_imports(
        &f,
        &[
            (
                "repository",
                "[imports.execution_points]\nlint = 'verify'\ntest = 'verify'\nown = 'audit'",
            ),
            ("software", "[imports.execution_points]\nlint = 'audit'"),
        ],
        &["verify", "audit"],
    );
    let project = f.load(true).unwrap();
    let verify = planner::build(&project, Some("verify"), false)
        .unwrap()
        .plan;
    assert_eq!(verify.scheduled.len(), 2);
    assert_eq!(
        planner::build(&project, Some("audit"), false)
            .unwrap()
            .plan
            .scheduled
            .len(),
        1
    );
    assert!(
        project
            .checks
            .iter()
            .any(|r| r.id.ends_with(":standalone") && !r.blocking)
    );
    let identities = project.effective.rules.keys().cloned().collect::<Vec<_>>();
    let before = serde_json::to_value(&project.effective.rules).unwrap();
    let contents = f.contents();
    assert!(
        fs::read_to_string(contents.join("repository/config.toml"))
            .unwrap()
            .contains("lint")
    );
    let state = f.state();
    fs::rename(&f.remote, f.remote.with_extension("offline")).unwrap();
    let offline = f.load(false).unwrap();
    assert_eq!(
        before,
        serde_json::to_value(&offline.effective.rules).unwrap()
    );
    let report = hannes::verify_project_at(
        offline,
        &Never,
        hannes::VerificationMode::AllRules,
        Some("verify"),
        &mut |_| {},
    )
    .unwrap();
    assert_eq!(report.overall, Status::Pass);
    assert_eq!(report.results.len(), 2);
    assert_eq!(f.state(), state);
    // Editing only mappings needs no refresh, changes no Rule identity or cache.
    mapped_imports(
        &f,
        &[
            (
                "repository",
                "[imports.execution_points]\nlint = 'verify'\ntest = 'verify'\nown = 'verify'",
            ),
            ("software", "[imports.execution_points]\nlint = 'verify'"),
        ],
        &["verify"],
    );
    let project = f.load(false).unwrap();
    assert_eq!(
        planner::build(&project, Some("verify"), false)
            .unwrap()
            .plan
            .scheduled
            .len(),
        3
    );
    assert_eq!(
        project.effective.rules.keys().cloned().collect::<Vec<_>>(),
        identities
    );
    assert_eq!(f.state(), state);
}

#[test]
fn invalid_import_mappings_fail_with_source_target_and_import_context() {
    let f = Fixture::new();
    source_points(&f, "repository", &["quality"]);
    commit(&f.remote);
    mapped_imports(
        &f,
        &[(
            "repository",
            "[imports.execution_points]\nquality = 'verify'",
        )],
        &["verify"],
    );
    f.load(true).unwrap();
    let state = f.state();
    for (source, target, reason) in [
        ("check", "verify", "built-in source"),
        ("pre-commit", "verify", "built-in source"),
        ("commit-msg", "verify", "built-in source"),
        ("pre-push", "verify", "built-in source"),
        ("quality", "check", "built-in target"),
        ("quality", "pre-commit", "built-in target"),
        ("quality", "commit-msg", "built-in target"),
        ("quality", "pre-push", "built-in target"),
        ("unknown", "verify", "unknown source"),
        ("quality", "missing", "unknown target"),
        ("", "verify", "invalid execution-point"),
        ("quality", "", "invalid execution-point"),
        ("Quality", "verify", "invalid execution-point"),
        ("quality", "bad/name", "invalid execution-point"),
    ] {
        mapped_imports(
            &f,
            &[(
                "repository",
                &format!("[imports.execution_points]\n{source:?} = {target:?}"),
            )],
            &["verify"],
        );
        let error = format!("{:#}", f.load(false).unwrap_err());
        assert!(
            error.contains("repository")
                && error.contains(source)
                && error.contains(target)
                && error.contains(reason),
            "{error}"
        );
        assert_eq!(f.state(), state);
    }
    for structure in [
        "execution_points = []",
        "execution_points = 'verify'",
        "execution_points = { quality = ['verify'] }",
        "execution_points = { quality = { target = 'verify' } }",
    ] {
        mapped_imports(&f, &[("repository", structure)], &["verify"]);
        assert!(f.load(false).is_err(), "{structure}");
    }
    mapped_imports(
        &f,
        &[(
            "repository",
            "[imports.execution_points]\nquality = 'verify'",
        )],
        &[],
    );
    assert!(format!("{:#}", f.load(false).unwrap_err()).contains("declare"));
}

#[test]
fn unmapped_custom_points_require_consumer_declarations_and_keep_names() {
    let f = Fixture::new();
    source_points(&f, "repository", &["quality", "audit"]);
    commit(&f.remote);
    mapped_imports(&f, &[("repository", "")], &["quality", "audit"]);
    let project = f.load(true).unwrap();
    assert_eq!(
        planner::build(&project, Some("quality"), false)
            .unwrap()
            .plan
            .scheduled
            .len(),
        1
    );
    mapped_imports(
        &f,
        &[(
            "repository",
            "[imports.execution_points]\nquality = 'verify'",
        )],
        &["verify", "audit"],
    );
    let project = f.load(false).unwrap();
    assert_eq!(
        planner::build(&project, Some("audit"), false)
            .unwrap()
            .plan
            .scheduled
            .len(),
        1
    );
    mapped_imports(
        &f,
        &[(
            "repository",
            "[imports.execution_points]\nquality = 'verify'",
        )],
        &["verify"],
    );
    assert!(
        format!("{:#}", f.load(false).unwrap_err()).contains("unknown resulting execution point")
    );
}

#[cfg(unix)]
#[test]
fn built_in_hooks_execute_imported_and_local_rules_with_message_context() {
    let f = Fixture::new();
    source_points(&f, "repository", &["pre-commit", "commit-msg", "pre-push"]);
    write(
        &f.remote,
        "repository/checks/required-files/index.sh",
        "if [ \"$HANNES_GIT_HOOK\" = commit-msg ]; then\n  test \"$HANNES_GIT_HOOK_ARG_COUNT\" = 1 && test -s \"$HANNES_GIT_HOOK_ARG_1\"\nelse\n  git --version\nfi\n",
    );
    commit(&f.remote);
    f.imports(&[("repository", "main")]);
    write(
        &f.root,
        ".hannes/checks/local/rule.toml",
        "id = 'local'\ndescription = 'Standalone local hook Rule'\nrun_at = ['pre-commit']\n[check]\nentrypoint = 'index.sh'\ninterpreter = 'sh'\n",
    );
    write(&f.root, ".hannes/checks/local/index.sh", "git --version\n");
    write(
        &f.root,
        ".hannes/rule-sets/local.toml",
        "id = 'local-set'\ndescription = 'Local set hook assignments'\nrules = ['always-ok']\nrun_at = ['pre-commit', 'commit-msg', 'pre-push']\n",
    );
    let setup = cli(&f, &["pass", "auf", "--auto-approve"]);
    assert!(
        setup.status.success(),
        "{}",
        String::from_utf8_lossy(&setup.stderr)
    );
    let paths = std::env::join_paths(
        std::iter::once(
            Path::new(env!("CARGO_BIN_EXE_hannes"))
                .parent()
                .unwrap()
                .to_path_buf(),
        )
        .chain(std::env::split_paths(&std::env::var_os("PATH").unwrap())),
    )
    .unwrap();
    let message = f.root.join(".git/COMMIT_EDITMSG");
    fs::write(&message, "test: message\n").unwrap();
    fs::rename(&f.remote, f.remote.with_extension("offline")).unwrap();
    for event in ["pre-commit", "commit-msg", "pre-push"] {
        let path = f.root.join(".git/hooks").join(event);
        assert!(path.is_file());
        let mut command = Command::new(path);
        command
            .current_dir(&f.root)
            .env("XDG_CACHE_HOME", &f.cache)
            .env("PATH", &paths);
        if event == "commit-msg" {
            command.arg(&message);
        }
        let output = command.output().unwrap();
        assert!(
            output.status.success(),
            "{event}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let report = hannes::report::read(&f.root).unwrap();
        assert!(report.results.iter().any(|r| r.id.starts_with("git:")));
        assert!(report.results.iter().any(|r| r.id == "always-ok"));
        assert_eq!(
            report.results.len(),
            if event == "pre-commit" { 3 } else { 2 }
        );
    }
    assert!(!f.root.join(".git/hooks/check").exists());
    assert!(!f.root.join(".github").exists());
}

#[test]
fn check_only_configuration_creates_no_hooks_or_workflows() {
    let f = Fixture::new();
    f.imports(&[]);
    let file = f.root.join(".hannes/checks/always-ok/rule.toml");
    fs::write(
        &file,
        fs::read_to_string(&file)
            .unwrap()
            .replace("pre-push", "check"),
    )
    .unwrap();
    let setup = cli(&f, &["pass", "auf", "--auto-approve"]);
    assert!(
        setup.status.success(),
        "{}",
        String::from_utf8_lossy(&setup.stderr)
    );
    for event in ["pre-commit", "commit-msg", "pre-push", "check"] {
        assert!(!f.root.join(".git/hooks").join(event).exists());
    }
    let run = cli(&f, &["mach", "was", "--at", "check"]);
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(hannes::report::read(&f.root).unwrap().results.len(), 1);
    assert!(!f.root.join(".github").exists());
}
