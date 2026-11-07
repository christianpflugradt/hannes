#![cfg(unix)]
mod support;
use hannes::{
    config,
    report::{self, Status},
};
use std::{fs, process::Command};
use support::{check, write};

#[test]
fn validated_execution_point_overrides_inherited_context_for_checks_and_provider_processes() {
    let root = tempfile::tempdir().unwrap();
    assert!(
        Command::new("git")
            .args(["init", "--quiet"])
            .current_dir(root.path())
            .status()
            .unwrap()
            .success()
    );
    config::init(root.path()).unwrap();
    write(
        root.path(),
        ".hannes/config.toml",
        "project = 'My_App'\n[execution_points]\ncustom = ['custom-point']\n",
    );
    let points = [
        "pre-commit",
        "commit-msg",
        "pre-push",
        "check",
        "custom-point",
    ];
    check(
        root.path(),
        "context",
        &points,
        r#"set -eu
test "$HANNES_EXECUTION_POINT" = "$EXPECTED_EXECUTION_POINT"
test "$HANNES_PARAM_LABEL" = 'preserved'
test "$(cat "$HANNES_RULE_DIRECTORY/resource")" = bundled
printf '%s' "$HANNES_EXECUTION_POINT"
"#,
    );
    let rule = root.path().join(".hannes/checks/context/rule.toml");
    fs::write(
        &rule,
        format!(
            "{}\n[parameters.label]\ntype = 'string'\nvalue = 'preserved'\n",
            fs::read_to_string(&rule).unwrap()
        ),
    )
    .unwrap();
    write(root.path(), ".hannes/checks/context/resource", "bundled\n");
    write(root.path(), "evidence", "Evidence\n");
    write(
        root.path(),
        ".hannes/reviews/context/rule.toml",
        &format!(
            "id = 'context-review'\ndescription = 'Context review'\nrun_at = {points:?}\n[review]\nprompt = 'prompt.md'\n"
        ),
    );
    write(
        root.path(),
        ".hannes/reviews/context/prompt.md",
        "Inspect evidence.",
    );
    let fake = tempfile::tempdir().unwrap();
    support::context_provider(fake.path());
    let path = std::env::join_paths(
        std::iter::once(fake.path().to_path_buf())
            .chain(std::env::split_paths(&std::env::var_os("PATH").unwrap())),
    )
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_hannes"))
        .args(["pass", "auf", "--auto-approve"])
        .current_dir(root.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    // No --at exercises the default pre-push context too.
    for point in points.into_iter().map(Some).chain([None]) {
        let expected = point.unwrap_or("pre-push");
        let calls = fake.path().join("calls");
        fs::write(&calls, "").unwrap();
        let mut command = Command::new(env!("CARGO_BIN_EXE_hannes"));
        command.args(["mach", "was"]);
        if let Some(point) = point {
            command.args(["--at", point]);
        }
        let output = command
            .current_dir(root.path())
            .env("PATH", &path)
            .env("HANNES_EXECUTION_POINT", "spoofed")
            .env("EXPECTED_EXECUTION_POINT", expected)
            .env("CONTEXT_CALLS", &calls)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{expected}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let report = report::read(root.path()).unwrap();
        assert_eq!(report.overall, Status::Pass);
        let item = report.results.iter().find(|r| r.id == "context").unwrap();
        assert_eq!(item.stdout.as_ref().unwrap().text, expected);
        assert_eq!(fs::read_to_string(calls).unwrap(), "discovery\nreview\n");
    }
}
