use anyhow::Result;
use hannes::{
    VerificationMode, config, planner,
    provider::{Provider, Request},
    report::{self, Status},
};
use std::{
    cell::Cell,
    fs,
    process::{Command, Output},
};
use tempfile::{TempDir, tempdir};

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
        this.write(
            "config.toml",
            "version = 2\nproject = 'Fixture'\nmax_display_files = 5\n",
        );
        this
    }
    fn write(&self, path: &str, text: &str) {
        let full = self.root.path().join(".hannes").join(path);
        fs::create_dir_all(full.parent().unwrap()).unwrap();
        fs::write(full, text).unwrap();
    }
    fn check(&self, id: &str, tags: &[&str]) {
        self.write(&format!("checks/{id}.toml"), &format!("id = '{id}'\ndescription = 'Check {id}'\nenabled = true\ncommand = ['git', '--version']\ntimeout_seconds = 30\ntags = {}\n", serde_json::to_string(tags).unwrap()));
    }
    fn profile(&self, id: &str, selectors: &str) {
        self.write(
            &format!("profiles/{id}.toml"),
            &format!("id = '{id}'\n{selectors}\n"),
        );
    }
    fn enable_review(&self) {
        let file = self
            .root
            .path()
            .join(".hannes/reviews/product-consistency.toml");
        fs::write(
            &file,
            fs::read_to_string(&file)
                .unwrap()
                .replace("enabled = false", "enabled = true"),
        )
        .unwrap();
    }
    fn plan(&self, profile: Option<&str>, checks_only: bool) -> planner::Plan {
        planner::build(
            &config::load(self.root.path()).unwrap(),
            profile,
            checks_only,
        )
        .unwrap()
        .plan
    }
    fn cli(&self, args: &[&str]) -> Output {
        // No test may accidentally use a real authenticated provider.
        let mut command = Command::new(env!("CARGO_BIN_EXE_hannes"));
        command
            .args(args)
            .current_dir(self.root.path())
            .env("HANNES_STATE_DIR", self.state.path())
            .env("NO_COLOR", "1");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let bin = self.state.path().join("bin");
            fs::create_dir_all(&bin).unwrap();
            let staged = self.state.path().join("script");
            fs::write(
                &staged,
                "#!/bin/sh\nprintf called > \"$HANNES_PROVIDER_MARKER\"\nexit 79\n",
            )
            .unwrap();
            let copied = Command::new("cp")
                .arg(&staged)
                .arg(bin.join("codex"))
                .status()
                .unwrap();
            assert!(copied.success());
            fs::set_permissions(bin.join("codex"), fs::Permissions::from_mode(0o700)).unwrap();
            let path = std::env::join_paths(
                std::iter::once(bin)
                    .chain(std::env::split_paths(&std::env::var_os("PATH").unwrap())),
            )
            .unwrap();
            command.env("PATH", path).env(
                "HANNES_PROVIDER_MARKER",
                self.state.path().join("provider-called"),
            );
        }
        command.output().unwrap()
    }
    fn run(
        &self,
        profile: &str,
        checks_only: bool,
        provider: &dyn Provider,
    ) -> Result<report::Report> {
        hannes::verify_project_with_profile(
            config::load(self.root.path())?,
            provider,
            if checks_only {
                VerificationMode::ChecksOnly
            } else {
                VerificationMode::AllRules
            },
            Some(profile),
            &mut |_| {},
        )
    }
}
struct Never;
impl Provider for Never {
    fn review(&self, _: &Request<'_>) -> Result<String> {
        panic!("provider must not be called")
    }
}
struct ReviewPass {
    calls: Cell<usize>,
}

#[test]
fn cli_execution_honors_defaults_and_overrides_without_starting_unselected_reviews() {
    let f = Fixture::new();
    f.check("local-check", &["local"]);
    f.enable_review();
    f.profile("local", "include_tags=['local']");
    f.profile("other", "rules=['always-ok']");
    f.write(
        "config.toml",
        "version=2\nproject='Fixture'\nmax_display_files=5\ndefault_profile='other'\n",
    );
    for (args, profile, id) in [
        (vec!["mach", "was"], "other", "always-ok"),
        (
            vec!["mach", "was", "--profil", "local"],
            "local",
            "local-check",
        ),
        (
            vec!["mach", "was", "--profil", "local", "--geiz-ist-geil"],
            "local",
            "local-check",
        ),
    ] {
        let output = f.cli(&args);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let report = report::read(f.root.path()).unwrap();
        assert_eq!(report.results.len(), 1);
        assert_eq!(report.results[0].id, id);
        assert_eq!(report.plan.unwrap().profile, profile);
        assert!(!f.state.path().join("provider-called").exists());
    }
    use clap::Parser;
    assert!(hannes::cli::Cli::try_parse_from(["hannes", "mach", "was", "--json"]).is_err());
    assert!(
        hannes::cli::Cli::try_parse_from(["hannes", "pass", "auf", "--profil", "local"]).is_err()
    );
}
impl Provider for ReviewPass {
    fn review(&self, request: &Request<'_>) -> Result<String> {
        self.calls.set(self.calls.get() + 1);
        assert_eq!(request.reviews.len(), 1);
        Ok(serde_json::json!({"reviews":[{"id":request.reviews[0].id,"status":"PASS","reason":"Documented criteria inspected.","findings":[{"message":"Criteria checked","path":".hannes/README.md","line":1}]}]}).to_string())
    }
}

#[test]
fn id_and_tag_union_is_deduplicated_and_ordered_checks_before_reviews() {
    let f = Fixture::new();
    f.check("z-last", &["local"]);
    f.check("a-first", &["local", "critical"]);
    f.enable_review();
    f.profile(
        "local",
        "rules = ['z-last', 'product-consistency']\ninclude_tags = ['local']",
    );
    let plan = f.plan(Some("local"), false);
    assert_eq!(
        plan.scheduled
            .iter()
            .map(|r| r.id.as_str())
            .collect::<Vec<_>>(),
        ["a-first", "z-last", "product-consistency"]
    );
    assert_eq!(plan.scheduled[1].selected_by, ["id:z-last", "tag:local"]);
    assert_eq!(plan.planned_model_calls, 1);
    f.check("b-new", &["local"]);
    assert!(f.plan(Some("local"), false).schedules("b-new"));
}

#[test]
fn defaults_explicit_overrides_and_v1_activation_are_strict() {
    let f = Fixture::new();
    f.check("local-test", &["local"]);
    f.profile("local", "include_tags = ['local']");
    f.write(
        "config.toml",
        "version = 2\nproject = 'Fixture'\nmax_display_files = 5\ndefault_profile = 'local'\n",
    );
    assert_eq!(f.plan(None, false).profile_source, "default");
    assert_eq!(f.plan(Some("all"), false).profile_source, "explicit");
    assert!(f.plan(Some("all"), false).schedules("always-ok"));
    f.write(
        "config.toml",
        "version = 1\nproject = 'Fixture'\nmax_display_files = 5\n",
    );
    assert!(config::load(f.root.path()).is_err()); // new tags/profile files cannot silently work in v1
    fs::remove_file(f.root.path().join(".hannes/checks/local-test.toml")).unwrap();
    fs::remove_dir_all(f.root.path().join(".hannes/profiles")).unwrap();
    f.write("checks/always-ok.toml", "id='always-ok'\ndescription='Test'\nenabled=true\ncommand=['git','--version']\ntimeout_seconds=30\ntags=[]\n");
    assert!(config::load(f.root.path()).is_err());
}

#[test]
fn invalid_unused_profiles_and_tags_are_rejected_before_execution() {
    for invalid in [
        "rules=['missing']",
        "include_tags=['missing']",
        "rules=['always-ok','always-ok']",
        "include_tags=['local','local']",
        "input='index'",
        "input='commit'",
        "other=true",
        "gate=true",
        "gate=true\nrules=['deliberately-failing']",
    ] {
        let f = Fixture::new();
        f.check("local", &["local"]);
        f.profile("unused", invalid);
        assert!(config::load(f.root.path()).is_err(), "{invalid}");
    }
    for invalid in [
        "UPPER",
        "-first",
        "",
        "contains space",
        "abcdefghijklmnopqrstuvwxyz1234567",
    ] {
        let f = Fixture::new();
        f.check("test", &[invalid]);
        assert!(config::load(f.root.path()).is_err(), "tag {invalid:?}");
    }
    let f = Fixture::new();
    f.profile("all", "rules=['always-ok']");
    assert!(config::load(f.root.path()).is_err());
    let f = Fixture::new();
    f.profile("one", "rules=['always-ok']");
    f.write("profiles/two.toml", "id='one'\n");
    assert!(config::load(f.root.path()).is_err());
}

#[test]
fn unknown_profile_and_invalid_configuration_have_structured_json_errors() {
    let f = Fixture::new();
    let output = f.cli(&["mach", "was", "--profil", "missing", "--plan", "--json"]);
    assert_eq!(output.status.code(), Some(2));
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["status"], "blocked");
    assert!(value["errors"][0].as_str().unwrap().contains("missing"));
    f.write("config.toml", "invalid");
    let output = f.cli(&["mach", "was", "--plan", "--json"]);
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap()["status"],
        "blocked"
    );
}

#[test]
fn preview_is_offline_and_preserves_report_and_git_state() {
    let f = Fixture::new();
    f.enable_review();
    f.write("checks/always-ok.toml", "id='always-ok'\ndescription='Marker'\nenabled=true\ncommand=['git','config','--local','hannes.executed','yes']\ntimeout_seconds=30\n");
    f.write("reports/last.json", "unchanged report bytes");
    let git_config = fs::read(f.root.path().join(".git/config")).unwrap();
    let output = f.cli(&["mach", "was", "--plan", "--json"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["planned_model_calls"], 1);
    assert_eq!(value["provider_availability_checked"], false);
    assert_eq!(
        fs::read_to_string(f.root.path().join(".hannes/reports/last.json")).unwrap(),
        "unchanged report bytes"
    );
    assert_eq!(
        fs::read(f.root.path().join(".git/config")).unwrap(),
        git_config
    );
    assert!(!f.root.path().join(".git/index").exists());
    assert!(!f.state.path().join("provider-called").exists());
    let text = f.cli(&["mach", "was", "--plan"]);
    assert!(text.status.success());
    assert!(String::from_utf8_lossy(&text.stdout).contains(value["plan_id"].as_str().unwrap()));
}

#[test]
fn checks_only_explains_omissions_and_cannot_shrink_a_gate() {
    let f = Fixture::new();
    f.enable_review();
    f.profile(
        "local",
        "rules=['always-ok','product-consistency','deliberately-failing']",
    );
    let plan = f.plan(Some("local"), true);
    assert_eq!(plan.planned_model_calls, 0);
    assert!(!plan.selection_complete);
    assert_eq!(
        plan.omitted
            .iter()
            .filter(|r| r.reason == "checks_only")
            .count(),
        1
    );
    assert_eq!(
        plan.omitted
            .iter()
            .filter(|r| r.reason == "disabled")
            .count(),
        1
    );
    let report = f.run("local", true, &Never).unwrap();
    assert_eq!(report.overall, Status::Pass);
    assert!(report.results.iter().all(|r| r.kind == "check"));
    f.profile(
        "gate",
        "gate=true\nrules=['always-ok','product-consistency']",
    );
    let saved = fs::read(f.root.path().join(".hannes/reports/last.json")).unwrap();
    let blocked = f.plan(Some("gate"), true);
    assert!(!blocked.executable());
    let output = f.cli(&[
        "mach",
        "was",
        "--profil",
        "gate",
        "--plan",
        "--geiz-ist-geil",
        "--json",
    ]);
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap()["status"],
        "blocked"
    );
    assert!(f.run("gate", true, &Never).is_err());
    assert_eq!(
        fs::read(f.root.path().join(".hannes/reports/last.json")).unwrap(),
        saved
    );
    assert!(!f.state.path().join("provider-called").exists());
}

#[test]
fn gates_execute_selected_rules_and_record_not_run_reviews_after_failed_checks() {
    let f = Fixture::new();
    f.enable_review();
    f.profile(
        "gate",
        "gate=true\nrules=['always-ok','product-consistency']",
    );
    let provider = ReviewPass {
        calls: Cell::new(0),
    };
    let report = f.run("gate", false, &provider).unwrap();
    assert_eq!(report.overall, Status::Pass);
    assert_eq!(provider.calls.get(), 1);
    assert!(report.plan.as_ref().unwrap().gate);
    report::read(f.root.path()).unwrap();
    f.write("checks/always-ok.toml", "id='always-ok'\ndescription='Failure'\nenabled=true\ncommand=['git','definitely-not-a-command']\ntimeout_seconds=30\n");
    let report = f.run("gate", false, &Never).unwrap();
    assert_eq!(report.overall, Status::Fail);
    assert_eq!(report.results[1].status, Status::NotRun);
    assert!(report.plan.unwrap().selection_complete);
}

#[test]
fn plan_ids_ignore_selector_and_metadata_order_but_track_semantic_inputs() {
    let f = Fixture::new();
    f.check("alpha", &["local", "critical"]);
    f.check("beta", &["local"]);
    f.profile(
        "local",
        "rules=['beta','alpha']\ninclude_tags=['local','critical']",
    );
    let original = f.plan(Some("local"), false);
    f.profile(
        "local",
        "include_tags=['critical','local']\nrules=['alpha','beta']",
    );
    fs::rename(
        f.root.path().join(".hannes/checks/alpha.toml"),
        f.root.path().join(".hannes/checks/renamed.toml"),
    )
    .unwrap();
    assert_eq!(original.plan_id, f.plan(Some("local"), false).plan_id);
    assert_ne!(
        original.input.fingerprint,
        f.plan(Some("local"), false).input.fingerprint
    );
    let provider = f.root.path().join(".hannes/provider.toml");
    fs::write(
        &provider,
        fs::read_to_string(&provider)
            .unwrap()
            .replace("gpt-6.1-sol", "another-model"),
    )
    .unwrap();
    let changed = f.plan(Some("local"), false);
    assert_ne!(original.plan_id, changed.plan_id);
    fs::write(f.root.path().join("source.txt"), "new source\n").unwrap();
    assert_ne!(changed.plan_id, f.plan(Some("local"), false).plan_id);
}

#[test]
fn context_is_deduplicated_bounded_and_part_of_identity() {
    let f = Fixture::new();
    f.enable_review();
    fs::write(
        f.root.path().join("Policy.md"),
        "# Policy\n## Keyboard\nRequirements\n## Other\nUnrelated\n",
    )
    .unwrap();
    f.write("reviews/product-consistency.toml", "id='product-consistency'\ndescription='Review'\nenabled=true\ninstructions='product-consistency.md'\nfiles=['Policy.md']\n[[context]]\npath='Policy.md'\nheadings=['Keyboard']\n[[context]]\npath='Policy.md'\nheadings=['Keyboard']\n");
    let plan = f.plan(None, false);
    assert_eq!(plan.context.sections.len(), 1);
    assert_eq!(plan.context.selections.len(), 1);
    assert_eq!(plan.context.sections[0].start_line, 2);
    assert!(!serde_json::to_string(&plan).unwrap().contains("Unrelated"));
    f.write(
        "reviews/product-consistency.md",
        "Changed review criteria.\n",
    );
    assert_ne!(plan.plan_id, f.plan(None, false).plan_id);
}

#[test]
fn empty_profiles_are_explicit_and_incomplete_snapshots_never_start_checks() {
    let f = Fixture::new();
    f.profile("empty", "");
    let report = f.run("empty", false, &Never).unwrap();
    assert!(report.results.is_empty());
    assert!(hannes::terminal::render(&report, false).contains("kein Regel-PASS"));
    f.profile("checks", "gate=true\nrules=['always-ok']");
    assert_eq!(f.run("checks", true, &Never).unwrap().overall, Status::Pass);
    let file = fs::File::create(f.root.path().join("large.bin")).unwrap();
    file.set_len(8 * 1024 * 1024 + 1).unwrap();
    let plan = f.plan(Some("checks"), false);
    assert!(!plan.executable());
    let report = f.run("checks", false, &Never).unwrap();
    assert_eq!(report.overall, Status::Error);
    assert!(
        report
            .results
            .iter()
            .filter(|r| r.kind == "check")
            .all(|r| r.status == Status::NotRun)
    );
}

#[test]
fn status_keeps_historical_scope_and_legacy_reports_cannot_claim_a_gate() {
    let f = Fixture::new();
    f.profile("local", "rules=['always-ok']");
    let report = f.run("local", false, &Never).unwrap();
    f.profile("different", "");
    f.write(
        "config.toml",
        "version=2\nproject='Fixture'\nmax_display_files=5\ndefault_profile='different'\n",
    );
    let output = f.cli(&["was", "ist", "phase"]);
    assert!(String::from_utf8_lossy(&output.stdout).contains("Profil: local"));
    let mut legacy = serde_json::to_value(report).unwrap();
    legacy["schema_version"] = 1.into();
    legacy.as_object_mut().unwrap().remove("plan");
    f.write("reports/last.json", &legacy.to_string());
    assert!(report::read(f.root.path()).unwrap().plan.is_none());
    assert!(
        String::from_utf8_lossy(&f.cli(&["was", "ist", "phase"]).stdout).contains("Legacy-Report")
    );
}

#[test]
fn saved_reports_reject_missing_gate_coverage_and_plan_metadata() {
    let f = Fixture::new();
    f.profile("gate", "gate=true\nrules=['always-ok']");
    let original = serde_json::to_value(f.run("gate", false, &Never).unwrap()).unwrap();
    for mutation in [
        "missing-plan",
        "missing-required",
        "duplicate-scheduled",
        "wrong-input",
        "blocked-pass",
    ] {
        let mut value = original.clone();
        match mutation {
            "missing-plan" => {
                value.as_object_mut().unwrap().remove("plan");
            }
            "missing-required" => value["plan"]["required_ids"] = serde_json::json!([]),
            "duplicate-scheduled" => {
                let rule = value["plan"]["scheduled"][0].clone();
                value["plan"]["scheduled"]
                    .as_array_mut()
                    .unwrap()
                    .push(rule);
            }
            "wrong-input" => value["plan"]["input"]["kind"] = "commit".into(),
            _ => {
                value["plan"]["status"] = "blocked".into();
                value["plan"]["blocking_reasons"] = serde_json::json!(["missing evidence"]);
            }
        }
        f.write("reports/last.json", &value.to_string());
        assert!(report::read(f.root.path()).is_err(), "{mutation}");
    }
}
