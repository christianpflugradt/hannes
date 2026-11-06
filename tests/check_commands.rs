//! CHK-001: command syntax is validated offline; processes receive exact arguments.
use hannes::{config, report::Status};
use std::{fs, path::Path, process::Command};

fn project() -> tempfile::TempDir {
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
    root
}

fn check(root: &Path, command: toml::Value, enabled: bool, directory: &str) {
    let value = toml::toml! {
        id = "syntax-test"
        description = "Check command syntax"
        enabled = enabled
        command = command
        working_directory = directory
        timeout_seconds = 10
    };
    fs::write(
        root.join(".hannes/checks/always-ok.toml"),
        format!(
            "[[checks]]\nrun_at = ['pre-push']\n{}",
            toml::to_string(&value).unwrap()
        ),
    )
    .unwrap();
}

fn cli(root: &Path, words: &[&str]) -> std::process::Output {
    let state = tempfile::tempdir().unwrap();
    Command::new(env!("CARGO_BIN_EXE_hannes"))
        .args(words)
        .current_dir(root)
        .env("HANNES_STATE_DIR", state.path())
        .env("NO_COLOR", "1")
        .output()
        .unwrap()
}

#[test]
fn pass_auf_rejects_invalid_commands_even_when_disabled_without_execution() {
    let root = project();
    let report = root.path().join(".hannes/reports/last.json");
    fs::write(&report, "previous report must remain untouched").unwrap();
    for source in [
        "",
        " \t ",
        "'' arg",
        "tool 'unclosed",
        "tool \"unclosed",
        "tool \\",
        "git config --local hannes.executed yes && git --version",
        "git --version || echo failed",
        "git --version; echo next",
        "git --version | tee out",
        "git --version > out",
        "git --version 2>> out",
        "git --version < in",
        "git --version &",
        "echo $HOME",
        "echo ${HOME}",
        "echo $(date)",
        "echo `date`",
        "ls *.rs",
        "ls file?.rs",
        "ls [ab].rs",
        "echo \"$HOME\"",
        "echo \"$(date)\"",
    ] {
        for enabled in [true, false] {
            check(root.path(), source.into(), enabled, ".");
            let output = cli(root.path(), &["pass", "auf"]);
            assert_eq!(output.status.code(), Some(2), "accepted {source:?}");
            let error = String::from_utf8_lossy(&output.stderr);
            assert!(
                error.contains("Invalid command in check 'syntax-test'"),
                "{error}"
            );
            if source.contains("&&") {
                assert!(
                    error.contains("Unsupported shell operator") && error.contains("npm script"),
                    "{error}"
                );
            }
            assert_eq!(
                fs::read_to_string(&report).unwrap(),
                "previous report must remain untouched"
            );
        }
    }
    assert!(
        !Command::new("git")
            .args(["config", "--local", "--get", "hannes.executed"])
            .current_dir(root.path())
            .output()
            .unwrap()
            .status
            .success()
    );
    assert!(!root.path().join("out").exists());
}

#[test]
fn arrays_are_rejected_in_configuration_version_three() {
    let root = project();
    {
        check(
            root.path(),
            toml::Value::Array(vec!["git".into(), "--version".into()]),
            true,
            ".",
        );
        let output = cli(root.path(), &["pass", "auf"]);
        assert_eq!(output.status.code(), Some(2));
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(
            error.contains("always-ok.toml") && error.contains("string"),
            "{error}"
        );
    }
}

#[test]
fn unavailable_executables_validate_then_fail_at_runtime() {
    let root = project();
    for source in [
        "hannes-nonexistent-test-program",
        "./missing/tools/check",
        "'missing tool/check'",
    ] {
        check(root.path(), source.into(), true, ".");
        let output = cli(root.path(), &["pass", "auf"]);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            cli(root.path(), &["mach", "was", "--plan"])
                .status
                .success()
        );
        let project = config::load(root.path()).unwrap();
        let check = project
            .checks
            .iter()
            .find(|check| check.id == "syntax-test")
            .unwrap();
        let result = hannes::checks::execute(&project, check);
        assert_eq!(result.status, Status::Error, "{source}: {}", result.reason);
        assert!(
            result.reason.contains("cannot start command"),
            "{}",
            result.reason
        );
    }
}

#[cfg(unix)]
#[test]
fn quoted_paths_and_literal_arguments_reach_a_native_process_without_a_shell() {
    use std::os::unix::fs::PermissionsExt;
    let root = project();
    let directory = root.path().join("work/tools");
    fs::create_dir_all(&directory).unwrap();
    let source = root.path().join("argument_printer.rs");
    fs::write(
        &source,
        r#"fn main() {
        for argument in std::env::args().skip(1) { println!("<{argument}>"); }
        eprintln!("{}", std::env::current_dir().unwrap().display());
    }"#,
    )
    .unwrap();
    assert!(
        Command::new("rustc")
            .arg(&source)
            .arg("-o")
            .arg(directory.join("argument printer"))
            .status()
            .unwrap()
            .success()
    );
    let poison = root.path().join("poison");
    fs::create_dir(&poison).unwrap();
    fs::write(
        poison.join("sh"),
        "#!/bin/sh\ntouch shell-was-used\nexit 99\n",
    )
    .unwrap();
    fs::set_permissions(poison.join("sh"), fs::Permissions::from_mode(0o755)).unwrap();
    let source = r#"'tools/argument printer' "two words" escaped\ space \"quote\" "Hello; world" 'a&&b|c<>d&' '$HOME' '$(date)' '`date`' '*.rs' ''"#;
    check(root.path(), source.into(), true, "work");
    assert!(cli(root.path(), &["pass", "auf"]).status.success());
    let state = tempfile::tempdir().unwrap();
    let paths = std::env::join_paths(
        std::iter::once(poison).chain(std::env::split_paths(&std::env::var_os("PATH").unwrap())),
    )
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_hannes"))
        .args(["mach", "was", "--geiz-ist-geil"])
        .current_dir(root.path())
        .env("PATH", paths)
        .env("HANNES_STATE_DIR", state.path())
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report = hannes::report::read(root.path()).unwrap();
    let item = &report.results[0];
    assert_eq!(item.status, Status::Pass);
    assert_eq!(
        item.stdout.as_ref().unwrap().text,
        "<two words>\n<escaped space>\n<\"quote\">\n<Hello; world>\n<a&&b|c<>d&>\n<$HOME>\n<$(date)>\n<`date`>\n<*.rs>\n<>\n"
    );
    assert_eq!(
        item.stderr.as_ref().unwrap().text.trim(),
        root.path()
            .join("work")
            .canonicalize()
            .unwrap()
            .to_str()
            .unwrap()
    );
    assert!(!root.path().join("work/shell-was-used").exists());
}

#[cfg(unix)]
#[test]
fn relative_executables_cannot_escape_through_symlinked_ancestors() {
    let root = project();
    let external = tempfile::tempdir().unwrap();
    std::os::unix::fs::symlink(external.path(), root.path().join("outside")).unwrap();
    check(root.path(), "outside/not-yet-installed".into(), true, ".");
    let error = config::load(root.path()).unwrap_err();
    assert!(format!("{error:#}").contains("escapes project"));
}
