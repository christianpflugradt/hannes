//! REC-001: isolated configuration, reconciliation and real Git hook execution.
#![cfg(unix)]
use hannes::{
    config,
    reconciliation::{self, HookInvocation},
};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::Path,
    process::{Command, Output},
};
use tempfile::TempDir;

fn git(root: &Path, args: &[&str]) -> Output {
    Command::new("git")
        .args(args)
        .current_dir(root)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_AUTHOR_NAME", "Test")
        .env("GIT_AUTHOR_EMAIL", "test@example.invalid")
        .env("GIT_COMMITTER_NAME", "Test")
        .env("GIT_COMMITTER_EMAIL", "test@example.invalid")
        .output()
        .unwrap()
}
fn ok(output: Output) -> Output {
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    output
}
fn project(mappings: &str) -> TempDir {
    let root = tempfile::tempdir().unwrap();
    ok(git(root.path(), &["init", "--quiet"]));
    config::init(root.path()).unwrap();
    fs::create_dir(root.path().join(".hannes/profiles")).unwrap();
    for id in ["first", "second"] {
        fs::write(
            root.path().join(format!(".hannes/profiles/{id}.toml")),
            format!("id = '{id}'\nrules = ['always-ok']\n"),
        )
        .unwrap();
    }
    mappings_write(root.path(), mappings);
    root
}
fn mappings_write(root: &Path, mappings: &str) {
    fs::write(
        root.join(".hannes/config.toml"),
        format!("version = 2\nproject = 'My_App'\nmax_display_files = 5\n{mappings}\n"),
    )
    .unwrap();
}
fn synchronize(root: &Path) {
    reconciliation::reconcile(&config::load(root).unwrap(), &mut |_| Ok(true)).unwrap();
}
fn hook(root: &Path, event: &str) -> std::path::PathBuf {
    root.join(".git/hooks").join(event)
}
fn executable(path: &Path, text: &str) {
    fs::write(path, text).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}
fn cli(root: &Path, args: &[&str]) -> Output {
    let state = tempfile::tempdir().unwrap();
    Command::new(env!("CARGO_BIN_EXE_hannes"))
        .args(args)
        .current_dir(root)
        .env("HANNES_STATE_DIR", state.path())
        .env("NO_COLOR", "1")
        .output()
        .unwrap()
}
fn hooked_git(root: &Path, args: &[&str]) -> Output {
    let state = tempfile::tempdir().unwrap();
    let path = std::env::join_paths(
        std::iter::once(
            Path::new(env!("CARGO_BIN_EXE_hannes"))
                .parent()
                .unwrap()
                .to_path_buf(),
        )
        .chain(std::env::split_paths(&std::env::var_os("PATH").unwrap())),
    )
    .unwrap();
    Command::new("git")
        .args(args)
        .current_dir(root)
        .env("PATH", path)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_AUTHOR_NAME", "Test")
        .env("GIT_AUTHOR_EMAIL", "test@example.invalid")
        .env("GIT_COMMITTER_NAME", "Test")
        .env("GIT_COMMITTER_EMAIL", "test@example.invalid")
        .env("HANNES_STATE_DIR", state.path())
        .output()
        .unwrap()
}
fn checker(root: &Path, script: &str) {
    fs::write(root.join("checker.sh"), script).unwrap();
    fs::write(root.join(".hannes/checks/always-ok.toml"), "id = 'always-ok'\ndescription = 'Hook checker'\nenabled = true\ncommand = 'sh checker.sh'\ntimeout_seconds = 10\n").unwrap();
}

#[test]
fn configuration_validates_every_mapping_and_preserves_provider_validation() {
    let root = project("");
    for mappings in [
        "",
        "[hooks]",
        "[hooks.git]",
        "[hooks.git]\npre-commit = 'first'\ncommit-msg = 'second'\npre-push = 'all'",
    ] {
        mappings_write(root.path(), mappings);
        config::load(root.path()).unwrap();
    }
    for mappings in [
        "[hooks.git]\npost-commit = 'first'",
        "[hooks.git]\npre-commit = 'missing'",
        "[hooks.git]\npre-commit = 'Invalid ID'",
        "[hooks.git]\npre-commit = 1",
        "[hooks.git]\npre-commit = ['first']",
        "[hooks.git]\npre-commit = 'first'\npre-commit = 'second'",
        "[hooks]\nsvn = {}",
    ] {
        mappings_write(root.path(), mappings);
        assert!(config::load(root.path()).is_err(), "{mappings}");
        assert!(!hook(root.path(), "pre-commit").exists());
    }
    for version in [1, 3] {
        mappings_write(root.path(), "[hooks.git]");
        let path = root.path().join(".hannes/config.toml");
        fs::write(
            &path,
            fs::read_to_string(&path)
                .unwrap()
                .replace("version = 2", &format!("version = {version}")),
        )
        .unwrap();
        assert!(config::load(root.path()).is_err());
    }
    mappings_write(root.path(), "");
    let provider = root.path().join(".hannes/provider.toml");
    let original = fs::read_to_string(&provider).unwrap();
    for bad in [
        original.replace("codex-cli", "unsupported"),
        original.replace("180", "0"),
        original.replace("medium", "unsupported"),
    ] {
        fs::write(&provider, bad).unwrap();
        assert_eq!(cli(root.path(), &["pass", "auf"]).status.code(), Some(2));
    }
}

#[test]
fn approval_updates_removals_and_idempotency() {
    let root = project("[hooks.git]\npre-commit = 'first'");
    let p = config::load(root.path()).unwrap();
    let mut prompted = 0;
    assert!(
        reconciliation::reconcile(&p, &mut |plan| {
            prompted += 1;
            assert!(plan.display().contains("Create hook"));
            Ok(false)
        })
        .is_err()
    );
    assert_eq!(prompted, 1);
    assert!(!hook(root.path(), "pre-commit").exists());
    synchronize(root.path());
    let path = hook(root.path(), "pre-commit");
    let metadata = fs::metadata(&path).unwrap();
    reconciliation::reconcile(&p, &mut |_| panic!("synchronized state must not prompt")).unwrap();
    assert_eq!(
        metadata.modified().unwrap(),
        fs::metadata(&path).unwrap().modified().unwrap()
    );
    assert!(metadata.permissions().mode() & 0o111 != 0);
    ok(cli(root.path(), &["pass", "auf"]));
    mappings_write(
        root.path(),
        "[hooks.git]\npre-commit = 'second'\npre-push = 'first'",
    );
    synchronize(root.path());
    assert!(
        fs::read_to_string(&path)
            .unwrap()
            .contains("--profil 'second'")
    );
    assert!(hook(root.path(), "pre-push").exists());
    // A profile rename is a mapping update with ordinary configuration validation.
    fs::write(
        root.path().join(".hannes/profiles/second.toml"),
        "id = 'renamed'\nrules = ['always-ok']\n",
    )
    .unwrap();
    assert!(config::load(root.path()).is_err());
    mappings_write(root.path(), "[hooks.git]\npre-commit = 'renamed'");
    synchronize(root.path());
    assert!(!hook(root.path(), "pre-push").exists());
    assert!(
        fs::read_to_string(&path)
            .unwrap()
            .contains("--profil 'renamed'")
    );
    mappings_write(root.path(), "");
    synchronize(root.path());
    assert!(!path.exists());
}

#[test]
fn mixed_ownership_preserves_bytes_permissions_and_custom_exit_behavior() {
    let root = project("[hooks.git]\npre-commit = 'first'");
    let path = hook(root.path(), "pre-commit");
    let original = "#!/bin/sh\n# custom behavior\nprintf 'custom ran\\n' >&2\nexit 17\n";
    executable(&path, original);
    fs::set_permissions(&path, fs::Permissions::from_mode(0o750)).unwrap();
    synchronize(root.path());
    assert!(
        fs::read_to_string(&path)
            .unwrap()
            .ends_with(&original[10..])
    );
    assert_eq!(
        fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o750
    );
    let output = hooked_git(
        root.path(),
        &["commit", "--allow-empty", "-m", "test: example"],
    );
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("custom ran"));
    mappings_write(root.path(), "");
    synchronize(root.path());
    assert_eq!(fs::read_to_string(&path).unwrap(), original);
}

#[test]
fn conflicts_preserve_all_hooks_before_confirmation() {
    let root = project("[hooks.git]\npre-commit = 'first'\npre-push = 'first'");
    let path = hook(root.path(), "pre-push");
    for original in [
        "#!/usr/bin/env python3\nprint('custom')\n",
        "#!/bin/sh\n# BEGIN HANNES MANAGED\n",
        "#!/bin/sh\n# END HANNES MANAGED\n",
        "#!/bin/sh\n# Hannes hook wrapper\n",
        "#!/bin/sh\n# BEGIN HANNES MANAGED\n# Hannes repository: foreign\n# END HANNES MANAGED\n",
    ] {
        executable(&path, original);
        assert!(
            reconciliation::reconcile(&config::load(root.path()).unwrap(), &mut |_| panic!(
                "conflict must precede approval"
            ))
            .is_err()
        );
        assert_eq!(fs::read_to_string(&path).unwrap(), original);
        assert!(!hook(root.path(), "pre-commit").exists());
    }
    fs::remove_file(&path).unwrap();
    synchronize(root.path());
    let valid = fs::read_to_string(&path).unwrap();
    for invalid in [
        format!("{valid}{valid}"),
        valid.replace("# END HANNES MANAGED\n", ""),
        valid.replace("# BEGIN HANNES MANAGED", "# BEGIN HANNES MANAGED broken"),
    ] {
        fs::write(&path, &invalid).unwrap();
        assert!(reconciliation::inspect(&config::load(root.path()).unwrap()).is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), invalid);
    }
}

#[test]
fn unsafe_paths_symlinks_and_foreign_ownership_fail_safely() {
    let root = project("[hooks.git]\npre-commit = 'first'");
    let outside = tempfile::tempdir().unwrap();
    ok(git(
        root.path(),
        &["config", "core.hooksPath", outside.path().to_str().unwrap()],
    ));
    assert!(reconciliation::inspect(&config::load(root.path()).unwrap()).is_err());
    assert_eq!(fs::read_dir(outside.path()).unwrap().count(), 0);
    ok(git(root.path(), &["config", "--unset", "core.hooksPath"]));
    let path = hook(root.path(), "pre-commit");
    std::os::unix::fs::symlink(outside.path().join("missing"), &path).unwrap();
    assert!(reconciliation::inspect(&config::load(root.path()).unwrap()).is_err());
    fs::remove_file(&path).unwrap();
    fs::create_dir(&path).unwrap();
    assert!(reconciliation::inspect(&config::load(root.path()).unwrap()).is_err());
    fs::remove_dir(&path).unwrap();
    synchronize(root.path());
    let other = project("[hooks.git]\npre-commit = 'first'");
    fs::copy(&path, hook(other.path(), "pre-commit")).unwrap();
    assert!(reconciliation::inspect(&config::load(other.path()).unwrap()).is_err());
    ok(git(root.path(), &["config", "core.hooksPath", "linked"]));
    std::os::unix::fs::symlink(outside.path(), root.path().join("linked")).unwrap();
    assert!(reconciliation::inspect(&config::load(root.path()).unwrap()).is_err());
}

#[test]
fn commands_block_checks_on_noninteractive_drift_and_keep_plan_read_only() {
    let root = project("[hooks.git]\npre-commit = 'first'");
    checker(root.path(), "git config --local hannes.executed yes\n");
    for args in [
        &["pass", "auf"][..],
        &["mach", "was", "--profil", "first"][..],
    ] {
        let output = cli(root.path(), args);
        assert_eq!(output.status.code(), Some(2));
        assert!(String::from_utf8_lossy(&output.stderr).contains("terminal"));
        assert!(
            !git(root.path(), &["config", "--get", "hannes.executed"])
                .status
                .success()
        );
        assert!(!hook(root.path(), "pre-commit").exists());
    }
    ok(cli(
        root.path(),
        &["mach", "was", "--profil", "first", "--plan"],
    ));
    assert!(!hook(root.path(), "pre-commit").exists());
    synchronize(root.path());
    ok(cli(root.path(), &["mach", "was", "--profil", "first"]));
    assert!(
        git(root.path(), &["config", "--get", "hannes.executed"])
            .status
            .success()
    );
}

#[test]
fn real_commit_hooks_forward_message_path_and_preserve_index_and_worktree() {
    let root = project("[hooks.git]\npre-commit = 'first'\ncommit-msg = 'second'");
    checker(
        root.path(),
        "set -eu\ncase \"$HANNES_GIT_HOOK\" in\npre-commit) test \"$HANNES_GIT_HOOK_ARG_COUNT\" = 0;;\ncommit-msg) test \"$HANNES_GIT_HOOK_ARG_COUNT\" = 1; grep -q '^test: accepted' \"$HANNES_GIT_HOOK_ARG_1\";;\n*) exit 19;;\nesac\n",
    );
    fs::write(root.path().join("file"), "staged\n").unwrap();
    ok(git(root.path(), &["add", "file"]));
    fs::write(root.path().join("file"), "unstaged\n").unwrap();
    let index = git(root.path(), &["ls-files", "--stage"]).stdout;
    synchronize(root.path());
    ok(hooked_git(root.path(), &["commit", "-m", "test: accepted"]));
    assert_eq!(
        fs::read_to_string(root.path().join("file")).unwrap(),
        "unstaged\n"
    );
    assert_eq!(git(root.path(), &["ls-files", "--stage"]).stdout, index);
    assert_eq!(
        hannes::report::read(root.path())
            .unwrap()
            .plan
            .unwrap()
            .profile,
        "second"
    );
    assert!(
        !hooked_git(root.path(), &["commit", "--allow-empty", "-m", "invalid"])
            .status
            .success()
    );
    let before = fs::read(hook(root.path(), "pre-commit")).unwrap();
    mappings_write(
        root.path(),
        "[hooks.git]\npre-commit = 'first'\ncommit-msg = 'first'",
    );
    let output = hooked_git(
        root.path(),
        &["commit", "--allow-empty", "-m", "test: drift"],
    );
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("hannes pass auf"));
    assert_eq!(fs::read(hook(root.path(), "pre-commit")).unwrap(), before);
}

#[test]
fn actual_push_hook_blocks_and_allows_pushes() {
    let root = project("[hooks.git]\npre-push = 'first'");
    let remote = tempfile::tempdir().unwrap();
    ok(git(remote.path(), &["init", "--bare", "--quiet"]));
    ok(git(
        root.path(),
        &["commit", "--allow-empty", "-m", "test: initial"],
    ));
    checker(root.path(), "exit 23\n");
    synchronize(root.path());
    assert!(
        !hooked_git(
            root.path(),
            &[
                "push",
                remote.path().to_str().unwrap(),
                "HEAD:refs/heads/test"
            ]
        )
        .status
        .success()
    );
    assert!(
        !git(remote.path(), &["rev-parse", "--verify", "refs/heads/test"])
            .status
            .success()
    );
    checker(
        root.path(),
        "test \"$HANNES_GIT_HOOK\" = pre-push && test \"$HANNES_GIT_HOOK_ARG_COUNT\" = 2\n",
    );
    ok(hooked_git(
        root.path(),
        &[
            "push",
            remote.path().to_str().unwrap(),
            "HEAD:refs/heads/test",
        ],
    ));
}

#[test]
fn custom_hooks_directory_and_worktrees_use_git_resolution() {
    let root = project("[hooks.git]\npre-commit = 'first'");
    ok(git(
        root.path(),
        &["config", "core.hooksPath", "custom hooks/nested"],
    ));
    synchronize(root.path());
    assert!(root.path().join("custom hooks/nested/pre-commit").exists());
    ok(hooked_git(
        root.path(),
        &["commit", "--allow-empty", "-m", "test: custom"],
    ));
    ok(git(root.path(), &["config", "--unset", "core.hooksPath"]));
    // Track configuration so the linked tree shares identical declarations.
    ok(git(root.path(), &["add", ".hannes"]));
    ok(git(root.path(), &["commit", "-m", "test: configuration"]));
    synchronize(root.path());
    let linked = tempfile::tempdir().unwrap();
    let path = linked.path().join("worktree");
    ok(git(
        root.path(),
        &[
            "worktree",
            "add",
            "--quiet",
            "-b",
            "linked",
            path.to_str().unwrap(),
        ],
    ));
    assert!(path.join(".git").is_file());
    assert!(
        reconciliation::inspect(&config::load(&path).unwrap())
            .unwrap()
            .synchronized()
    );
    ok(hooked_git(
        &path,
        &["commit", "--allow-empty", "-m", "test: linked"],
    ));
}

#[test]
fn invocation_validation_recursion_and_missing_executable_block_commits() {
    let root = project("[hooks.git]\npre-commit = 'first'");
    synchronize(root.path());
    let p = config::load(root.path()).unwrap();
    assert!(
        reconciliation::prepare(
            &p,
            Some(HookInvocation {
                event: "pre-commit".into(),
                path: "checker.sh".into(),
                arguments: vec![]
            }),
            Some("first")
        )
        .is_err()
    );
    assert!(
        reconciliation::prepare(
            &p,
            Some(HookInvocation {
                event: "pre-commit".into(),
                path: hook(root.path(), "pre-commit"),
                arguments: vec![]
            }),
            Some("second")
        )
        .is_err()
    );
    checker(root.path(), "hannes mach was --profil first\n");
    let output = hooked_git(
        root.path(),
        &["commit", "--allow-empty", "-m", "test: recurse"],
    );
    assert!(!output.status.success());
    let report = hannes::report::read(root.path()).unwrap();
    assert!(
        report.results[0]
            .stderr
            .as_ref()
            .unwrap()
            .text
            .contains("recursive Hannes hook execution blocked")
    );
    // Invoke Git with a PATH containing Git/sh but no Hannes.
    let bins = tempfile::tempdir().unwrap();
    let git_path = Command::new("sh")
        .args(["-c", "command -v git"])
        .output()
        .unwrap();
    std::os::unix::fs::symlink(
        String::from_utf8(git_path.stdout).unwrap().trim(),
        bins.path().join("git"),
    )
    .unwrap();
    let output = Command::new(bins.path().join("git"))
        .args(["commit", "--allow-empty", "-m", "test: unavailable"])
        .current_dir(root.path())
        .env("PATH", bins.path())
        .env("GIT_AUTHOR_NAME", "Test")
        .env("GIT_AUTHOR_EMAIL", "test@example.invalid")
        .env("GIT_COMMITTER_NAME", "Test")
        .env("GIT_COMMITTER_EMAIL", "test@example.invalid")
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("hannes")
            && String::from_utf8_lossy(&output.stderr).contains("not found"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn stale_approval_and_interrupted_changes_can_be_recovered() {
    let root = project("[hooks.git]\npre-commit = 'first'\npre-push = 'first'");
    let p = config::load(root.path()).unwrap();
    let plan = reconciliation::inspect(&p).unwrap();
    executable(&hook(root.path(), "pre-push"), "#!/bin/sh\nexit 0\n");
    assert!(reconciliation::apply(&p, &plan).is_err());
    assert!(!hook(root.path(), "pre-commit").exists());
    synchronize(root.path());
    // Simulate interruption after one complete atomic replacement in a multi-file plan.
    mappings_write(
        root.path(),
        "[hooks.git]\npre-commit = 'second'\npre-push = 'second'",
    );
    let path = hook(root.path(), "pre-commit");
    fs::write(
        &path,
        fs::read_to_string(&path)
            .unwrap()
            .replace("--profil 'first'", "--profil 'second'"),
    )
    .unwrap();
    assert!(
        !reconciliation::inspect(&config::load(root.path()).unwrap())
            .unwrap()
            .synchronized()
    );
    synchronize(root.path());
    assert!(
        fs::read_to_string(hook(root.path(), "pre-push"))
            .unwrap()
            .contains("--profil 'second'")
    );
}

#[test]
fn actual_terminal_approval_and_rejection_gate_execution() {
    use std::{
        io::{Read, Write},
        os::fd::FromRawFd,
    };
    fn terminal(root: &Path, args: &[&str], answer: &[u8]) -> std::process::ExitStatus {
        let (mut master_fd, mut slave_fd) = (-1, -1);
        // A real PTY exercises terminal detection; pipes deliberately cannot approve.
        assert_eq!(
            unsafe {
                libc::openpty(
                    &mut master_fd,
                    &mut slave_fd,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                )
            },
            0
        );
        let mut master = unsafe { fs::File::from_raw_fd(master_fd) };
        let slave = unsafe { fs::File::from_raw_fd(slave_fd) };
        let state = tempfile::tempdir().unwrap();
        let mut child = Command::new(env!("CARGO_BIN_EXE_hannes"))
            .args(args)
            .current_dir(root)
            .env("HANNES_STATE_DIR", state.path())
            .env("NO_COLOR", "1")
            .stdin(slave.try_clone().unwrap())
            .stdout(slave.try_clone().unwrap())
            .stderr(slave)
            .spawn()
            .unwrap();
        master.write_all(answer).unwrap();
        let reader = std::thread::spawn(move || {
            let mut bytes = [0; 4096];
            while matches!(master.read(&mut bytes), Ok(n) if n > 0) {}
        });
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
        let status = loop {
            if let Some(status) = child.try_wait().unwrap() {
                break status;
            }
            if std::time::Instant::now() > deadline {
                child.kill().unwrap();
                panic!("terminal reconciliation timed out");
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        };
        reader.join().unwrap();
        status
    }
    let root = project("[hooks.git]\npre-commit = 'first'");
    checker(root.path(), "git config --local hannes.executed yes\n");
    assert_eq!(
        terminal(root.path(), &["mach", "was", "--profil", "first"], b"n\n").code(),
        Some(2)
    );
    assert!(!hook(root.path(), "pre-commit").exists());
    assert!(
        !git(root.path(), &["config", "--get", "hannes.executed"])
            .status
            .success()
    );
    assert!(terminal(root.path(), &["pass", "auf"], b"yes\n").success());
    assert!(hook(root.path(), "pre-commit").exists());
    mappings_write(root.path(), "[hooks.git]\npre-commit = 'second'");
    assert!(terminal(root.path(), &["mach", "was", "--profil", "first"], b"y\n").success());
    assert!(
        git(root.path(), &["config", "--get", "hannes.executed"])
            .status
            .success()
    );
}
