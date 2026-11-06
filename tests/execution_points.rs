use hannes::{
    VerificationMode, config, planner,
    provider::{Provider, Request},
    report::{self, Status},
    rules::{self, Assignment},
};
use std::{fs, path::PathBuf, process::Command};
use tempfile::{TempDir, tempdir};
struct Never;
impl Provider for Never {
    fn review(&self, _: &Request<'_>) -> anyhow::Result<String> {
        panic!("no provider call expected")
    }
}
fn assignment(id: &str, enabled: bool, at: &[&str]) -> Assignment {
    Assignment {
        id: id.into(),
        enabled,
        run_at: at.iter().map(|s| s.to_string()).collect(),
        source: PathBuf::from(format!(".hannes/{id}.toml")),
    }
}
fn set(id: &str, enabled: bool, at: &[&str], ids: &[&str]) -> (PathBuf, config::RuleSet) {
    (
        format!(".hannes/{id}.toml").into(),
        config::RuleSet {
            id: id.into(),
            enabled,
            run_at: at.iter().map(|s| s.to_string()).collect(),
            rules: ids.iter().map(|s| s.to_string()).collect(),
        },
    )
}
fn points() -> config::ExecutionPoints {
    config::ExecutionPoints {
        custom: vec!["check".into(), "release".into()],
    }
}
#[test]
fn resolution_obeys_active_membership_override_union_and_disabled_rules() {
    let definitions = [
        assignment("a", true, &["pre-commit"]),
        assignment("b", true, &[]),
        assignment("c", false, &[]),
    ];
    let sets = [
        set("one", true, &["pre-push", "check"], &["a", "b", "c"]),
        set("two", true, &["pre-commit", "check"], &["b"]),
        set("off", false, &[], &["a"]),
    ];
    let resolved = rules::resolve(&points(), &definitions, &sets).unwrap();
    assert_eq!(
        resolved.rules["a"]
            .execution_points
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["check", "pre-push"]
    );
    assert_eq!(
        resolved.rules["b"]
            .execution_points
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["check", "pre-commit", "pre-push"]
    );
    assert!(resolved.rules["c"].execution_points.is_empty());
    assert_eq!(resolved.diagnostics.len(), 1);
    assert!(resolved.diagnostics[0].contains(".hannes/a.toml: rule a"));
    let mut reversed = sets.clone();
    reversed.reverse();
    assert_eq!(
        serde_json::to_value(&resolved.rules).unwrap(),
        serde_json::to_value(
            rules::resolve(&points(), &definitions, &reversed)
                .unwrap()
                .rules
        )
        .unwrap()
    );
    let alone = rules::resolve(
        &points(),
        &[assignment("a", true, &["release"])],
        &[set("off", false, &[], &["a"])],
    )
    .unwrap();
    assert!(alone.rules["a"].execution_points.contains("release"));
    assert!(rules::resolve(&points(), &[assignment("a", false, &[])], &[]).is_ok());
}
#[test]
fn invalid_assignments_are_rejected_even_when_disabled_or_overridden() {
    let a = assignment("a", true, &[]);
    for sets in [
        vec![],
        vec![set("off", false, &[], &["a"])],
        vec![set("empty", true, &[], &["a"])],
        vec![set("bad", true, &["check"], &["missing"])],
        vec![set("nested", true, &["check"], &["nested"])],
        vec![set("dup", true, &["check"], &["a", "a"])],
        vec![set("same", false, &[], &[]), set("same", false, &[], &[])],
        vec![set("bad", false, &["chekc"], &[])],
    ] {
        let error = rules::resolve(&points(), std::slice::from_ref(&a), &sets)
            .unwrap_err()
            .to_string();
        assert!(
            error.contains(".hannes/")
                && (error.contains("run_at") || error.contains("rules") || error.contains("ID")),
            "{error}"
        );
    }
    for rule in [
        assignment("a", false, &["chekc"]),
        assignment("a", true, &["check", "check"]),
    ] {
        assert!(
            rules::resolve(
                &points(),
                &[rule],
                &[set("active", true, &["check"], &["a"])]
            )
            .is_err()
        );
    }
    assert!(rules::resolve(&points(), &[a.clone(), a], &[]).is_err());
    for custom in [
        vec!["check", "check"],
        vec!["pre-commit"],
        vec![""],
        vec!["Invalid"],
        vec!["../bad"],
    ] {
        assert!(
            rules::resolve(
                &config::ExecutionPoints {
                    custom: custom.into_iter().map(String::from).collect()
                },
                &[],
                &[]
            )
            .is_err()
        );
    }
    let resolved = rules::resolve(&config::ExecutionPoints::default(), &[], &[]).unwrap();
    assert_eq!(resolved.points.len(), 3);
}
struct Fixture {
    root: TempDir,
    state: TempDir,
}
impl Fixture {
    fn new() -> Self {
        let root = tempdir().unwrap();
        assert!(
            Command::new("git")
                .args(["init", "--quiet"])
                .current_dir(root.path())
                .status()
                .unwrap()
                .success()
        );
        config::init(root.path()).unwrap();
        let this = Self {
            root,
            state: tempdir().unwrap(),
        };
        this.write("config.toml", "version=3\nproject='My_App'\nmax_display_files=5\n[execution_points]\ncustom=['check', 'release']\n");
        this
    }
    fn write(&self, file: &str, text: &str) {
        let path = self.root.path().join(".hannes").join(file);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }
    fn check(&self, file: &str, id: &str, command: &str, at: &str) {
        self.write(file, &format!("[[checks]]\nid='{id}'\ndescription='Check'\nenabled=true\ncommand={command:?}\ntimeout_seconds=10\nrun_at={at}\n"));
    }
    fn cli(&self, args: &[&str]) -> std::process::Output {
        Command::new(env!("CARGO_BIN_EXE_hannes"))
            .args(args)
            .current_dir(self.root.path())
            .env("HANNES_STATE_DIR", self.state.path())
            .env("NO_COLOR", "1")
            .output()
            .unwrap()
    }
}
#[test]
fn discovery_is_recursive_content_based_and_validates_all_documents() {
    let f = Fixture::new();
    f.check("security/arbitrary.toml", "scan", "git --version", "[]");
    f.write(
        "backend/deep/collection.toml",
        "[[rule_sets]]\nid='security'\nenabled=true\nrules=['scan']\nrun_at=['check']\n",
    );
    f.check(
        "conventions/other.toml",
        "other",
        "git --version",
        "['pre-commit']",
    );
    // Definitions may also live in the central document.
    let central = f.root.path().join(".hannes/config.toml");
    fs::write(
        &central,
        fs::read_to_string(&central).unwrap()
            + "\n[[rule_sets]]\nid='empty'\nenabled=true\nrules=[]\nrun_at=['release']\n",
    )
    .unwrap();
    for ignored in [
        "reports/bad.toml",
        "cache/bad.toml",
        ".git/bad.toml",
        ".management/bad.toml",
    ] {
        f.write(ignored, "invalid TOML");
    }
    let p = config::load(f.root.path()).unwrap();
    assert_eq!(
        p.effective.rules["scan"].source,
        PathBuf::from(".hannes/security/arbitrary.toml")
    );
    assert_eq!(
        planner::build(&p, Some("check"), false)
            .unwrap()
            .plan
            .scheduled
            .iter()
            .map(|r| r.id.as_str())
            .collect::<Vec<_>>(),
        ["scan"]
    );
    f.check("duplicate.toml", "scan", "git --version", "['check']");
    let error = format!("{:#}", config::load(f.root.path()).unwrap_err());
    assert!(error.contains("duplicate") && error.contains(".toml"));
    fs::remove_file(f.root.path().join(".hannes/duplicate.toml")).unwrap();
    f.write("bad.toml", "id='unwrapped'");
    assert!(format!("{:#}", config::load(f.root.path()).unwrap_err()).contains("bad.toml"));
    f.write("bad.toml", "[[checks]]\nthis is not TOML");
    assert!(config::load(f.root.path()).is_err());
}
#[test]
fn execution_selects_once_in_order_and_empty_points_succeed() {
    let f = Fixture::new();
    f.check("z.toml", "z", "git --version", "[]");
    f.check("a.toml", "a", "git --version", "[]");
    f.write("sets.toml", "[[rule_sets]]\nid='one'\nenabled=true\nrules=['z','a']\nrun_at=['check']\n[[rule_sets]]\nid='two'\nenabled=true\nrules=['z']\nrun_at=['check','pre-commit']\n");
    let p = config::load(f.root.path()).unwrap();
    assert!(planner::build(&p, Some("chekc"), false).is_err());
    let run = hannes::verify_project_at(
        p,
        &Never,
        VerificationMode::AllRules,
        Some("check"),
        &mut |_| {},
    )
    .unwrap();
    assert_eq!(run.overall, Status::Pass);
    assert_eq!(
        run.results
            .iter()
            .map(|r| r.id.as_str())
            .collect::<Vec<_>>(),
        ["a", "z"]
    );
    assert_eq!(
        report::read(f.root.path()).unwrap().plan.unwrap().at,
        "check"
    );
    assert!(f.cli(&["mach", "was", "--at", "release"]).status.success());
    assert!(report::read(f.root.path()).unwrap().results.is_empty());
    f.check(
        "a.toml",
        "a",
        "git definitely-not-a-subcommand",
        "['check']",
    );
    assert_eq!(
        f.cli(&["mach", "was", "--at", "check"]).status.code(),
        Some(1)
    );
    assert_eq!(report::read(f.root.path()).unwrap().overall, Status::Fail);
    assert_eq!(
        f.cli(&["mach", "was", "--at", "unknown", "--plan", "--json"])
            .status
            .code(),
        Some(2)
    );
}
#[test]
fn checks_only_cannot_shrink_an_execution_point_and_validation_does_not_repair() {
    let f = Fixture::new();
    let review = f
        .root
        .path()
        .join(".hannes/reviews/product-consistency.toml");
    fs::write(
        &review,
        fs::read_to_string(&review)
            .unwrap()
            .replace("enabled = false", "enabled = true"),
    )
    .unwrap();
    assert!(f.cli(&["pass", "auf"]).status.success());
    assert!(!f.root.path().join(".editorconfig").exists());
    f.check(
        "editorconfig.toml",
        "editorconfig",
        "git ls-files --error-unmatch .editorconfig",
        "['check']",
    );
    assert!(f.cli(&["pass", "auf"]).status.success());
    assert!(!f.root.path().join(".editorconfig").exists());
    assert_eq!(
        f.cli(&["mach", "was", "--at", "check"]).status.code(),
        Some(1)
    );
    assert!(!f.root.path().join(".editorconfig").exists());
    let p = config::load(f.root.path()).unwrap();
    assert!(!planner::build(&p, None, true).unwrap().plan.executable());
    assert!(
        hannes::verify_project_at(p, &Never, VerificationMode::ChecksOnly, None, &mut |_| {})
            .is_err()
    );
    assert!(
        f.cli(&["mach", "was", "--at", "release", "--geiz-ist-geil"])
            .status
            .success()
    );
}
#[cfg(unix)]
#[test]
fn discovery_rejects_symlinks_and_bounds_nesting() {
    use std::os::unix::fs::symlink;
    let f = Fixture::new();
    let outside = tempdir().unwrap();
    symlink(outside.path(), f.root.path().join(".hannes/link")).unwrap();
    assert!(format!("{:#}", config::load(f.root.path()).unwrap_err()).contains("link"));
    fs::remove_file(f.root.path().join(".hannes/link")).unwrap();
    symlink(
        f.root.path().join(".hannes/checks/always-ok.toml"),
        f.root.path().join(".hannes/alias.toml"),
    )
    .unwrap();
    assert!(config::load(f.root.path()).is_err());
    fs::remove_file(f.root.path().join(".hannes/alias.toml")).unwrap();
    let deep = std::iter::repeat_n("nested", hannes::limits::MAX_CONFIG_DEPTH + 1)
        .collect::<Vec<_>>()
        .join("/");
    fs::create_dir_all(f.root.path().join(".hannes").join(deep)).unwrap();
    assert!(format!("{:#}", config::load(f.root.path()).unwrap_err()).contains("nesting"));
}
#[test]
fn prepared_input_is_shared_with_the_provider_without_rereading_context() {
    let f = Fixture::new();
    fs::write(
        f.root.path().join("Policy.md"),
        "# Policy\nOriginal requirement.\n",
    )
    .unwrap();
    f.write("reviews/product-consistency.toml", "[[reviews]]\nrun_at=['pre-push']\nid='product-consistency'\ndescription='Review'\nenabled=true\ninstructions='product-consistency.md'\nfiles=['Policy.md']\n[[reviews.context]]\npath='Policy.md'\n");
    let project = config::load(f.root.path()).unwrap();
    let prepared = planner::build(&project, None, false).unwrap();
    let input = prepared.review_input.as_ref().unwrap();
    assert_eq!(
        prepared.plan.context.selected_bytes,
        input.excerpts.iter().map(|e| e.text.len()).sum::<usize>()
    );
    assert_eq!(
        prepared.plan.context.orientation_diff_bytes,
        input.diff.len()
    );
    let active = [&project.reviews[0]];
    let request = Request {
        project: &project,
        reviews: &active,
        snapshot: &prepared.snapshot,
        input,
    };
    let original = hannes::provider::prompt(&request).unwrap();
    fs::remove_file(f.root.path().join("Policy.md")).unwrap();
    assert_eq!(hannes::provider::prompt(&request).unwrap(), original);
    assert!(original.contains("Original requirement."));
    // Reusing preparation does not weaken the separate source-stability requirement.
    let current = hannes::git_snapshot::capture(f.root.path()).unwrap();
    assert_eq!(
        hannes::git_snapshot::compare(&prepared.snapshot, &current, 5).state,
        "STALE"
    );
    let checks_only = planner::build(&project, None, true).unwrap();
    assert!(checks_only.review_input.is_none());
}

#[test]
fn saved_reports_reject_missing_coverage_and_plan_metadata() {
    let f = Fixture::new();
    hannes::verify(f.root.path(), &Never).unwrap();
    let path = f.root.path().join(".hannes/reports/last.json");
    let original: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    for field in ["results", "required_ids", "plan_id", "at", "plan"] {
        let mut broken = original.clone();
        match field {
            "results" => broken["results"] = serde_json::json!([]),
            "required_ids" => broken["plan"]["required_ids"] = serde_json::json!([]),
            "plan_id" => broken["plan"]["plan_id"] = serde_json::json!("bad"),
            "at" => broken["plan"]["at"] = serde_json::json!(""),
            "plan" => broken
                .as_object_mut()
                .unwrap()
                .remove("plan")
                .map(|_| ())
                .unwrap(),
            _ => unreachable!(),
        }
        fs::write(&path, serde_json::to_string(&broken).unwrap()).unwrap();
        assert!(
            report::read(f.root.path()).is_err(),
            "accepted inconsistent {field}"
        );
    }
}

#[test]
fn selected_reviews_keep_the_existing_shared_provider_execution() {
    struct Mock(std::cell::Cell<usize>);
    impl Provider for Mock {
        fn review(&self, request: &Request<'_>) -> anyhow::Result<String> {
            self.0.set(self.0.get() + 1);
            assert_eq!(request.reviews.len(), 1);
            Ok(serde_json::json!({"reviews":[{"id":"product-consistency","status":"PASS","reason":"Inspected explicit expectations","findings":[{"message":"Reviewed documented expectations","path":".hannes/README.md","line":1}]}]}).to_string())
        }
    }
    let f = Fixture::new();
    let path = f
        .root
        .path()
        .join(".hannes/reviews/product-consistency.toml");
    fs::write(
        &path,
        fs::read_to_string(&path)
            .unwrap()
            .replace("enabled = false", "enabled = true"),
    )
    .unwrap();
    f.write("collections/reviews.toml", "[[rule_sets]]\nid='one'\nenabled=true\nrules=['product-consistency']\nrun_at=['check']\n[[rule_sets]]\nid='two'\nenabled=true\nrules=['product-consistency']\nrun_at=['check']\n");
    let mock = Mock(std::cell::Cell::new(0));
    let run = hannes::verify_project_at(
        config::load(f.root.path()).unwrap(),
        &mock,
        VerificationMode::AllRules,
        Some("check"),
        &mut |_| {},
    )
    .unwrap();
    assert_eq!(mock.0.get(), 1);
    assert_eq!(run.overall, Status::Pass);
    assert_eq!(run.results.len(), 1);
}

#[test]
fn mixed_documents_support_nested_reviews_and_report_entity_schema_errors() {
    let f = Fixture::new();
    f.write(
        "nested/criteria.md",
        "Evaluate explicitly documented expectations.\n",
    );
    f.write("nested/mixed.toml", "[[reviews]]\nid='nested-review'\ndescription='Nested criteria'\nenabled=false\ninstructions='criteria.md'\n[[checks]]\nid='nested-check'\ndescription='Nested check'\nenabled=true\ncommand='git --version'\ntimeout_seconds=10\n[[rule_sets]]\nid='mixed'\nenabled=true\nrules=['nested-check','nested-review']\nrun_at=['check']\n");
    let p = config::load(f.root.path()).unwrap();
    assert_eq!(
        p.reviews
            .iter()
            .find(|r| r.id == "nested-review")
            .unwrap()
            .text,
        "Evaluate explicitly documented expectations.\n"
    );
    assert_eq!(
        planner::build(&p, Some("check"), false)
            .unwrap()
            .plan
            .scheduled
            .len(),
        1
    );
    let path = f.root.path().join(".hannes/nested/mixed.toml");
    fs::write(
        &path,
        fs::read_to_string(&path)
            .unwrap()
            .replace("timeout_seconds=10\n", ""),
    )
    .unwrap();
    let error = format!("{:#}", config::load(f.root.path()).unwrap_err());
    assert!(
        error.contains("nested/mixed.toml")
            && error.contains("nested-check")
            && error.contains("timeout_seconds"),
        "{error}"
    );
}
