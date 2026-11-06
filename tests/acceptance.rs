mod support;
use anyhow::{Result, bail};
use clap::{CommandFactory, Parser};
#[cfg(unix)]
use hannes::process;
use hannes::{
    cli::Cli,
    config, git_snapshot,
    provider::{Provider, Request},
    report::{self, Status},
    terminal,
};
#[cfg(unix)]
use std::time::{Duration, Instant};
use std::{
    cell::Cell,
    fs,
    path::{Path, PathBuf},
    process::Command,
};
use tempfile::{TempDir, tempdir};
fn git(root: &Path, args: &[&str]) {
    let out = Command::new("git")
        .current_dir(root)
        .args(args)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}
fn project() -> TempDir {
    let dir = tempdir().unwrap();
    git(dir.path(), &["init", "--quiet"]);
    config::init(dir.path()).unwrap();
    dir
}
fn path(root: &Path, relative: &str) -> PathBuf {
    root.join(".hannes").join(relative)
}
fn replace(root: &Path, file: &str, from: &str, to: &str) {
    let file = path(root, file);
    let text = fs::read_to_string(&file).unwrap();
    assert!(text.contains(from), "fixture replacement missing");
    fs::write(file, text.replace(from, to)).unwrap();
}
fn enable(root: &Path) {
    replace(
        root,
        "reviews/product-consistency.toml",
        "enabled = false",
        "enabled = true",
    );
}
struct Never;
impl Provider for Never {
    fn review(&self, _: &Request<'_>) -> Result<String> {
        panic!("provider must not be called")
    }
}
struct Mock {
    calls: Cell<u32>,
    payload: String,
    error: bool,
}
impl Provider for Mock {
    fn review(&self, request: &Request<'_>) -> Result<String> {
        self.calls.set(self.calls.get() + 1);
        assert!(!request.reviews.is_empty());
        if self.error {
            bail!("mock process failure / timeout");
        }
        Ok(self.payload.clone())
    }
}
fn verdict(id: &str, status: &str) -> serde_json::Value {
    serde_json::json!({"id":id,"status":status,"reason":"Compared explicit expectations with evidence", "findings":[{"message":"Documented expectations inspected", "path":".hannes/README.md", "line":1}]})
}
fn mock(values: Vec<serde_json::Value>) -> Mock {
    Mock {
        calls: Cell::new(0),
        payload: serde_json::json!({"reviews":values}).to_string(),
        error: false,
    }
}
fn cli(root: &Path, args: &[&str]) -> std::process::Output {
    let state = tempdir().unwrap();
    Command::new(env!("CARGO_BIN_EXE_hannes"))
        .args(args)
        .current_dir(root)
        .env("NO_COLOR", "1")
        .env("HANNES_STATE_DIR", state.path())
        .output()
        .unwrap()
}
#[test]
fn all_command_paths_and_standard_help() {
    for words in [
        "fang an",
        "pass auf",
        "mach was",
        "was ist phase",
        "so hilf mir doch",
    ] {
        let argv: Vec<_> = std::iter::once("hannes")
            .chain(words.split_whitespace())
            .collect();
        assert!(Cli::try_parse_from(argv).is_ok(), "{words}");
    }
    for args in [["hannes", "--help"], ["hannes", "--version"]] {
        assert_eq!(Cli::try_parse_from(args).unwrap_err().exit_code(), 0);
    }
    for words in [
        "fang",
        "fang an extra",
        "was phase",
        "so hilf doch",
        "mach falsch",
    ] {
        let argv: Vec<_> = std::iter::once("hannes")
            .chain(words.split_whitespace())
            .collect();
        assert!(Cli::try_parse_from(argv).is_err());
    }
}
#[test]
fn init_exact_paths_idempotent_preserves_edits() {
    let d = project();
    let root = d.path();
    for file in [
        "config.toml",
        "provider.toml",
        "README.md",
        "checks/always-ok.toml",
        "checks/deliberately-failing.toml",
        "reviews/product-consistency.toml",
        "reviews/product-consistency.md",
        "reports/.gitignore",
    ] {
        assert!(path(root, file).is_file(), "{file}");
    }
    // Check the actual directory entry, even on case-insensitive filesystems.
    assert!(
        fs::read_dir(root)
            .unwrap()
            .any(|entry| { entry.unwrap().file_name() == ".hannes" })
    );
    assert!(!path(root, "reports/last.json").exists());
    assert_eq!(config::init(root).unwrap(), 0);
    fs::write(path(root, "config.toml"), "personal edits").unwrap();
    assert_eq!(config::init(root).unwrap(), 0);
    assert_eq!(
        fs::read_to_string(path(root, "config.toml")).unwrap(),
        "personal edits"
    );
}
#[test]
fn validation_rejects_invalid_including_disabled() {
    let cases = [
        (
            "checks/always-ok.toml",
            "enabled = true",
            "disabled = false",
        ),
        ("config.toml", "version = 1", "version = 99"),
        ("config.toml", "version = 1", "version = 1\nunknown = 1"),
        ("config.toml", "version = 1", "version = 1\nversion = 1"),
        (
            "config.toml",
            "max_display_files = 5",
            "max_display_files = 6",
        ),
        ("provider.toml", "codex-cli", "unknown-provider"),
        ("provider.toml", "180", "0"),
        ("provider.toml", "180", "3601"),
        (
            "checks/deliberately-failing.toml",
            "enabled = false",
            "enabled = false\ndisabled = true",
        ),
        (
            "checks/deliberately-failing.toml",
            "timeout_seconds = 30",
            "timeout_seconds = 0",
        ),
        (
            "checks/deliberately-failing.toml",
            "command = \"git definitely-not-a-subcommand\"",
            "command = []",
        ),
        (
            "checks/deliberately-failing.toml",
            "command = \"git definitely-not-a-subcommand\"",
            "command = [1]",
        ),
        (
            "checks/deliberately-failing.toml",
            "command = \"git definitely-not-a-subcommand\"",
            "command = \"../outside\"",
        ),
        (
            "checks/deliberately-failing.toml",
            "timeout_seconds = 30",
            "timeout_seconds = 30\nworking_directory = \"../outside\"",
        ),
        (
            "checks/deliberately-failing.toml",
            "deliberately-failing",
            "always-ok",
        ),
        (
            "checks/deliberately-failing.toml",
            "deliberately-failing",
            "product-consistency",
        ),
        (
            "checks/deliberately-failing.toml",
            "deliberately-failing",
            "hannes-snapshot",
        ),
        (
            "reviews/product-consistency.toml",
            "product-consistency.md",
            "missing.md",
        ),
        (
            "reviews/product-consistency.toml",
            "product-consistency.md",
            "../README.md",
        ),
    ];
    for (file, from, to) in cases {
        let d = project();
        replace(d.path(), file, from, to);
        assert!(
            config::load(d.path()).is_err(),
            "accepted invalid fixture {file}: {to}"
        );
        assert_eq!(cli(d.path(), &["pass", "auf"]).status.code(), Some(2));
        assert!(!path(d.path(), "reports/last.json").exists());
    }
}
#[test]
fn validation_does_not_execute_checks_or_provider() {
    let d = project();
    enable(d.path());
    replace(
        d.path(),
        "checks/always-ok.toml",
        "command = \"git --version\"",
        "command = \"nonexistent-command-hannes\"",
    );
    assert!(config::load(d.path()).is_ok());
    assert_eq!(cli(d.path(), &["pass", "auf"]).status.code(), Some(0));
}
#[test]
fn default_run_passes_omits_disabled_and_roundtrips_report() {
    let d = project();
    let r = hannes::verify(d.path(), &Never).unwrap();
    assert_eq!(r.overall, Status::Pass);
    assert_eq!(r.results.len(), 1);
    assert_eq!(r.results[0].id, "always-ok");
    let decoded = report::read(d.path()).unwrap();
    assert_eq!(decoded.snapshot.fingerprint, r.snapshot.fingerprint);
    assert!(!terminal::render(&r, false).contains('\x1b'));
    assert!(!terminal::render(&r, false).contains("deliberately-failing"));
    assert!(terminal::render(&r, true).contains("\x1b[32mPASS"));
    assert_eq!(
        git_snapshot::compare(&r.snapshot, &git_snapshot::capture(d.path()).unwrap(), 5).state,
        "CURRENT"
    );
    assert_eq!(
        cli(d.path(), &["was", "ist", "phase"]).status.code(),
        Some(0)
    );
}
#[test]
fn failing_check_runs_remaining_and_blocks_provider() {
    let d = project();
    enable(d.path());
    replace(
        d.path(),
        "checks/deliberately-failing.toml",
        "enabled = false",
        "enabled = true",
    );
    replace(
        d.path(),
        "checks/deliberately-failing.toml",
        "deliberately-failing",
        "aaa-failure",
    );
    let r = hannes::verify(d.path(), &Never).unwrap();
    assert_eq!(r.overall, Status::Fail);
    assert_eq!(r.overall.exit_code(), 1);
    assert_eq!(r.results[0].status, Status::Fail);
    assert_eq!(r.results[1].status, Status::Pass);
    assert_eq!(r.results[2].status, Status::NotRun);
    assert!(path(d.path(), "reports/last.json").is_file());
    assert_eq!(cli(d.path(), &["mach", "was"]).status.code(), Some(1));
}
#[test]
fn spawn_error_writes_blocking_error_report() {
    let d = project();
    replace(
        d.path(),
        "checks/always-ok.toml",
        "command = \"git --version\"",
        "command = \"nonexistent-command-hannes\"",
    );
    let r = hannes::verify(d.path(), &Never).unwrap();
    assert_eq!(r.overall, Status::Error);
    assert_eq!(r.results[0].status, Status::Error);
    assert_eq!(cli(d.path(), &["mach", "was"]).status.code(), Some(2));
    assert_eq!(report::read(d.path()).unwrap().overall, Status::Error);
}
#[test]
fn all_reviews_one_call_exact_coverage_pass_fail() {
    for status in ["PASS", "FAIL"] {
        let d = project();
        enable(d.path());
        let original =
            fs::read_to_string(path(d.path(), "reviews/product-consistency.toml")).unwrap();
        fs::write(
            path(d.path(), "reviews/second.toml"),
            original.replace("id = \"product-consistency\"", "id = \"second\""),
        )
        .unwrap();
        let mock = mock(vec![
            verdict("second", status),
            verdict("product-consistency", status),
        ]);
        let r = hannes::verify(d.path(), &mock).unwrap();
        assert_eq!(mock.calls.get(), 1);
        assert_eq!(r.results.len(), 3);
        assert_eq!(r.overall.label(), status);
        assert_eq!(r.results[1].id, "product-consistency");
        assert_eq!(r.results[2].id, "second");
    }
}
#[test]
fn malformed_missing_extra_duplicate_ungrounded_provider_outputs_fail_closed() {
    let mut no_path = verdict("product-consistency", "PASS");
    no_path["findings"] = serde_json::json!([]);
    let mut invalid_line = verdict("product-consistency", "PASS");
    invalid_line["findings"][0]["line"] = serde_json::json!(999999);
    let mut invalid_path = verdict("product-consistency", "PASS");
    invalid_path["findings"][0]["path"] = serde_json::json!("../secrets");
    let mut missing_field = verdict("product-consistency", "PASS");
    missing_field["findings"][0]
        .as_object_mut()
        .unwrap()
        .remove("line");
    let responses = vec![
        "not json".into(), "{}".into(), "{\"reviews\":[]}".into(),
        serde_json::json!({"reviews":[verdict("wrong", "PASS")]}).to_string(),
        serde_json::json!({"reviews":[verdict("product-consistency", "PASS"), verdict("product-consistency", "PASS")]}).to_string(),
        serde_json::json!({"reviews":[verdict("product-consistency", "NOT_RUN")]}).to_string(),
        serde_json::json!({"reviews":[verdict("product-consistency", "PASS")],"extra":true}).to_string(),
        serde_json::json!({"reviews":[no_path]}).to_string(),
        serde_json::json!({"reviews":[invalid_line]}).to_string(),
        serde_json::json!({"reviews":[invalid_path]}).to_string(),
        serde_json::json!({"reviews":[missing_field]}).to_string(),
        format!("{}\nextra text", serde_json::json!({"reviews":[verdict("product-consistency", "PASS")]})),
    ];
    for payload in responses {
        let d = project();
        enable(d.path());
        let mock = Mock {
            calls: Cell::new(0),
            payload,
            error: false,
        };
        let r = hannes::verify(d.path(), &mock).unwrap();
        assert_eq!(r.overall, Status::Error, "{}", mock.payload);
        assert_eq!(r.results[1].status, Status::Error);
        assert!(path(d.path(), "reports/last.json").is_file());
    }
}
#[test]
fn provider_error_and_timeout_are_errors() {
    let d = project();
    enable(d.path());
    let mock = Mock {
        calls: Cell::new(0),
        payload: String::new(),
        error: true,
    };
    let r = hannes::verify(d.path(), &mock).unwrap();
    assert_eq!(r.overall, Status::Error);
    assert_eq!(mock.calls.get(), 1);
}
#[test]
fn codex_arguments_are_read_only_noninteractive_and_structured() {
    let command = hannes::provider::command(
        Path::new("/repo"),
        Path::new("/tmp/schema"),
        Path::new("/tmp/output"),
        None,
    );
    let args: Vec<_> = command
        .get_args()
        .map(|s| s.to_string_lossy().into_owned())
        .collect();
    for pair in [
        ["--ask-for-approval", "never"],
        ["--sandbox", "read-only"],
        ["--output-schema", "/tmp/schema"],
        ["--output-last-message", "/tmp/output"],
    ] {
        assert!(args.windows(2).any(|a| a == pair));
    }
    assert_eq!(args.last().unwrap(), "-");
    assert!(args.contains(&"--json".into()));
    assert!(!args.contains(&"--model".into()));
    assert!(!args.iter().any(|a| a.contains("bypass")));
}
#[test]
fn status_tracks_staged_modified_untracked_and_ignores_report_changes() {
    let d = project();
    let root = d.path();
    fs::write(root.join("tracked.txt"), "original\n").unwrap();
    git(root, &["add", "tracked.txt"]);
    let r = hannes::verify(root, &Never).unwrap();
    fs::write(path(root, "reports/extra.json"), "ignored local data").unwrap();
    assert_eq!(
        git_snapshot::compare(&r.snapshot, &git_snapshot::capture(root).unwrap(), 5).state,
        "CURRENT"
    );
    fs::write(root.join("tracked.txt"), "modified same status\n").unwrap();
    for i in 0..7 {
        fs::write(root.join(format!("new-{i}.txt")), "new\n").unwrap();
    }
    git(root, &["add", "new-0.txt"]);
    let changes = git_snapshot::compare(&r.snapshot, &git_snapshot::capture(root).unwrap(), 5);
    assert_eq!(changes.state, "STALE");
    assert_eq!(changes.total_files, Some(8));
    assert_eq!(changes.sample_paths.len(), 5);
    assert!(changes.line_counts.is_none());
    let text = terminal::render_changes(&changes, false);
    assert!(text.contains("changed files since verification: 8"));
    assert!(text.contains("hash-only baseline"));
    assert_eq!(cli(root, &["was", "ist", "phase"]).status.code(), Some(1));
    let staged = git_snapshot::capture(root).unwrap();
    fs::write(root.join("tracked.txt"), "yet another change\n").unwrap();
    assert_eq!(
        git_snapshot::compare(&staged, &git_snapshot::capture(root).unwrap(), 5).total_files,
        Some(1)
    );
    fs::remove_file(root.join("tracked.txt")).unwrap();
    assert_eq!(
        git_snapshot::compare(&staged, &git_snapshot::capture(root).unwrap(), 5).total_files,
        Some(1)
    );
}
#[test]
fn ignored_configuration_is_still_fingerprinted() {
    let d = project();
    fs::write(d.path().join(".gitignore"), ".hannes/\n").unwrap();
    let before = git_snapshot::capture(d.path()).unwrap();
    replace(
        d.path(),
        "config.toml",
        "max_display_files = 5",
        "max_display_files = 3",
    );
    let changes = git_snapshot::compare(&before, &git_snapshot::capture(d.path()).unwrap(), 5);
    assert_eq!(changes.state, "STALE");
    assert_eq!(changes.total_files, Some(1));
}
#[test]
fn no_report_corrupt_report_schema_and_no_color() {
    let d = project();
    assert_eq!(
        cli(d.path(), &["was", "ist", "phase"]).status.code(),
        Some(3)
    );
    fs::write(path(d.path(), "reports/last.json"), "broken").unwrap();
    assert_eq!(
        cli(d.path(), &["was", "ist", "phase"]).status.code(),
        Some(2)
    );
    let r = hannes::verify(d.path(), &Never).unwrap();
    let mut value = serde_json::to_value(r).unwrap();
    value["schema_version"] = serde_json::json!(999);
    fs::write(path(d.path(), "reports/last.json"), value.to_string()).unwrap();
    assert_eq!(
        cli(d.path(), &["was", "ist", "phase"]).status.code(),
        Some(2)
    );
    let out = cli(d.path(), &["--no-color", "mach", "was"]);
    assert!(out.status.success());
    assert!(!out.stdout.contains(&0x1b));
}
#[test]
fn huge_file_gives_unknown_and_blocks_verification() {
    let d = project();
    let f = fs::File::create(d.path().join("huge.bin")).unwrap();
    f.set_len(8 * 1024 * 1024 + 1).unwrap();
    let r = hannes::verify(d.path(), &Never).unwrap();
    assert_eq!(r.overall, Status::Error);
    let changes = git_snapshot::compare(&r.snapshot, &git_snapshot::capture(d.path()).unwrap(), 5);
    assert_eq!(changes.state, "UNKNOWN");
    assert_eq!(changes.total_files, None);
}
#[test]
fn no_root_detection_or_git_mutation() {
    let d = project();
    let head = fs::read(d.path().join(".git/HEAD")).unwrap();
    hannes::verify(d.path(), &Never).unwrap();
    assert_eq!(fs::read(d.path().join(".git/HEAD")).unwrap(), head);
    assert!(!d.path().join(".git/index").exists());
    assert!(!d.path().join(".git/refs/heads/master").exists());
    let nested = d.path().join("nested");
    fs::create_dir(&nested).unwrap();
    assert!(config::init(&nested).is_err());
    assert!(!nested.join(".hannes").exists());
}
#[cfg(unix)]
#[test]
fn timeout_output_capture_and_shell_are_explicit() {
    let start = Instant::now();
    let output = process::run(
        Command::new("sh").args(["-c", "printf before; while :; do :; done"]),
        Duration::from_millis(80),
    )
    .unwrap();
    assert!(output.timed_out);
    assert!(start.elapsed() < Duration::from_secs(3));
    assert_eq!(output.stdout.text, "before");
    let output = process::run(
        Command::new("sh").args(["-c", "yes x | head -c 200000; yes y | head -c 200000 >&2"]),
        Duration::from_secs(10),
    )
    .unwrap();
    assert!(output.status.success());
    assert_eq!(output.stdout.text.len(), process::OUTPUT_LIMIT);
    assert_eq!(output.stdout.total_bytes, 200000);
    assert!(output.stdout.truncated);
    assert_eq!(output.stderr.total_bytes, 200000);
    assert!(output.stderr.truncated);
    let d = project();
    replace(
        d.path(),
        "checks/always-ok.toml",
        "command = \"git --version\"",
        "command = \"sh -c 'while :; do :; done'\"",
    );
    replace(
        d.path(),
        "checks/always-ok.toml",
        "timeout_seconds = 30",
        "timeout_seconds = 1",
    );
    let r = hannes::verify(d.path(), &Never).unwrap();
    assert_eq!(r.results[0].status, Status::Error);
    assert!(r.results[0].reason.contains("timed out"));
}
#[cfg(unix)]
#[test]
fn symlink_escape_and_unsupported_snapshot_are_not_green() {
    use std::os::unix::fs::symlink;
    let d = project();
    let other = tempdir().unwrap();
    fs::write(other.path().join("outside.md"), "secret").unwrap();
    symlink(
        other.path().join("outside.md"),
        path(d.path(), "reviews/outside.md"),
    )
    .unwrap();
    replace(
        d.path(),
        "reviews/product-consistency.toml",
        "product-consistency.md",
        "outside.md",
    );
    assert!(config::load(d.path()).is_err());
    replace(
        d.path(),
        "reviews/product-consistency.toml",
        "outside.md",
        "product-consistency.md",
    );
    let r = hannes::verify(d.path(), &Never).unwrap();
    assert_eq!(r.overall, Status::Error);
    assert!(!r.snapshot.complete);
    let noinit = tempdir().unwrap();
    git(noinit.path(), &["init", "--quiet"]);
    symlink(other.path(), noinit.path().join(".hannes")).unwrap();
    assert!(config::init(noinit.path()).is_err());
    assert!(!other.path().join("config.toml").exists());
}
#[cfg(unix)]
#[test]
fn changing_files_during_checks_blocks_pass() {
    let d = project();
    enable(d.path());
    replace(
        d.path(),
        "checks/always-ok.toml",
        "command = \"git --version\"",
        "command = \"sh -c 'printf changed > created.txt'\"",
    );
    let r = hannes::verify(d.path(), &Never).unwrap();
    assert_eq!(r.overall, Status::Error);
    assert!(r.results.iter().any(|r| r.id == "hannes-tree-stability"));
}
#[test]
fn terminal_strips_control_sequences_from_untrusted_values() {
    assert_eq!(terminal::clean("evil\x1b[31m\nrow"), "evil [31m row");
}

#[cfg(unix)]
#[test]
fn real_provider_transport_with_fake_executable_never_calls_codex() {
    for mode in [
        "pass",
        "malformed",
        "missing",
        "exit",
        "timeout",
        "no-usage",
        "invalid-usage",
    ] {
        let d = project();
        enable(d.path());
        replace(
            d.path(),
            "provider.toml",
            "timeout_seconds = 180",
            "timeout_seconds = 1",
        );
        let fake = tempdir().unwrap();
        let executable = fake.path().join("fake-codex");
        let payload =
            serde_json::json!({"reviews":[verdict("product-consistency", "PASS")]}).to_string();
        let behavior = match mode {
            "pass" | "no-usage" | "invalid-usage" => {
                format!("printf '%s' '{}' > \"$output\"", payload)
            }
            "malformed" => "printf 'invalid JSON' > \"$output\"".into(),
            "missing" => ":".into(),
            "exit" => "printf 'SENSITIVE_TRACE' >&2; exit 17".into(),
            _ => "while :; do :; done".into(),
        };
        let telemetry = match mode {
            "no-usage" | "timeout" => String::new(),
            "invalid-usage" => "printf '%s\\n' '{\"type\":\"turn.completed\",\"usage\":{\"input_tokens\":\"oops\"}}'".into(),
            _ => "printf '%s\\n' '{\"type\":\"turn.completed\",\"usage\":{\"input_tokens\":1000,\"cached_input_tokens\":800,\"output_tokens\":200,\"reasoning_output_tokens\":50,\"cache_write_input_tokens\":100}}'".into(),
        };
        // The completion arrives after more than the retained stdout limit.
        write_executable(&executable, format!("#!/bin/sh\nset -eu\nif [ \"${{1:-}}\" = mcp ]; then printf '[]\\n'; exit 0; fi\noutput=''\nwhile [ $# -gt 0 ]; do\n case \"$1\" in --output-last-message) shift; output=$1;; esac\n shift\ndone\ncat > /dev/null\nprintf 'unstructured stdout is not a verdict\\n'\nprintf '%070000d\\n' 0\n{telemetry}\n{behavior}\n")).unwrap();
        let provider = hannes::provider::Codex::new(executable);
        let r = hannes::verify(d.path(), &provider).unwrap();
        assert_eq!(
            r.overall,
            if matches!(mode, "pass" | "no-usage" | "invalid-usage") {
                Status::Pass
            } else {
                Status::Error
            },
            "{mode}"
        );
        let serialized = fs::read_to_string(path(d.path(), "reports/last.json")).unwrap();
        assert!(!serialized.contains("SENSITIVE_TRACE"));
        assert!(!serialized.contains("unstructured stdout"));
        let expected_usage = !matches!(mode, "no-usage" | "invalid-usage" | "timeout");
        assert_eq!(r.token_usage, expected_usage.then_some(1200), "{mode}");
        assert_eq!(r.provider_usage.is_some(), expected_usage, "{mode}");
        let rendered = terminal::render(&r, false);
        if expected_usage {
            assert!(rendered.contains("1200 gesamt · 1000 Eingabe (800 Cache) · 200 Ausgabe"));
            assert!(
                terminal::render_details(&r, false)
                    .contains("Reasoning-Tokens (in Ausgabe enthalten): 50")
            );
        } else {
            assert!(rendered.contains("Codex-Tokens: nicht verfügbar"));
        }
        let saved = report::read(d.path()).unwrap();
        assert_eq!(saved.provider_usage, r.provider_usage);
        assert_eq!(saved.token_usage, r.token_usage);
    }
}
#[test]
fn staging_alone_does_not_stale_content_and_head_changes_do() {
    let d = project();
    fs::write(d.path().join("file.txt"), "content\n").unwrap();
    let snapshot = git_snapshot::capture(d.path()).unwrap();
    git(d.path(), &["add", "file.txt"]);
    let after = git_snapshot::capture(d.path()).unwrap();
    assert_eq!(git_snapshot::compare(&snapshot, &after, 5).state, "CURRENT");
    let mut changed_head = after.clone();
    changed_head.head = Some("different-head".into());
    let changes = git_snapshot::compare(&snapshot, &changed_head, 5);
    assert_eq!(changes.state, "STALE");
    assert_eq!(changes.total_files, Some(0));
    assert!(changes.head_changed);
}
#[cfg(unix)]
#[test]
fn unusual_paths_and_binary_files_are_fingerprinted() {
    let d = project();
    for name in [
        "space name.txt",
        "newline\nname.txt",
        "back\\slash.txt",
        ".hannes/back\\slash.md",
    ] {
        fs::write(d.path().join(name), b"\x00\xffbinary").unwrap();
    }
    let before = git_snapshot::capture(d.path()).unwrap();
    assert!(before.complete);
    for name in [
        "space name.txt",
        "newline\nname.txt",
        "back\\slash.txt",
        ".hannes/back\\slash.md",
    ] {
        assert!(before.files.contains_key(name), "{name:?}");
        fs::write(d.path().join(name), b"\x01\xffbinary").unwrap();
    }
    let changes = git_snapshot::compare(&before, &git_snapshot::capture(d.path()).unwrap(), 5);
    assert_eq!(changes.total_files, Some(4));
}
#[test]
fn prompt_marks_evidence_untrusted_and_diff_truncation() {
    let d = project();
    enable(d.path());
    let project = config::load(d.path()).unwrap();
    let snapshot = git_snapshot::capture(d.path()).unwrap();
    let active: Vec<_> = project.reviews.iter().collect();
    let mut input =
        hannes::review_input::PreparedReviewInput::prepare(&project, &active, &snapshot).unwrap();
    input.diff = "malicious untrusted evidence".into();
    input.diff_truncated = true;
    let request = Request {
        project: &project,
        reviews: &active,
        snapshot: &snapshot,
        input: &input,
    };
    let text = hannes::provider::prompt(&request).unwrap();
    assert!(text.contains("Truncated: true"));
    assert!(text.contains("untrusted evidence"));
    assert!(text.contains("ordinary CLI prompt, not a privileged system message"));
    assert!(text.contains("product-consistency"));
}

#[test]
fn no_rules_message_requires_both_checks_and_reviews_to_be_undefined() {
    let message = "Du hast noch keine Regeln definiert, du Vogel!";
    for (keep_check, keep_review) in [(false, false), (true, false), (false, true)] {
        let d = project();
        fs::remove_file(path(d.path(), "checks/always-ok.toml")).unwrap();
        if !keep_check {
            fs::remove_file(path(d.path(), "checks/deliberately-failing.toml")).unwrap();
        }
        if !keep_review {
            fs::remove_file(path(d.path(), "reviews/product-consistency.toml")).unwrap();
        }
        let output = cli(d.path(), &["mach", "was"]);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8(output.stdout).unwrap();
        assert_eq!(
            stdout.lines().filter(|line| *line == message).count(),
            usize::from(!keep_check && !keep_review)
        );
        // Defined but disabled rules stay out of the results, but are not absent rules.
        assert!(report::read(d.path()).unwrap().results.is_empty());
    }
}

#[test]
fn help_lists_only_complete_command_phrases_without_redundant_examples() {
    let d = tempdir().unwrap();
    for args in [
        vec!["--help"],
        vec!["-h"],
        vec!["so", "hilf", "mir", "doch"],
    ] {
        let output = cli(d.path(), &args);
        assert!(output.status.success());
        let text = String::from_utf8(output.stdout).unwrap();
        assert!(text.contains("Der Harness ohne 'r'"));
        assert!(!text.contains("Examples:"));
        let commands = text
            .split_once("Commands:\n")
            .unwrap()
            .1
            .split_once("\nOptions:")
            .unwrap()
            .0;
        let rows: Vec<_> = commands
            .lines()
            .filter(|line| !line.trim().is_empty())
            .collect();
        let phrases = [
            "fang an",
            "pass auf",
            "mach was",
            "was ist phase",
            "so hilf mir doch",
        ];
        assert_eq!(rows.len(), phrases.len());
        for (row, phrase) in rows.iter().zip(phrases) {
            let description = row.trim_start().strip_prefix(phrase).unwrap();
            assert!(description.starts_with("  "));
            assert!(!description.trim().is_empty());
        }
        assert!(
            text.contains("--no-color") && text.contains("--help") && text.contains("--version")
        );
    }
    assert!(Cli::try_parse_from(["hannes", "help"]).is_err());
}

#[test]
fn unsupported_commands_show_one_supplied_quote_and_face_without_execution() {
    let d = tempdir().unwrap();
    let quotes: Vec<_> = include_str!("../src/confused-quotes.txt").lines().collect();
    assert_eq!(quotes.len(), 41);
    for args in [
        vec!["quatsch"],
        vec!["mach"],
        vec!["mach", "falsch"],
        vec!["mach", "mal"],
        vec!["mach", "mal", "--geiz-ist-geil"],
        vec!["was", "ist"],
        vec!["fang", "an", "extra"],
        vec!["--unknown"],
        vec!["--no-color", "quatsch"],
    ] {
        let output = cli(d.path(), &args);
        assert_eq!(output.status.code(), Some(2), "{args:?}");
        assert!(output.stdout.is_empty());
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(stderr.contains(".-------.") && stderr.contains(r"\_________/"));
        let quote = stderr
            .lines()
            .find_map(|line| line.split_once("|  < ").map(|(_, quote)| quote))
            .unwrap();
        assert!(quotes.contains(&quote));
        assert!(stderr.contains("hannes so hilf mir doch"));
        assert!(!stderr.contains('\x1b'));
        assert!(!d.path().join(".hannes").exists());
    }
}
#[test]
fn colors_preserve_help_layout_and_use_semantic_status_colors() {
    let help = Cli::command().render_help().to_string();
    assert_eq!(terminal::render_help(&help, false), help);
    let colored = terminal::render_help(&help, true);
    assert!(colored.contains("\x1b[36mfang an\x1b[0m"));
    assert!(colored.contains("\x1b[1mCommands:\x1b[0m"));
    assert!(colored.contains("\x1b[36m--help\x1b[0m"));
    assert!(colored.contains("\x1b[36m--geiz-ist-geil\x1b[0m"));
    assert_eq!(
        terminal::paint("PASS", terminal::Tone::Success, true),
        "\x1b[32mPASS\x1b[0m"
    );
    assert_eq!(
        terminal::paint("FAIL", terminal::Tone::Failure, true),
        "\x1b[31mFAIL\x1b[0m"
    );
    let face = terminal::render_unknown_command(1, None, true);
    assert!(face.starts_with("\x1b[33m      .-------."));
    let changes = git_snapshot::Changes {
        state: "CURRENT".into(),
        total_files: Some(0),
        sample_paths: vec![],
        head_changed: false,
        line_counts: None,
    };
    assert!(terminal::render_changes(&changes, true).contains("\x1b[32mCURRENT\x1b[0m"));
    assert!(!terminal::render_changes(&changes, false).contains('\x1b'));
}
#[test]
fn piped_help_and_errors_never_include_ansi_even_if_color_is_forced() {
    let d = tempdir().unwrap();
    let state = tempdir().unwrap();
    for args in [
        vec!["--help"],
        vec!["so", "hilf", "mir", "doch"],
        vec!["--version"],
        vec!["quatsch"],
        vec!["--no-color", "quatsch"],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_hannes"))
            .args(&args)
            .current_dir(d.path())
            .env_remove("NO_COLOR")
            .env("CLICOLOR_FORCE", "1")
            .env("HANNES_STATE_DIR", state.path())
            .output()
            .unwrap();
        assert!(!output.stdout.contains(&0x1b), "stdout: {args:?}");
        assert!(!output.stderr.contains(&0x1b), "stderr: {args:?}");
    }
}

fn cli_with_state(root: &Path, state: &Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_hannes"))
        .args(args)
        .current_dir(root)
        .env("NO_COLOR", "1")
        .env("HANNES_STATE_DIR", state)
        .output()
        .unwrap()
}
fn patience(state: &Path) -> u8 {
    let value: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(state.join("patience")).unwrap()).unwrap();
    value
        .as_u64()
        .or_else(|| value.get("level").and_then(|v| v.as_u64()))
        .unwrap() as u8
}

#[test]
fn patience_persists_across_processes_and_repositories_and_saturates() {
    let first = tempdir().unwrap();
    let second = tempdir().unwrap();
    let state = tempdir().unwrap();
    let quotes: Vec<_> = include_str!("../src/confused-quotes.txt").lines().collect();
    for index in 1..=6 {
        let root = if index % 2 == 0 {
            first.path()
        } else {
            second.path()
        };
        let output = cli_with_state(root, state.path(), &["quatsch"]);
        assert_eq!(output.status.code(), Some(2));
        let expected = ((index - 1) as u8).min(4);
        assert_eq!(patience(state.path()), expected);
        let text = String::from_utf8(output.stderr).unwrap();
        let quote = text
            .lines()
            .find_map(|line| line.split_once("|  < ").map(|(_, quote)| quote))
            .unwrap();
        if expected == 4 {
            assert!(
                quotes
                    .iter()
                    .any(|original| original.to_uppercase() == quote)
            );
            assert!(text.contains("|   X   X   |") && text.contains(r"|   /###\   |"));
        } else {
            assert!(quotes.contains(&quote));
        }
    }
    for (args, expected) in [
        (vec!["--help"], 3),
        (vec!["so", "hilf", "mir", "doch"], 2),
        (vec!["--version"], 1),
        (vec!["pass", "auf"], 0),
        (vec!["--help"], 0),
    ] {
        let output = cli_with_state(first.path(), state.path(), &args);
        assert_eq!(patience(state.path()), expected);
        assert_eq!(
            output.status.code(),
            Some(if args == ["pass", "auf"] { 2 } else { 0 })
        );
        assert!(
            !String::from_utf8(output.stderr)
                .unwrap()
                .contains(".-------.")
        );
    }
    fs::write(state.path().join("patience"), "3\n").unwrap();
    let output = cli_with_state(first.path(), state.path(), &[]);
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(patience(state.path()), 3, "no command does not change mood");
}
#[test]
fn every_valid_command_works_and_calms_hannes_from_maximum_anger() {
    let d = project();
    let state = tempdir().unwrap();
    for args in [
        vec!["fang", "an"],
        vec!["pass", "auf"],
        vec!["mach", "was"],
        vec!["was", "ist", "phase"],
        vec!["so", "hilf", "mir", "doch"],
    ] {
        fs::write(state.path().join("patience"), "4\n").unwrap();
        let output = cli_with_state(d.path(), state.path(), &args);
        assert_eq!(
            output.status.code(),
            Some(0),
            "{args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(patience(state.path()), 3);
    }
}
#[test]
fn patience_recovers_invalid_state_and_does_not_block_commands_on_io_errors() {
    let directory = tempdir().unwrap();
    for invalid in [
        b"broken".as_slice(),
        b"5",
        b"255",
        b"-1",
        b"\xff",
        b"12345678901234567890",
    ] {
        fs::write(directory.path().join("patience"), invalid).unwrap();
        assert_eq!(hannes::mood::record_in(directory.path(), true).unwrap(), 0);
    }
    let unavailable = directory.path().join("not-a-directory");
    fs::write(&unavailable, "file").unwrap();
    for args in [vec!["--version"], vec!["quatsch"]] {
        let output = cli_with_state(directory.path(), &unavailable, &args);
        assert_eq!(
            output.status.code(),
            Some(if args == ["--version"] { 0 } else { 2 })
        );
        assert!(
            String::from_utf8(output.stderr)
                .unwrap()
                .contains("Stimmung konnte nicht gespeichert werden")
        );
    }
}
#[test]
fn all_five_faces_match_the_requested_expressions_and_accept_every_quote() {
    let eyes = [
        r"     /  o   o  \",
        r"     /  o   O  \",
        r"     /  -   -  \",
        r"     /  \   /  \",
        r"     /  \   /  \",
    ];
    let noses = [
        "    |     >     |",
        "    |     >     |",
        "    |     >     |",
        "    |   >   <   |",
        "    |   X   X   |",
    ];
    let mouths = [
        r"    |   \___/   |",
        "    |    ___    |",
        "    |    ___    |",
        r"    |   \___/   |",
        r"    |   /###\   |",
    ];
    for level in 0..=4 {
        for quote in include_str!("../src/confused-quotes.txt").lines() {
            let output = terminal::render_face(level, quote, false);
            let displayed = if level == 4 {
                quote.to_uppercase()
            } else {
                quote.into()
            };
            let expected = format!(
                "      .-------.\n{}\n{}\n{}  < {displayed}\n     \\_________/\n        |   |\n\nVersuch's mit: hannes so hilf mir doch\n",
                eyes[level as usize], noses[level as usize], mouths[level as usize]
            );
            assert_eq!(output, expected);
        }
    }
    assert!(terminal::render_face(0, "friendly", true).starts_with("\x1b[32m"));
    assert!(terminal::render_face(2, "annoyed", true).starts_with("\x1b[33m"));
    assert!(terminal::render_face(4, "angry", true).starts_with("\x1b[31m"));
}
#[test]
fn concurrent_cli_calls_do_not_lose_patience_updates() {
    use std::process::Stdio;
    let d = tempdir().unwrap();
    let state = tempdir().unwrap();
    for (args, expected, code) in [(vec!["quatsch"], 4, 2), (vec!["--version"], 0, 0)] {
        let mut children = Vec::new();
        for _ in 0..5 {
            children.push(
                Command::new(env!("CARGO_BIN_EXE_hannes"))
                    .args(&args)
                    .current_dir(d.path())
                    .env("HANNES_STATE_DIR", state.path())
                    .env("NO_COLOR", "1")
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn()
                    .unwrap(),
            );
        }
        for mut child in children {
            assert_eq!(child.wait().unwrap().code(), Some(code));
        }
        assert_eq!(patience(state.path()), expected);
    }
}

#[test]
fn one_valid_command_improves_the_next_error_face_and_help_has_no_debug_output() {
    let d = tempdir().unwrap();
    let state = tempdir().unwrap();
    let expressions = [
        r"/  o   o  \",
        r"/  o   O  \",
        r"/  -   -  \",
        "|   >   <   |",
        "|   X   X   |",
    ];
    for level in 0..=4 {
        let output = cli_with_state(d.path(), state.path(), &["foo"]);
        assert_eq!(output.status.code(), Some(2));
        assert_eq!(patience(state.path()), level);
        assert!(
            String::from_utf8(output.stderr)
                .unwrap()
                .contains(expressions[level as usize])
        );
    }
    for (args, expected) in [
        (vec!["--help"], 3),
        (vec!["so", "hilf", "mir", "doch"], 2),
        (vec!["-h"], 1),
        (vec!["--help"], 0),
        (vec!["so", "hilf", "mir", "doch"], 0),
    ] {
        let help = cli_with_state(d.path(), state.path(), &args);
        assert!(help.status.success());
        assert_eq!(patience(state.path()), expected);
        let text = String::from_utf8(help.stdout).unwrap();
        assert!(text.contains("Commands:"));
        assert!(
            !text.contains(".-------.") && !text.contains("Stufe") && !text.contains("Stimmung")
        );
        let error = cli_with_state(d.path(), state.path(), &["foo"]);
        assert_eq!(error.status.code(), Some(2));
        assert_eq!(patience(state.path()), expected);
        let text = String::from_utf8(error.stderr).unwrap();
        assert!(text.contains(expressions[expected as usize]));
        if expected == 0 {
            assert!(text.contains(r"|   \___/   |"));
        }
        assert!(!text.contains("Stufe"));
    }
    // The exact reported regression: yellow 1 -> good command -> green 0.
    let error = cli_with_state(d.path(), state.path(), &["foo"]);
    assert_eq!(error.status.code(), Some(2));
    assert_eq!(patience(state.path()), 1);
    let help = cli_with_state(d.path(), state.path(), &["so", "hilf", "mir", "doch"]);
    assert!(help.status.success());
    assert_eq!(patience(state.path()), 0);
    let error = cli_with_state(d.path(), state.path(), &["foo"]);
    assert_eq!(error.status.code(), Some(2));
    assert_eq!(patience(state.path()), 0);
    assert!(
        String::from_utf8(error.stderr)
            .unwrap()
            .contains(r"/  o   o  \")
    );
}
#[test]
fn legacy_integer_patience_is_migrated_and_recovers_without_resetting() {
    let state = tempdir().unwrap();
    fs::write(state.path().join("patience"), "3\n").unwrap();
    assert_eq!(hannes::mood::record_in(state.path(), false).unwrap(), 2);
    assert_eq!(hannes::mood::record_in(state.path(), true).unwrap(), 2);
    assert_eq!(hannes::mood::record_in(state.path(), true).unwrap(), 3);
}

#[test]
fn review_summary_is_visible_for_pass_and_fail_while_details_stay_separate() {
    for status in ["PASS", "FAIL"] {
        let d = project();
        enable(d.path());
        let mut value = verdict("product-consistency", status);
        value["reason"] =
            serde_json::json!("Settings navigation exposes labels and keyboard focus.");
        value["findings"][0]["message"] = serde_json::json!(
            "DETAIL_ONLY: Full explanation of a concrete trigger, consequence, and inspected evidence."
        );
        let provider = mock(vec![value]);
        let report = hannes::verify(d.path(), &provider).unwrap();
        assert_eq!(provider.calls.get(), 1);
        let short = terminal::render(&report, false);
        assert!(short.contains("SUMMARY"));
        assert!(short.contains("Settings navigation exposes labels and keyboard focus."));
        assert!(!short.contains("DETAIL_ONLY") && !short.contains(".hannes/README.md:1"));
        assert!(short.contains("hannes was ist phase --details"));
        let detailed = terminal::render_details(&report, false);
        assert!(detailed.contains("DETAIL_ONLY") && detailed.contains(".hannes/README.md:1"));
        assert!(
            report::read(d.path()).unwrap().results[1].findings[0]
                .message
                .contains("DETAIL_ONLY")
        );
    }
}
#[test]
fn provider_summary_must_be_short_nonempty_and_single_line() {
    let d = project();
    enable(d.path());
    let project = config::load(d.path()).unwrap();
    let snapshot = git_snapshot::capture(d.path()).unwrap();
    let active: Vec<_> = project.reviews.iter().filter(|r| r.enabled).collect();
    for reason in [
        " ".into(),
        "a".repeat(121),
        "first\nsecond".into(),
        "first\tsecond".into(),
        "first\u{2028}second".into(),
    ] {
        let mut value = verdict("product-consistency", "PASS");
        value["reason"] = serde_json::json!(reason);
        let json = serde_json::json!({"reviews":[value]}).to_string();
        assert!(hannes::reviews::parse(&project, &active, &snapshot, &json).is_err());
    }
    let mut value = verdict("product-consistency", "PASS");
    value["reason"] = serde_json::json!("ä".repeat(120));
    assert!(
        hannes::reviews::parse(
            &project,
            &active,
            &snapshot,
            &serde_json::json!({"reviews":[value]}).to_string()
        )
        .is_ok()
    );
}
#[cfg(unix)]
#[test]
fn detailed_status_reads_saved_findings_without_another_codex_call() {
    let d = project();
    enable(d.path());
    let mut value = verdict("product-consistency", "FAIL");
    value["reason"] = serde_json::json!("One keyboard accessibility issue found.");
    value["findings"][0]["message"] =
        serde_json::json!("DETAIL_ONLY: keyboard focus skips a visible control.");
    let provider = mock(vec![value]);
    hannes::verify(d.path(), &provider).unwrap();
    let bytes_before = fs::read(path(d.path(), "reports/last.json")).unwrap();
    let fake = tempdir().unwrap();
    let state = tempdir().unwrap();
    let executable = fake.path().join("codex");
    write_executable(
        &executable,
        "#!/bin/sh\nprintf 'called' > \"$HANNES_TEST_PROVIDER_MARKER\"\nexit 90\n",
    )
    .unwrap();
    let marker = fake.path().join("provider-called");
    let env_path = std::env::join_paths(
        std::iter::once(fake.path().to_path_buf())
            .chain(std::env::split_paths(&std::env::var_os("PATH").unwrap())),
    )
    .unwrap();
    for detailed in [false, true] {
        let mut command = Command::new(env!("CARGO_BIN_EXE_hannes"));
        command.args(["was", "ist", "phase"]);
        if detailed {
            command.arg("--details");
        }
        let output = command
            .current_dir(d.path())
            .env("PATH", &env_path)
            .env("HANNES_STATE_DIR", state.path())
            .env("HANNES_TEST_PROVIDER_MARKER", &marker)
            .env("NO_COLOR", "1")
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1));
        let text = String::from_utf8(output.stdout).unwrap();
        assert!(text.contains("One keyboard accessibility issue found."));
        assert_eq!(text.contains("DETAIL_ONLY"), detailed);
        assert!(text.contains("CURRENT"));
        assert!(!marker.exists());
    }
    assert_eq!(
        fs::read(path(d.path(), "reports/last.json")).unwrap(),
        bytes_before
    );
}
#[test]
fn compact_report_bounds_legacy_or_execution_error_reasons_without_losing_saved_details() {
    let d = project();
    let mut report = hannes::verify(d.path(), &Never).unwrap();
    report.results[0].reason = format!("first line\n{}", "x".repeat(4000));
    let compact = terminal::render(&report, false);
    let row = compact
        .lines()
        .find(|line| line.starts_with("check"))
        .unwrap();
    assert!(row.ends_with('…'));
    assert_eq!(
        compact
            .lines()
            .filter(|line| line.contains("first line"))
            .count(),
        1
    );
    assert!(terminal::render_details(&report, false).contains(&"x".repeat(4000)));
}

#[test]
fn progress_numbers_only_enabled_rules_and_announces_one_shared_provider_call() {
    use std::cell::RefCell;
    struct Observing<'a> {
        events: &'a RefCell<Vec<String>>,
        calls: Cell<u32>,
    }
    impl Provider for Observing<'_> {
        fn review(&self, request: &Request<'_>) -> Result<String> {
            self.calls.set(self.calls.get() + 1);
            let events = self.events.borrow();
            assert!(
                events
                    .iter()
                    .any(|line| line.starts_with("[2/4] PASS check"))
            );
            assert!(
                events
                    .iter()
                    .any(|line| line.starts_with("[3/4] LÄUFT review"))
            );
            assert!(
                events
                    .iter()
                    .any(|line| line.starts_with("[4/4] LÄUFT review"))
            );
            assert!(!events.iter().any(|line| line.starts_with("[3/4] PASS")));
            Ok(serde_json::json!({"reviews":request.reviews.iter().map(|review| verdict(&review.id,if review.id=="second" {"FAIL"} else {"PASS"})).collect::<Vec<_>>()} ).to_string())
        }
    }
    let d = project();
    enable(d.path());
    fs::write(path(d.path(),"checks/second.toml"),"id = \"second-check\"\ndescription = \"Second deterministic check\"\nenabled = true\ncommand = \"git --version\"\ntimeout_seconds = 30\n").unwrap();
    let original = fs::read_to_string(path(d.path(), "reviews/product-consistency.toml")).unwrap();
    fs::write(
        path(d.path(), "reviews/second.toml"),
        original.replace("id = \"product-consistency\"", "id = \"second\""),
    )
    .unwrap();
    let events = RefCell::new(Vec::new());
    let provider = Observing {
        events: &events,
        calls: Cell::new(0),
    };
    let report = hannes::verify_project_with_progress(
        config::load(d.path()).unwrap(),
        &provider,
        &mut |event| {
            events
                .borrow_mut()
                .push(terminal::render_progress(&event, false))
        },
    )
    .unwrap();
    assert_eq!(report.overall, Status::Fail);
    assert_eq!(provider.calls.get(), 1);
    let output = events.borrow().concat();
    for number in 1..=4 {
        assert!(output.contains(&format!("[{number}/4] LÄUFT")));
    }
    assert!(output.contains("[1/4] PASS check · always-ok"));
    assert!(output.contains("[4/4] FAIL review · second"));
    assert!(output.contains("Second deterministic check"));
    assert_eq!(output.matches("gebündelt in einem Aufruf").count(), 1);
    assert!(!output.contains("deliberately-failing"));
    assert!(!output.contains("Documented expectations inspected"));
}
#[test]
fn failed_checks_report_skipped_reviews_without_claiming_they_started() {
    let d = project();
    enable(d.path());
    replace(
        d.path(),
        "checks/deliberately-failing.toml",
        "enabled = false",
        "enabled = true",
    );
    let mut output = String::new();
    let report = hannes::verify_project_with_progress(
        config::load(d.path()).unwrap(),
        &Never,
        &mut |event| output.push_str(&terminal::render_progress(&event, false)),
    )
    .unwrap();
    assert_eq!(report.overall, Status::Fail);
    assert!(output.contains("[1/3] PASS check · always-ok"));
    assert!(output.contains("[2/3] FAIL check · deliberately-failing"));
    assert!(output.contains("[3/3] NOT_RUN review · product-consistency"));
    assert!(!output.contains("LÄUFT review") && !output.contains("Codex:"));
    assert!(path(d.path(), "reports/last.json").is_file());
}
#[cfg(unix)]
#[test]
fn cli_flushes_rule_start_while_the_check_is_still_running_and_keeps_raw_output_private() {
    use std::io::{BufRead, BufReader, Read};
    use std::process::Stdio;
    let d = project();
    let external = tempdir().unwrap();
    fs::write(path(d.path(),"checks/always-ok.toml"),r#"id = "always-ok"
description = "Wait for the test release signal"
enabled = true
command = "sh -c 'while [ ! -f \"$HANNES_TEST_RELEASE\" ]; do sleep 0.02; done; printf RAW_CHECK_OUTPUT'"
timeout_seconds = 5
"#).unwrap();
    let marker = external.path().join("release");
    let mut child = Command::new(env!("CARGO_BIN_EXE_hannes"))
        .args(["mach", "was"])
        .current_dir(d.path())
        .env("HANNES_STATE_DIR", external.path().join("state"))
        .env("HANNES_TEST_RELEASE", &marker)
        .env("NO_COLOR", "1")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut reader = BufReader::new(child.stderr.take().unwrap());
    let mut progress = String::new();
    loop {
        let mut line = String::new();
        assert!(
            reader.read_line(&mut line).unwrap() > 0,
            "missing start event: {progress}"
        );
        progress.push_str(&line);
        if line.starts_with("[1/1] LÄUFT") {
            break;
        }
    }
    assert!(
        child.try_wait().unwrap().is_none(),
        "progress arrived only after the process ended"
    );
    assert!(!path(d.path(), "reports/last.json").exists());
    assert!(progress.contains("Wait for the test release signal"));
    fs::write(marker, "continue").unwrap();
    reader.read_to_string(&mut progress).unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success(), "{progress}");
    assert!(progress.contains("[1/1] PASS"));
    assert!(!progress.contains("RAW_CHECK_OUTPUT"));
    assert!(
        !String::from_utf8(output.stdout)
            .unwrap()
            .contains("RAW_CHECK_OUTPUT")
    );
    assert_eq!(
        report::read(d.path()).unwrap().results[0]
            .stdout
            .as_ref()
            .unwrap()
            .text,
        "RAW_CHECK_OUTPUT"
    );
}
#[test]
fn progress_uses_semantic_colors_and_sanitizes_rule_descriptions() {
    let event = hannes::Progress::RuleStarted {
        number: 1,
        total: 2,
        kind: "check",
        id: "test",
        description: "Tests\nwith\x1bescape",
    };
    let plain = terminal::render_progress(&event, false);
    assert_eq!(plain.lines().count(), 1);
    assert!(!plain.contains('\x1b'));
    assert!(terminal::render_progress(&event, true).contains("\x1b[36mLÄUFT"));
    let item = report::Item::simple(
        "test",
        "check",
        Status::Fail,
        "Command exited unsuccessfully".into(),
    );
    let completed = hannes::Progress::RuleFinished {
        number: 1,
        total: 2,
        item: &item,
    };
    assert!(terminal::render_progress(&completed, true).contains("\x1b[31mFAIL"));
}

#[test]
fn usage_is_recorded_once_for_bundled_reviews_and_old_reports_still_load() {
    use hannes::{provider::Response, report::TokenUsage};
    struct Metered {
        calls: Cell<u32>,
    }
    impl Provider for Metered {
        fn review(&self, _: &Request<'_>) -> Result<String> {
            panic!("the usage-aware entry point must be used")
        }
        fn review_with_usage(&self, request: &Request<'_>) -> Response {
            self.calls.set(self.calls.get() + 1);
            assert_eq!(request.reviews.len(), 2);
            Response {
                verdict: Ok(serde_json::json!({"reviews": request.reviews.iter().map(|r| verdict(&r.id, "PASS")).collect::<Vec<_>>()} ).to_string()),
                usage: Some(TokenUsage {
                    input_tokens: 1000, cached_input_tokens: 800,
                    output_tokens: 200, reasoning_output_tokens: Some(50),
                    cache_write_input_tokens: None,
                }),
            }
        }
    }
    let d = project();
    enable(d.path());
    let metadata = fs::read_to_string(path(d.path(), "reviews/product-consistency.toml")).unwrap();
    fs::write(
        path(d.path(), "reviews/second.toml"),
        metadata.replace("id = \"product-consistency\"", "id = \"second\""),
    )
    .unwrap();
    let provider = Metered {
        calls: Cell::new(0),
    };
    let r = hannes::verify(d.path(), &provider).unwrap();
    assert_eq!(provider.calls.get(), 1);
    assert_eq!(r.overall, Status::Pass);
    assert_eq!(r.token_usage, Some(1200));
    assert_eq!(
        terminal::render(&r, false).matches("Codex-Tokens:").count(),
        1
    );
    assert_eq!(
        report::read(d.path()).unwrap().provider_usage,
        r.provider_usage
    );
    // Read a frozen older field set rather than recreating it from today's report.
    let legacy = support::historical_report(d.path(), "v1/report-review.json");
    fs::write(path(d.path(), "reports/last.json"), legacy.to_string()).unwrap();
    let legacy = report::read(d.path()).unwrap();
    assert_eq!(legacy.provider_usage, None);
    assert!(terminal::render(&legacy, false).contains("Codex-Tokens: nicht verfügbar"));
    assert_eq!(provider.calls.get(), 1);
}

fn copy_executable(source: &Path, executable: &Path) {
    #[cfg(unix)]
    {
        // A concurrent fork can inherit fs::copy's writable descriptor until exec,
        // causing ETXTBSY even after the copying thread closes it. Copy in a child
        // process so no writer for this executable exists in the test process.
        let output = Command::new("cp")
            .arg(source)
            .arg(executable)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "cannot copy executable fixture: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    #[cfg(not(unix))]
    fs::copy(source, executable).unwrap();
}

#[cfg(unix)]
fn write_executable(path: &Path, contents: impl AsRef<[u8]>) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    // Rust may hold writable handles to this staging file, but never executes it.
    let source = tempfile::NamedTempFile::new()?;
    fs::write(source.path(), contents)?;
    copy_executable(source.path(), path);
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
}

fn disposable_hannes(directory: &Path) -> PathBuf {
    let executable = directory.join(if cfg!(windows) {
        "hannes.exe"
    } else {
        "hannes"
    });
    copy_executable(Path::new(env!("CARGO_BIN_EXE_hannes")), &executable);
    executable
}

#[test]
fn third_consecutive_screaming_reaction_really_uninstalls_only_the_disposable_binary() {
    let installation = tempdir().unwrap();
    let state = tempdir().unwrap();
    let root = tempdir().unwrap();
    let executable = disposable_hannes(installation.path());
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let original = fs::metadata(env!("CARGO_BIN_EXE_hannes")).unwrap();
        let disposable = fs::metadata(&executable).unwrap();
        assert_ne!(
            (original.dev(), original.ino()),
            (disposable.dev(), disposable.ino()),
            "uninstall fixture must be an independent copy, not a hard link"
        );
    }
    let sentinel = installation.path().join("keep.txt");
    fs::write(&sentinel, "other installed files").unwrap();
    fs::write(root.path().join("keep.txt"), "project content").unwrap();
    for number in 1..=7 {
        let output = Command::new(&executable)
            .arg("foo")
            .current_dir(root.path())
            .env("HANNES_STATE_DIR", state.path())
            .env("NO_COLOR", "1")
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        let text = String::from_utf8(output.stderr).unwrap();
        if number < 7 {
            assert!(executable.exists());
            assert!(text.contains(".-------."));
            assert!(!text.contains("deinstalliert"));
        } else {
            assert_eq!(
                text,
                "\nHannes möchte nichts mehr mit dir zu tun haben und hat sich deinstalliert.\n\n"
            );
            assert!(!executable.exists());
        }
    }
    assert!(
        !state.path().exists(),
        "external mood data and its empty directory are removed"
    );
    assert!(Path::new(env!("CARGO_BIN_EXE_hannes")).is_file());
    assert_eq!(
        fs::read_to_string(sentinel).unwrap(),
        "other installed files"
    );
    assert_eq!(
        fs::read_to_string(root.path().join("keep.txt")).unwrap(),
        "project content"
    );
    assert!(Command::new(&executable).arg("--help").output().is_err());
    // Installing the executable again must not inherit the angry streak.
    disposable_hannes(installation.path());
    let first = Command::new(&executable)
        .arg("foo")
        .current_dir(root.path())
        .env("HANNES_STATE_DIR", state.path())
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    assert_eq!(first.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&first.stderr).contains(r"/  o   o  \"));
    assert_eq!(patience(state.path()), 0);
    assert!(executable.exists());
}

#[test]
fn a_valid_command_resets_the_screaming_streak_across_invocations() {
    let installation = tempdir().unwrap();
    let state = tempdir().unwrap();
    let root = tempdir().unwrap();
    let executable = disposable_hannes(installation.path());
    // Old JSON state remains readable, with no retroactive screaming count.
    fs::write(
        state.path().join("patience"),
        r#"{"level":4,"previous_unknown":false}"#,
    )
    .unwrap();
    for args in [
        vec!["foo"],
        vec!["foo"],
        vec!["--help"],
        vec!["foo"],
        vec!["foo"],
        vec!["foo"],
    ] {
        let output = Command::new(&executable)
            .args(&args)
            .current_dir(root.path())
            .env("HANNES_STATE_DIR", state.path())
            .env("NO_COLOR", "1")
            .output()
            .unwrap();
        assert!(executable.exists());
        assert!(!String::from_utf8_lossy(&output.stderr).contains("deinstalliert"));
    }
    let saved: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(state.path().join("patience")).unwrap()).unwrap();
    assert_eq!(saved["screaming_streak"], 2);
    assert_eq!(
        terminal::render_farewell(false),
        "\nHannes möchte nichts mehr mit dir zu tun haben und hat sich deinstalliert.\n\n"
    );
    assert!(terminal::render_farewell(true).starts_with("\n\x1b[31m"));
}

#[cfg(unix)]
#[test]
fn uninstall_failure_never_claims_success_or_removes_other_files() {
    use std::os::unix::fs::PermissionsExt;
    if unsafe { libc::geteuid() } == 0 {
        return;
    }
    let installation = tempdir().unwrap();
    let state = tempdir().unwrap();
    let executable = disposable_hannes(installation.path());
    fs::write(
        state.path().join("patience"),
        r#"{"level":4,"previous_unknown":true,"screaming_streak":2}"#,
    )
    .unwrap();
    fs::set_permissions(installation.path(), fs::Permissions::from_mode(0o555)).unwrap();
    let result = Command::new(&executable)
        .arg("foo")
        .env("HANNES_STATE_DIR", state.path())
        .env("NO_COLOR", "1")
        .output();
    fs::set_permissions(installation.path(), fs::Permissions::from_mode(0o755)).unwrap();
    let output = result.unwrap();
    assert_eq!(output.status.code(), Some(2));
    let text = String::from_utf8(output.stderr).unwrap();
    assert!(text.contains("konnte sich nicht deinstallieren"));
    assert!(!text.contains("hat sich deinstalliert") && !text.contains(".-------."));
    assert!(executable.exists());
}

#[test]
fn uninstall_preserves_foreign_state_directory_contents_and_project_configuration() {
    let installation = tempdir().unwrap();
    let state = tempdir().unwrap();
    let root = project();
    let executable = disposable_hannes(installation.path());
    fs::write(state.path().join("keep.txt"), "foreign file").unwrap();
    fs::write(
        state.path().join("patience"),
        r#"{"level":4,"previous_unknown":true,"screaming_streak":2}"#,
    )
    .unwrap();
    let config = fs::read(path(root.path(), "config.toml")).unwrap();
    let output = Command::new(&executable)
        .arg("foo")
        .current_dir(root.path())
        .env("HANNES_STATE_DIR", state.path())
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    assert!(String::from_utf8_lossy(&output.stderr).contains("hat sich deinstalliert"));
    assert!(!executable.exists());
    assert!(!state.path().join("patience").exists());
    assert!(!state.path().join("patience.lock").exists());
    assert_eq!(
        fs::read_to_string(state.path().join("keep.txt")).unwrap(),
        "foreign file"
    );
    assert_eq!(fs::read(path(root.path(), "config.toml")).unwrap(), config);
}

#[test]
fn state_overrides_inside_repositories_or_hannes_directories_are_never_removed() {
    let installation = tempdir().unwrap();
    let root = project();
    let executable = disposable_hannes(installation.path());
    for directory in [
        root.path().to_path_buf(),
        root.path().join("state"),
        root.path().join(".hannes"),
        root.path().join(".Hannes"), // Legacy spelling must remain protected from cleanup.
    ] {
        fs::create_dir_all(&directory).unwrap();
        let data = br#"{"level":4,"previous_unknown":true,"screaming_streak":2}"#;
        fs::write(directory.join("patience"), data).unwrap();
        let output = Command::new(&executable)
            .arg("foo")
            .current_dir(root.path())
            .env("HANNES_STATE_DIR", &directory)
            .env("NO_COLOR", "1")
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert!(executable.exists());
        assert_eq!(fs::read(directory.join("patience")).unwrap(), data);
        assert!(!String::from_utf8_lossy(&output.stderr).contains("hat sich deinstalliert"));
        assert!(!directory.join("patience.lock").exists());
    }
}

#[cfg(unix)]
#[test]
fn symlinked_state_overrides_cannot_reach_into_a_repository() {
    use std::os::unix::fs::symlink;
    let installation = tempdir().unwrap();
    let outside = tempdir().unwrap();
    let root = project();
    let executable = disposable_hannes(installation.path());
    let alias = outside.path().join("alias");
    symlink(root.path().join(".hannes"), &alias).unwrap();
    let config = fs::read(path(root.path(), "config.toml")).unwrap();
    let output = Command::new(&executable)
        .arg("foo")
        .current_dir(root.path())
        .env("HANNES_STATE_DIR", alias)
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    assert!(executable.exists());
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("Stimmung konnte nicht gespeichert werden")
    );
    assert_eq!(fs::read(path(root.path(), "config.toml")).unwrap(), config);
    assert!(!path(root.path(), "patience").exists());
}

#[test]
fn checks_only_never_calls_provider_and_counts_only_checks_even_on_failure() {
    use hannes::{Progress, VerificationMode, verify_project_with_mode};
    for case in ["pass", "fail", "error", "no-checks"] {
        let d = project();
        enable(d.path());
        if case == "fail" {
            replace(
                d.path(),
                "checks/deliberately-failing.toml",
                "enabled = false",
                "enabled = true",
            );
        } else if case == "error" {
            replace(
                d.path(),
                "checks/always-ok.toml",
                "git",
                "hannes-nonexistent-executable-for-test",
            );
        } else if case == "no-checks" {
            replace(
                d.path(),
                "checks/always-ok.toml",
                "enabled = true",
                "enabled = false",
            );
        }
        let mut plan = None;
        let mut review_event = false;
        let report = verify_project_with_mode(
            config::load(d.path()).unwrap(),
            &Never,
            VerificationMode::ChecksOnly,
            &mut |event| match event {
                Progress::Plan { total } => plan = Some(total),
                Progress::ReviewBatch { .. } | Progress::RuleStarted { kind: "review", .. } => {
                    review_event = true
                }
                _ => {}
            },
        )
        .unwrap();
        assert!(!review_event);
        assert!(report.checks_only);
        assert!(report.results.iter().all(|item| item.kind != "review"));
        assert_eq!(
            report.overall,
            match case {
                "fail" => Status::Fail,
                "error" => Status::Error,
                _ => Status::Pass,
            }
        );
        assert_eq!(
            plan,
            Some(match case {
                "fail" => 2,
                "no-checks" => 0,
                _ => 1,
            })
        );
        assert_eq!(report.token_usage, None);
        assert_eq!(report.provider_usage, None);
        assert!(report::read(d.path()).unwrap().checks_only);
        let text = terminal::render(&report, false);
        assert!(text.contains("Nur Checks") && text.contains("kein LLM-Aufruf"));
        if case == "no-checks" {
            assert!(text.contains("Keine aktiven Checks"));
        }
    }
}

#[test]
fn mach_was_and_geiz_flag_are_scoped_to_verification() {
    use hannes::cli::{Mach, Words};
    assert!(Cli::try_parse_from(["hannes", "mach", "mal"]).is_err());
    for enabled in [false, true] {
        let mut args = vec!["hannes", "mach", "was"];
        if enabled {
            args.push("--geiz-ist-geil");
        }
        let parsed = Cli::try_parse_from(args).unwrap();
        assert!(
            matches!(parsed.command, Words::Mach { command: Mach::Was { geiz_ist_geil, .. } } if geiz_ist_geil == enabled)
        );
    }
    assert!(Cli::try_parse_from(["hannes", "fang", "an", "--geiz-ist-geil"]).is_err());
    assert!(
        Cli::command()
            .render_help()
            .to_string()
            .contains("--geiz-ist-geil")
    );
}

#[cfg(unix)]
#[test]
fn checks_only_cli_skips_enabled_codex_reviews_and_status_keeps_the_scope() {
    let d = project();
    enable(d.path());
    let fake = tempdir().unwrap();
    let state = tempdir().unwrap();
    let executable = fake.path().join("codex");
    let marker = fake.path().join("called");
    write_executable(
        &executable,
        "#!/bin/sh\nif [ \"${1:-}\" = mcp ]; then printf '[]\\n'; exit 0; fi\nprintf called > \"$HANNES_PROVIDER_MARKER\"\nexit 23\n",
    )
    .unwrap();
    let search_path = std::env::join_paths(
        std::iter::once(fake.path().to_path_buf())
            .chain(std::env::split_paths(&std::env::var_os("PATH").unwrap())),
    )
    .unwrap();
    let run = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_hannes"))
            .args(args)
            .current_dir(d.path())
            .env("PATH", &search_path)
            .env("HANNES_PROVIDER_MARKER", &marker)
            .env("HANNES_STATE_DIR", state.path())
            .env("NO_COLOR", "1")
            .output()
            .unwrap()
    };
    let output = run(&["mach", "was", "--geiz-ist-geil"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!marker.exists());
    assert!(String::from_utf8_lossy(&output.stdout).contains("Nur Checks"));
    assert!(!String::from_utf8_lossy(&output.stderr).contains("Codex:"));
    let saved = fs::read(path(d.path(), "reports/last.json")).unwrap();
    for args in [
        vec!["was", "ist", "phase"],
        vec!["was", "ist", "phase", "--details"],
    ] {
        let status = run(&args);
        assert!(status.status.success());
        assert!(String::from_utf8_lossy(&status.stdout).contains("Nur Checks"));
        assert_eq!(
            fs::read(path(d.path(), "reports/last.json")).unwrap(),
            saved
        );
        assert!(!marker.exists());
    }
    // The flag is per invocation; the next ordinary run still invokes enabled reviews.
    let full = run(&["mach", "was"]);
    assert_eq!(full.status.code(), Some(2));
    assert!(marker.exists());
    assert!(!report::read(d.path()).unwrap().checks_only);
}

#[test]
fn old_reports_default_to_all_rules_and_inconsistent_scope_is_rejected() {
    let d = project();
    let mut old = support::historical_report(d.path(), "v1/report-minimal.json");
    fs::write(path(d.path(), "reports/last.json"), old.to_string()).unwrap();
    assert!(!report::read(d.path()).unwrap().checks_only);
    old["checks_only"] = true.into();
    old["token_usage"] = 100.into();
    fs::write(path(d.path(), "reports/last.json"), old.to_string()).unwrap();
    assert!(report::read(d.path()).is_err());
}

#[test]
fn quote_history_persists_globally_and_excludes_the_last_twenty_displayed_sayings() {
    use std::collections::VecDeque;
    let roots = [tempdir().unwrap(), tempdir().unwrap()];
    let state = tempdir().unwrap();
    let quotes = hannes::quotes::all();
    let mut recent = VecDeque::new();
    for turn in 0..45 {
        let root = roots[turn % 2].path();
        let output = cli_with_state(root, state.path(), &["foo"]);
        assert_eq!(output.status.code(), Some(2));
        let text = String::from_utf8(output.stderr).unwrap();
        let displayed = text
            .lines()
            .find_map(|line| line.split_once("|  < ").map(|(_, quote)| quote))
            .unwrap();
        let index = quotes.iter().position(|quote| *quote == displayed).unwrap();
        assert!(
            !recent.contains(&index),
            "repeat within 20 previous sayings at turn {turn}"
        );
        if recent.len() == 20 {
            recent.pop_front();
        }
        recent.push_back(index);
        let saved: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(state.path().join("patience")).unwrap())
                .unwrap();
        let indices: Vec<usize> = serde_json::from_value(saved["recent_quotes"].clone()).unwrap();
        assert_eq!(indices, recent.iter().copied().collect::<Vec<_>>());
        assert!(fs::metadata(state.path().join("patience")).unwrap().len() <= 512);
        // Calm Hannes without erasing his quote memory or triggering self-uninstall.
        assert!(
            cli_with_state(root, state.path(), &["--help"])
                .status
                .success()
        );
        let after: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(state.path().join("patience")).unwrap())
                .unwrap();
        assert_eq!(saved["recent_quotes"], after["recent_quotes"]);
    }
}

#[test]
fn simultaneous_errors_reserve_distinct_quotes_under_the_shared_state_lock() {
    let root = tempdir().unwrap();
    let state = tempdir().unwrap();
    let mut children = Vec::new();
    for _ in 0..5 {
        children.push(
            Command::new(env!("CARGO_BIN_EXE_hannes"))
                .arg("foo")
                .current_dir(root.path())
                .env("HANNES_STATE_DIR", state.path())
                .env("NO_COLOR", "1")
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .unwrap(),
        );
    }
    let mut displayed = std::collections::BTreeSet::new();
    for child in children {
        let output = child.wait_with_output().unwrap();
        assert_eq!(output.status.code(), Some(2));
        let text = String::from_utf8(output.stderr).unwrap();
        let quote = text
            .lines()
            .find_map(|line| line.split_once("|  < ").map(|(_, quote)| quote))
            .unwrap();
        assert!(displayed.insert(quote.to_uppercase()));
    }
    let saved: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(state.path().join("patience")).unwrap()).unwrap();
    let indices: Vec<usize> = serde_json::from_value(saved["recent_quotes"].clone()).unwrap();
    assert_eq!(indices.len(), 5);
    assert_eq!(
        indices
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        5
    );
}

#[test]
fn review_context_sections_keep_source_lines_merge_overlap_and_ignore_code_headings() {
    let d = project();
    let document = "# Policy\nIntro\n## Scope\nRequired\n```md\n## Scope\n```\n### Detail\nMore\n## Other\nUnrelated\n";
    fs::write(d.path().join("Policy.md"), document).unwrap();
    let metadata = path(d.path(), "reviews/product-consistency.toml");
    let original = fs::read_to_string(&metadata).unwrap();
    fs::write(&metadata, format!("{original}\nfiles = [\"./Policy.md\"]\n[[context]]\npath = \"./Policy.md\"\nheadings = [\"Scope\", \"Detail\"]\n")).unwrap();
    let p = config::load(d.path()).unwrap();
    assert_eq!(p.reviews[0].files, ["Policy.md"]);
    let excerpts =
        hannes::review_context::collect(&p.root, &[&p.reviews[0], &p.reviews[0]]).unwrap();
    assert_eq!(excerpts.len(), 1);
    assert_eq!((excerpts[0].start_line, excerpts[0].end_line), (3, 9));
    assert!(excerpts[0].text.contains("Required") && excerpts[0].text.contains("More"));
    assert!(!excerpts[0].text.contains("Unrelated"));
    // Missing and duplicate real headings fail even for disabled reviews.
    fs::write(
        &metadata,
        format!("{original}\n[[context]]\npath = \"Policy.md\"\nheadings = [\"Missing\"]\n"),
    )
    .unwrap();
    assert!(config::load(d.path()).is_err());
    fs::write(d.path().join("Policy.md"), "## Scope\nOne\n## Scope\nTwo\n").unwrap();
    fs::write(
        &metadata,
        format!("{original}\n[[context]]\npath = \"Policy.md\"\nheadings = [\"Scope\"]\n"),
    )
    .unwrap();
    assert!(config::load(d.path()).is_err());
    // A whole-file entry must not hide an invalid named section in another entry.
    fs::write(&metadata, format!("{original}\n[[context]]\npath = \"Policy.md\"\n[[context]]\npath = \"Policy.md\"\nheadings = [\"Missing\"]\n")).unwrap();
    assert!(config::load(d.path()).is_err());
}

#[test]
fn review_context_rejects_escaping_missing_and_excessive_sources_without_a_provider() {
    for extra in [
        "files = [\"../outside.swift\"]",
        "files = [\"missing.swift\"]",
        "files = [\".hannes\"]",
        "[[context]]\npath = \"../outside.md\"",
        "[[context]]\npath = \"missing.md\"",
        "[[context]]\npath = \"Policy.md\"",
    ] {
        let d = project();
        fs::write(d.path().join("Policy.md"), "x".repeat(65537)).unwrap();
        let file = path(d.path(), "reviews/product-consistency.toml");
        let original = fs::read_to_string(&file).unwrap();
        fs::write(file, format!("{original}\n{extra}\n")).unwrap();
        assert!(config::load(d.path()).is_err(), "{extra}");
    }
}

#[test]
fn focused_reviews_exclude_unrelated_paths_and_diff_and_include_untracked_entry_hints() {
    let d = project();
    fs::write(d.path().join("entry[1].swift"), "old entry\n").unwrap();
    fs::write(d.path().join("unrelated.swift"), "old unrelated\n").unwrap();
    git(d.path(), &["add", "entry[1].swift", "unrelated.swift"]);
    fs::write(d.path().join("entry[1].swift"), "new entry\n").unwrap();
    fs::write(d.path().join("unrelated.swift"), "UNRELATED_DIFF_SECRET\n").unwrap();
    fs::write(d.path().join("new.swift"), "new untracked entry\n").unwrap();
    fs::write(
        d.path().join("Policy.md"),
        "# Relevant\nRelevant requirement\n# Other\nUNRELATED_POLICY_SECRET\n",
    )
    .unwrap();
    let metadata = path(d.path(), "reviews/product-consistency.toml");
    let original = fs::read_to_string(&metadata).unwrap();
    fs::write(metadata, format!("{original}\nfiles = [\"entry[1].swift\", \"new.swift\"]\n[[context]]\npath = \"Policy.md\"\nheadings = [\"Relevant\"]\n")).unwrap();
    let p = config::load(d.path()).unwrap();
    let snapshot = git_snapshot::capture(d.path()).unwrap();
    let active = [&p.reviews[0]];
    let input = hannes::review_input::PreparedReviewInput::prepare(&p, &active, &snapshot).unwrap();
    let request = Request {
        project: &p,
        reviews: &active,
        snapshot: &snapshot,
        input: &input,
    };
    assert!(input.diff.contains("new entry") && !input.diff.contains("UNRELATED_DIFF_SECRET"));
    let prompt = hannes::provider::prompt(&request).unwrap();
    assert!(prompt.contains("new.swift") && prompt.contains("Relevant requirement"));
    assert!(!prompt.contains("UNRELATED_POLICY_SECRET") && !prompt.contains("unrelated.swift"));
    assert!(prompt.contains("starting points, not an exhaustive boundary"));
    assert!(prompt.contains("targeted searches") && prompt.contains("ONCE"));
}

#[cfg(unix)]
#[test]
fn lean_review_transport_disables_unlisted_mcp_and_honors_model_profile_and_allowlist() {
    let d = project();
    enable(d.path());
    let fake = tempdir().unwrap();
    let executable = fake.path().join("fake-codex");
    let args_file = fake.path().join("args");
    let input_file = fake.path().join("input");
    let list_marker = fake.path().join("listed");
    let payload =
        serde_json::json!({"reviews":[verdict("product-consistency", "PASS")]}).to_string();
    let script = format!(
        r#"#!/bin/sh
set -eu
if [ "${{1:-}}" = mcp ]; then
 printf listed >> '{}'
 printf '%s\n' '[{{"name":"memory"}},{{"name":"external-other"}}]'
 exit 0
fi
printf '%s\n' "$@" > '{}'
output=''
while [ $# -gt 0 ]; do
 case "$1" in --output-last-message) shift; output=$1;; esac
 shift
done
cat > '{}'
printf '%s' '{}' > "$output"
"#,
        list_marker.display(),
        args_file.display(),
        input_file.display(),
        payload
    );
    write_executable(&executable, script).unwrap();
    let provider = hannes::provider::Codex::new(executable);
    let result = hannes::verify(d.path(), &provider).unwrap();
    assert_eq!(result.overall, Status::Pass);
    let args = fs::read_to_string(&args_file).unwrap();
    assert!(args.contains("--model\ngpt-6.1-sol\n"));
    for flag in [
        "skills.include_instructions=false",
        "features.plugins=false",
        "features.apps=false",
        "agents.enabled=false",
        "model_reasoning_effort=\"medium\"",
        "tool_output_token_limit=2000",
        "mcp_servers.memory.enabled=false",
        "mcp_servers.external-other.enabled=false",
    ] {
        assert!(args.contains(flag), "missing {flag}: {args}");
    }
    assert!(
        fs::read_to_string(&input_file)
            .unwrap()
            .contains("bounded numbered line ranges")
    );
    replace(
        d.path(),
        "provider.toml",
        "mcp_servers = []",
        "mcp_servers = [\"memory\"]",
    );
    replace(
        d.path(),
        "provider.toml",
        "gpt-6.1-sol",
        "custom-review-model",
    );
    replace(
        d.path(),
        "provider.toml",
        "reasoning_effort = \"medium\"",
        "reasoning_effort = \"low\"",
    );
    let result = hannes::verify(d.path(), &provider).unwrap();
    assert_eq!(result.overall, Status::Pass);
    let args = fs::read_to_string(&args_file).unwrap();
    assert!(args.contains("mcp_servers.memory.enabled=true"));
    assert!(args.contains("mcp_servers.external-other.enabled=false"));
    assert!(
        args.contains("custom-review-model") && args.contains("model_reasoning_effort=\"low\"")
    );
    assert_eq!(fs::read_to_string(list_marker).unwrap(), "listedlisted");
}

#[cfg(unix)]
#[test]
fn failed_mcp_discovery_or_unavailable_allowlist_never_starts_a_review_or_leaks_config() {
    for mode in [
        "invalid-json",
        "exit",
        "oversized",
        "unknown-allowed",
        "unsafe-name",
    ] {
        let d = project();
        enable(d.path());
        if mode == "unknown-allowed" {
            replace(
                d.path(),
                "provider.toml",
                "mcp_servers = []",
                "mcp_servers = [\"missing\"]",
            );
        }
        let fake = tempdir().unwrap();
        let executable = fake.path().join("fake-codex");
        let marker = fake.path().join("review-called");
        let listing = match mode {
            "invalid-json" => "printf SENSITIVE_CONFIG",
            "exit" => "printf SENSITIVE_CONFIG >&2; exit 17",
            "oversized" => "printf '%070000d' 0",
            "unsafe-name" => "printf '[{\"name\":\"unsafe.name\"}]'",
            _ => "printf '[]'",
        };
        write_executable(&executable, format!("#!/bin/sh\nif [ \"${{1:-}}\" = mcp ]; then {listing}; exit 0; fi\nprintf called > '{}'\nexit 19\n", marker.display())).unwrap();
        let result = hannes::verify(d.path(), &hannes::provider::Codex::new(&executable)).unwrap();
        assert_eq!(result.overall, Status::Error, "{mode}");
        assert!(!marker.exists(), "{mode}");
        assert!(
            !fs::read_to_string(path(d.path(), "reports/last.json"))
                .unwrap()
                .contains("SENSITIVE_CONFIG")
        );
    }
}

#[test]
fn provider_profile_validation_is_strict_and_old_configs_remain_readable() {
    for (before, after) in [
        (
            "tool_output_token_limit = 2000",
            "tool_output_token_limit = 0",
        ),
        (
            "reasoning_effort = \"medium\"",
            "reasoning_effort = \"guess\"",
        ),
        (
            "mcp_servers = []",
            "mcp_servers = [\"duplicate\", \"duplicate\"]",
        ),
        ("mcp_servers = []", "mcp_servers = [\"\"]"),
    ] {
        let d = project();
        replace(d.path(), "provider.toml", before, after);
        assert!(config::load(d.path()).is_err());
    }
    let d = project();
    fs::write(
        path(d.path(), "provider.toml"),
        "provider = \"codex-cli\"\ntimeout_seconds = 180\n",
    )
    .unwrap();
    let p = config::load(d.path()).unwrap();
    assert!(p.provider.model.is_none());
    assert!(p.provider.mcp_servers.is_empty());
    assert!(p.provider.review_profile.reasoning_effort.is_none());
    assert_eq!(p.provider.review_profile.tool_output_token_limit, 2000);
}
