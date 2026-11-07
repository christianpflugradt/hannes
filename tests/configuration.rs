use hannes::{
    config, external, planner,
    provider::{Provider, Request},
    report::{self, Status},
};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};
use tempfile::TempDir;

fn write(root: &Path, path: &str, text: &str) {
    let path = root.join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}
fn git(root: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
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
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap().trim().into()
}
fn project() -> TempDir {
    let d = tempfile::tempdir().unwrap();
    git(d.path(), &["init", "--quiet", "--initial-branch=main"]);
    write(
        d.path(),
        ".hannes/config.toml",
        "project = 'My_App'\n[execution_points]\ncustom = ['release']\n",
    );
    write(d.path(), "README.md", "# My_App\n");
    d
}
fn check(root: &Path, id: &str, attributes: &str, implementation: &str, script: &str) {
    write(
        root,
        &format!(".hannes/checks/{id}/rule.toml"),
        &format!(
            "id = {id:?}\ndescription = 'Test check'\n{attributes}\n[check]\n{implementation}\n"
        ),
    );
    write(root, &format!(".hannes/checks/{id}/index.sh"), script);
}
fn review(root: &Path, attributes: &str) {
    write(
        root,
        ".hannes/reviews/design/rule.toml",
        &format!(
            "id = 'design'\ndescription = 'Review design'\n{attributes}\n[review]\nprompt = 'prompt.md'\n"
        ),
    );
    write(
        root,
        ".hannes/reviews/design/prompt.md",
        "Inspect documented expectations.\n",
    );
}
fn error(root: &Path) -> String {
    format!("{:#}", config::load(root).unwrap_err())
}
struct Never;
impl Provider for Never {
    fn review(&self, _: &Request<'_>) -> anyhow::Result<String> {
        panic!("unexpected provider call")
    }
}
struct Mock(&'static str);
impl Provider for Mock {
    fn review(&self, request: &Request<'_>) -> anyhow::Result<String> {
        assert_eq!(
            request.reviews[0].text,
            "Inspect documented expectations.\n"
        );
        if self.0 == "unavailable" {
            anyhow::bail!("provider unavailable");
        }
        if self.0 == "invalid" {
            return Ok("invalid JSON".into());
        }
        Ok(serde_json::json!({"reviews": [{"id": request.reviews[0].id, "status": self.0,
            "reason": "Documented behavior", "findings": [{"message": "Evidence", "path": "README.md", "line": 1}]}]}).to_string())
    }
}

#[test]
fn fixed_discovery_and_defaults() {
    let d = project();
    assert!(config::load(d.path()).unwrap().checks.is_empty());
    check(
        d.path(),
        "readme",
        "run_at = ['check']",
        "entrypoint = 'index.sh'",
        "exit 0\n",
    );
    // Supporting data and unrelated TOML documents are never parsed as governance.
    write(
        d.path(),
        ".hannes/checks/readme/gitleaks.toml",
        "arbitrary data, not TOML",
    );
    write(d.path(), ".hannes/arbitrary/invalid.toml", "not TOML");
    let p = config::load(d.path()).unwrap();
    assert!(p.checks[0].enabled);
    assert!(p.checks[0].blocking);
    assert_eq!(p.checks[0].timeout_seconds, 30);
    let plan = planner::build(&p, Some("check"), false).unwrap().plan;
    assert_eq!(plan.plan_schema_version, 3);
    assert_eq!(plan.scheduled.len(), 1);
    assert!(plan.scheduled[0].blocking);
    write(
        d.path(),
        ".hannes/checks/readme/gitleaks.toml",
        "changed bundled data",
    );
    let changed = config::load(d.path()).unwrap();
    assert_ne!(
        plan.plan_id,
        planner::build(&changed, Some("check"), false)
            .unwrap()
            .plan
            .plan_id
    );
}

#[test]
fn strict_configuration_and_implementation_validation() {
    for (attributes, implementation) in [
        ("run_at = ['unknown']", "entrypoint = 'index.sh'"),
        (
            "run_at = ['check']\nunknown = true",
            "entrypoint = 'index.sh'",
        ),
        (
            "run_at = ['check']",
            "entrypoint = 'index.sh'\nunknown = true",
        ),
        (
            "run_at = ['check']",
            "entrypoint = 'index.sh'\ninterpreter = ''",
        ),
        (
            "run_at = ['check']",
            "entrypoint = 'index.sh'\ntimeout_seconds = 0",
        ),
        ("", "entrypoint = 'index.sh'"),
    ] {
        let d = project();
        check(d.path(), "readme", attributes, implementation, "exit 0");
        assert!(
            config::load(d.path()).is_err(),
            "{attributes}: {implementation}"
        );
    }
    let d = project();
    for text in [
        "version = 4\nproject = 'My_App'",
        "version = 3\nproject = 'My_App'",
        "version = 99\nproject = 'My_App'",
        "project = 'My_App'\nunknown = true",
        "project = 'My_App'\n[[checks]]\nid = 'legacy'",
    ] {
        write(d.path(), ".hannes/config.toml", text);
        assert!(config::load(d.path()).is_err());
    }
}

#[test]
fn resources_and_definitions_are_required_including_disabled_rules() {
    let d = project();
    fs::create_dir_all(d.path().join(".hannes/checks/readme")).unwrap();
    assert!(error(d.path()).contains("rule.toml"));
    check(
        d.path(),
        "readme",
        "enabled = false",
        "entrypoint = 'missing.sh'",
        "exit 0",
    );
    assert!(config::load(d.path()).is_err());
    fs::remove_dir_all(d.path().join(".hannes/checks")).unwrap();
    fs::create_dir_all(d.path().join(".hannes/reviews/design")).unwrap();
    assert!(error(d.path()).contains("rule.toml"));
    review(d.path(), "enabled = false");
    fs::remove_file(d.path().join(".hannes/reviews/design/prompt.md")).unwrap();
    assert!(config::load(d.path()).is_err());
}

#[test]
fn resource_filenames_are_confined() {
    for name in [
        "",
        ".",
        "..",
        "../index.sh",
        "/tmp/index.sh",
        "helpers/index.sh",
        "helpers\\index.sh",
        "C:\\index.sh",
    ] {
        let d = project();
        check(
            d.path(),
            "readme",
            "run_at = ['check']",
            &format!("entrypoint = {name:?}"),
            "exit 0",
        );
        assert!(config::load(d.path()).is_err(), "accepted {name:?}");
        fs::remove_dir_all(d.path().join(".hannes/checks")).unwrap();
        review(d.path(), "run_at = ['check']");
        let path = d.path().join(".hannes/reviews/design/rule.toml");
        fs::write(
            &path,
            fs::read_to_string(&path)
                .unwrap()
                .replace("'prompt.md'", &format!("{name:?}")),
        )
        .unwrap();
        assert!(config::load(d.path()).is_err(), "accepted prompt {name:?}");
    }
    let d = project();
    check(
        d.path(),
        "readme",
        "run_at = ['check']",
        "entrypoint = 'helpers'",
        "exit 0",
    );
    fs::create_dir(d.path().join(".hannes/checks/readme/helpers")).unwrap();
    assert!(config::load(d.path()).is_err());
}

#[test]
fn local_sets_override_union_and_validate_scope() {
    let d = project();
    check(
        d.path(),
        "readme",
        "run_at = ['pre-commit']",
        "entrypoint = 'index.sh'",
        "exit 0",
    );
    for (set, at) in [("repository", "check"), ("software", "release")] {
        write(
            d.path(),
            &format!(".hannes/rule-sets/{set}.toml"),
            &format!(
                "id = {set:?}\ndescription = 'Reusable rules'\nrules = ['readme']\nrun_at = [{at:?}]"
            ),
        );
    }
    let p = config::load(d.path()).unwrap();
    assert_eq!(
        p.effective.rules["readme"]
            .execution_points
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["check", "release"]
    );
    assert_eq!(
        planner::build(&p, Some("check"), false)
            .unwrap()
            .plan
            .scheduled
            .len(),
        1
    );
    assert!(
        planner::build(&p, Some("pre-commit"), false)
            .unwrap()
            .plan
            .scheduled
            .is_empty()
    );
    let path = ".hannes/rule-sets/repository.toml";
    let original = fs::read_to_string(d.path().join(path)).unwrap();
    for attribute in [
        "enabled = true",
        "blocking = false",
        "git = 'source'",
        "path = 'source'",
        "ref = 'main'",
    ] {
        write(d.path(), path, &format!("{original}\n{attribute}"));
        assert!(config::load(d.path()).is_err(), "accepted {attribute}");
    }
    write(d.path(), path, &original.replace("'readme'", "'missing'"));
    assert!(config::load(d.path()).is_err());
    write(d.path(), path, &original.replace("'readme'", "'software'"));
    assert!(config::load(d.path()).is_err());
}

#[test]
fn duplicate_ids_and_exactly_one_implementation() {
    let d = project();
    check(
        d.path(),
        "design",
        "run_at = ['check']",
        "entrypoint = 'index.sh'",
        "exit 0",
    );
    review(d.path(), "run_at = ['check']");
    assert!(error(d.path()).contains("duplicate"));
    fs::remove_dir_all(d.path().join(".hannes/reviews")).unwrap();
    let path = ".hannes/checks/design/rule.toml";
    let original = fs::read_to_string(d.path().join(path)).unwrap();
    write(
        d.path(),
        path,
        &format!("{original}\n[review]\nprompt = 'prompt.md'"),
    );
    assert!(config::load(d.path()).is_err());
}

#[cfg(unix)]
#[test]
fn direct_and_interpreter_execution_keep_consumer_and_resources() {
    use std::os::unix::fs::PermissionsExt;
    for interpreter in ["", "interpreter = 'sh'"] {
        let d = project();
        check(
            d.path(),
            "readme",
            "run_at = ['check']",
            &format!("entrypoint = 'index.sh'\n{interpreter}"),
            "#!/bin/sh\ntest -s README.md || exit 1\ncat \"$HANNES_RULE_DIRECTORY/data.txt\"\n",
        );
        write(d.path(), ".hannes/checks/readme/data.txt", "bundled data\n");
        if interpreter.is_empty() {
            fs::set_permissions(
                d.path().join(".hannes/checks/readme/index.sh"),
                fs::Permissions::from_mode(0o755),
            )
            .unwrap();
        }
        let p = config::load(d.path()).unwrap();
        let result = hannes::verify_project_at(
            p,
            &Never,
            hannes::VerificationMode::AllRules,
            Some("check"),
            &mut |_| {},
        )
        .unwrap();
        assert_eq!(result.overall, Status::Pass);
        assert_eq!(
            result.results[0].stdout.as_ref().unwrap().text,
            "bundled data\n"
        );
        report::read(d.path()).unwrap();
    }
}

#[cfg(unix)]
#[test]
fn blocking_failures_errors_and_disabled_rules() {
    for (attributes, implementation, script, expected, item_status) in [
        (
            "",
            "interpreter = 'sh'",
            "exit 1",
            Status::Fail,
            Status::Fail,
        ),
        (
            "blocking = false",
            "interpreter = 'sh'",
            "exit 1",
            Status::Pass,
            Status::Fail,
        ),
        (
            "blocking = false",
            "interpreter = 'sh'",
            "exit 2",
            Status::Error,
            Status::Error,
        ),
        (
            "blocking = false",
            "interpreter = 'hannes-missing-interpreter'",
            "exit 0",
            Status::Error,
            Status::Error,
        ),
        (
            "blocking = false",
            "interpreter = 'sh'\ntimeout_seconds = 1",
            "sleep 4",
            Status::Error,
            Status::Error,
        ),
        (
            "blocking = false",
            "",
            "exit 0",
            Status::Error,
            Status::Error,
        ),
    ] {
        let d = project();
        check(
            d.path(),
            "readme",
            &format!("run_at = ['pre-push']\n{attributes}"),
            &format!("entrypoint = 'index.sh'\n{implementation}"),
            script,
        );
        let result = hannes::verify(d.path(), &Never).unwrap();
        assert_eq!(result.overall, expected, "{implementation} {script}");
        assert_eq!(result.results[0].status, item_status);
        let saved = report::read(d.path()).unwrap();
        assert_eq!(saved.schema_version, 4);
        assert_eq!(saved.results[0].blocking, attributes.is_empty());
        if expected == Status::Pass {
            assert!(hannes::terminal::render(&saved, false).contains("Non-blocking"));
        }
    }
    let d = project();
    check(
        d.path(),
        "readme",
        "enabled = false",
        "entrypoint = 'index.sh'",
        "exit 1",
    );
    let p = config::load(d.path()).unwrap();
    let plan = planner::build(&p, None, false).unwrap().plan;
    assert!(plan.scheduled.is_empty());
    assert_eq!(plan.disabled_rules[0].id, "readme");
    let result = hannes::verify(d.path(), &Never).unwrap();
    assert!(result.results.is_empty());
}

#[test]
fn reviews_preserve_provider_contract_and_nonblocking_failures() {
    for (attributes, verdict, expected) in [
        ("", "PASS", Status::Pass),
        ("", "FAIL", Status::Fail),
        ("blocking = false", "FAIL", Status::Pass),
        ("blocking = false", "ERROR", Status::Error),
        ("blocking = false", "unavailable", Status::Error),
        ("blocking = false", "invalid", Status::Error),
    ] {
        let d = project();
        review(d.path(), &format!("run_at = ['pre-push']\n{attributes}"));
        let result = hannes::verify(d.path(), &Mock(verdict)).unwrap();
        assert_eq!(result.overall, expected, "{verdict}");
        report::read(d.path()).unwrap();
    }
}

#[cfg(unix)]
#[test]
fn advisory_check_does_not_skip_reviews() {
    let d = project();
    check(
        d.path(),
        "readme",
        "run_at = ['pre-push']\nblocking = false",
        "entrypoint = 'index.sh'\ninterpreter = 'sh'",
        "exit 1",
    );
    review(d.path(), "run_at = ['pre-push']");
    let result = hannes::verify(d.path(), &Mock("PASS")).unwrap();
    assert_eq!(result.overall, Status::Pass);
    assert_eq!(result.results.len(), 2);
    assert_eq!(result.results[1].status, Status::Pass);
}

#[cfg(unix)]
#[test]
fn all_designated_paths_reject_symlinks() {
    use std::os::unix::fs::symlink;
    for relative in [
        "checks/readme/index.sh",
        "checks/readme/rule.toml",
        "checks/readme",
        "checks",
        "config.toml",
        "reviews/design/prompt.md",
        "reviews/design/rule.toml",
        "reviews/design",
        "reviews",
    ] {
        let d = project();
        check(
            d.path(),
            "readme",
            "run_at = ['check']",
            "entrypoint = 'index.sh'",
            "exit 0",
        );
        review(d.path(), "run_at = ['check']");
        let source = d.path().join(".hannes").join(relative);
        let outside = d.path().join("outside");
        fs::rename(&source, &outside).unwrap();
        symlink(&outside, &source).unwrap();
        assert!(config::load(d.path()).is_err(), "followed {relative}");
    }
}

struct External {
    _temp: TempDir,
    root: PathBuf,
    remote: PathBuf,
    cache: PathBuf,
}
impl External {
    fn new() -> Self {
        let d = project();
        let root = d.path().canonicalize().unwrap();
        let remote = root.join("remote");
        let cache = root.join("cache");
        fs::create_dir(&remote).unwrap();
        fs::create_dir(&cache).unwrap();
        git(&remote, &["init", "--quiet", "--initial-branch=main"]);
        for name in ["repository", "software"] {
            write(
                &remote,
                &format!("{name}/config.toml"),
                &format!(
                    "id = {name:?}\ndescription = 'Reusable set'\nrules = ['readme']\nrun_at = ['check']"
                ),
            );
            write(
                &remote,
                &format!("{name}/checks/readme/rule.toml"),
                "id = 'readme'\ndescription = 'Consumer README'\n[check]\nentrypoint = 'index.sh'\ninterpreter = 'sh'\n",
            );
            write(
                &remote,
                &format!("{name}/checks/readme/index.sh"),
                "test -s README.md || exit 1\ntest -s \"$HANNES_RULE_DIRECTORY/data.txt\" || exit 1\n",
            );
            write(
                &remote,
                &format!("{name}/checks/readme/data.txt"),
                "bundled resource\n",
            );
        }
        let f = Self {
            _temp: d,
            root,
            remote,
            cache,
        };
        f.commit();
        f
    }
    fn commit(&self) -> String {
        git(&self.remote, &["add", "."]);
        git(&self.remote, &["commit", "--quiet", "-m", "fixture"]);
        git(&self.remote, &["rev-parse", "HEAD"])
    }
    fn imports(&self, selections: &[(&str, &str)]) {
        let mut text = "project = 'My_App'\n[execution_points]\ncustom = []\n".to_string();
        for (path, reference) in selections {
            text += &format!(
                "[[imports]]\ngit = {:?}\npath = {path:?}\nref = {reference:?}\n",
                self.remote.to_str().unwrap()
            );
        }
        write(&self.root, ".hannes/config.toml", &text);
    }
    fn load(&self, refresh: bool) -> anyhow::Result<config::Project> {
        external::load(&self.root, refresh, Some(&self.cache))
    }
}

#[cfg(unix)]
#[test]
fn external_refs_cache_offline_isolation_and_consumer_execution() {
    let f = External::new();
    git(&f.remote, &["tag", "v1.0.0"]);
    let commit = git(&f.remote, &["rev-parse", "HEAD"]);
    // Local and two external scopes all use the same rule ID without collision.
    check(
        &f.root,
        "readme",
        "run_at = ['check']",
        "entrypoint = 'index.sh'\ninterpreter = 'sh'",
        "exit 0",
    );
    for reference in ["main", "v1.0.0", &commit] {
        f.imports(&[("repository", reference), ("software", reference)]);
        let p = f.load(true).unwrap();
        assert_eq!(p.checks.len(), 3);
        let origins: Vec<_> = p
            .effective
            .rules
            .values()
            .filter_map(|r| r.external.as_ref())
            .collect();
        assert_eq!(origins.len(), 2);
        assert!(origins.iter().all(|o| o.commit == commit));
        assert_eq!(p.checks.len(), 3);
        let p = f.load(false).unwrap();
        assert_eq!(
            planner::build(&p, Some("check"), false)
                .unwrap()
                .plan
                .scheduled
                .len(),
            3
        );
    }
    fs::rename(&f.remote, f.root.join("offline-remote")).unwrap();
    let p = f.load(false).unwrap();
    // Keep test fixture cache/remote contents outside the consumer snapshot.
    write(&f.root, ".gitignore", "cache/\noffline-remote/\n");
    let result = hannes::verify_project_at(
        p,
        &Never,
        hannes::VerificationMode::AllRules,
        Some("check"),
        &mut |_| {},
    )
    .unwrap();
    assert_eq!(result.overall, Status::Pass);
    assert_eq!(result.results.len(), 3);
    report::read(&f.root).unwrap();
}

#[test]
fn external_invalid_roots_import_overrides_and_scope_references_fail() {
    let f = External::new();
    f.imports(&[("missing", "main")]);
    assert!(f.load(true).is_err());
    f.imports(&[("repository", "main")]);
    let file = f.remote.join("repository/config.toml");
    let original = fs::read_to_string(&file).unwrap();
    for suffix in [
        "\nproject = 'Consumer'",
        "\nimports = []",
        "\nenabled = true",
        "\nblocking = false",
    ] {
        fs::write(&file, format!("{original}{suffix}")).unwrap();
        f.commit();
        assert!(f.load(true).is_err(), "accepted {suffix}");
    }
    fs::write(&file, original.replace("'readme'", "'sibling-only'")).unwrap();
    f.commit();
    assert!(f.load(true).is_err());
    fs::write(&file, &original).unwrap();
    fs::remove_file(f.remote.join("repository/checks/readme/rule.toml")).unwrap();
    f.commit();
    assert!(f.load(true).is_err());
    let central = f.root.join(".hannes/config.toml");
    let text = fs::read_to_string(&central).unwrap();
    for extra in [
        "enabled = false",
        "run_at = ['check']",
        "id = 'override'",
        "blocking = false",
        "description = 'Override'",
    ] {
        fs::write(&central, format!("{text}{extra}\n")).unwrap();
        assert!(f.load(true).is_err(), "accepted import {extra}");
    }
}

#[test]
fn initialization_is_idempotent() {
    let d = tempfile::tempdir().unwrap();
    git(d.path(), &["init", "--quiet"]);
    assert!(config::init(d.path()).unwrap() > 0);
    config::load(d.path()).unwrap();
    assert!(
        !fs::read_to_string(d.path().join(".hannes/config.toml"))
            .unwrap()
            .contains("version")
    );
    assert!(
        d.path()
            .join(".hannes/checks/always-ok/rule.toml")
            .is_file()
    );
    assert_eq!(config::init(d.path()).unwrap(), 0);
    assert_eq!(
        hannes::verify(d.path(), &Never).unwrap().overall,
        Status::Pass
    );
    report::read(d.path()).unwrap();
}

#[test]
fn checks_only_requires_complete_selection() {
    let d = project();
    review(d.path(), "run_at = ['pre-push']\nblocking = false");
    let p = config::load(d.path()).unwrap();
    let plan = planner::build(&p, None, true).unwrap().plan;
    assert!(!plan.selection_complete);
    assert!(!plan.executable());
    assert!(
        hannes::verify_project_with_mode(
            p,
            &Never,
            hannes::VerificationMode::ChecksOnly,
            &mut |_| {}
        )
        .is_err()
    );
}

#[test]
fn prepared_input_is_shared_without_rereading_resources() {
    let d = project();
    review(d.path(), "run_at = ['pre-push']");
    let path = d.path().join(".hannes/reviews/design/rule.toml");
    let text = fs::read_to_string(&path).unwrap();
    fs::write(
        &path,
        format!("{text}\n[[review.context]]\npath = 'README.md'\n"),
    )
    .unwrap();
    let p = config::load(d.path()).unwrap();
    let prepared = planner::build(&p, None, false).unwrap();
    let input = prepared.review_input.as_ref().unwrap();
    assert!(input.excerpts[0].text.contains("My_App"));
    // Rendering the prepared request performs no filesystem reread.
    fs::remove_file(d.path().join("README.md")).unwrap();
    fs::remove_file(d.path().join(".hannes/reviews/design/prompt.md")).unwrap();
    assert!(input.excerpts[0].text.contains("My_App"));
    assert_eq!(p.reviews[0].text, "Inspect documented expectations.\n");
    let rendered = hannes::provider::prompt(&Request {
        project: &p,
        reviews: &[&p.reviews[0]],
        snapshot: &prepared.snapshot,
        input,
    })
    .unwrap();
    assert!(rendered.contains("My_App"));
}

#[test]
fn saved_reports_reject_inconsistent_metadata() {
    let d = project();
    review(d.path(), "run_at = ['pre-push']\nblocking = false");
    let r = hannes::verify(d.path(), &Mock("FAIL")).unwrap();
    let original = serde_json::to_value(r).unwrap();
    for case in ["missing-rule", "blocking", "point", "plan", "overall"] {
        let mut value = original.clone();
        match case {
            "missing-rule" => value["results"] = serde_json::json!([]),
            "blocking" => value["results"][0]["blocking"] = true.into(),
            "point" => value["plan"]["at"] = "release".into(),
            "plan" => value["plan"] = serde_json::Value::Null,
            _ => value["overall"] = "FAIL".into(),
        }
        write(d.path(), ".hannes/reports/last.json", &value.to_string());
        assert!(report::read(d.path()).is_err(), "accepted {case}");
    }
}
