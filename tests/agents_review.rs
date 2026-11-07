#![cfg(unix)]
mod support;
use hannes::{
    config,
    report::{self, Status},
};
use std::{fs, os::unix::fs::PermissionsExt, path::Path, process::Command};
use support::write;

fn git(root: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .current_dir(root)
        .args(args)
        .output()
        .unwrap();
    assert!(output.status.success(), "{:?}", output);
    String::from_utf8(output.stdout).unwrap().trim().into()
}

#[test]
fn repository_review_uses_complete_context_and_triggered_cli() {
    for mode in [
        "pass",
        "fail",
        "malformed",
        "credential",
        "gate",
        "rubric",
        "skip",
    ] {
        let root = tempfile::tempdir().unwrap();
        let tools = tempfile::tempdir().unwrap();
        git(root.path(), &["init", "-q"]);
        config::init(root.path()).unwrap();
        fs::remove_dir_all(root.path().join(".hannes/checks")).unwrap();
        fs::remove_dir_all(root.path().join(".hannes/reviews")).unwrap();
        for (path, text) in [
            (
                ".hannes/checks/agents-size/rule.toml",
                include_str!("../.hannes/checks/agents-size/rule.toml"),
            ),
            (
                ".hannes/reviews/agents-quality/rule.toml",
                include_str!("../.hannes/reviews/agents-quality/rule.toml"),
            ),
            (
                ".hannes/reviews/agents-quality/prompt.md",
                include_str!("../.hannes/reviews/agents-quality/prompt.md"),
            ),
            (
                ".hannes/providers/gemini-flash.toml",
                include_str!("../.hannes/providers/gemini-flash.toml"),
            ),
            (
                "scripts/check-word-count.py",
                include_str!("../scripts/check-word-count.py"),
            ),
            (
                "scripts/gemini-review.py",
                include_str!("../scripts/gemini-review.py"),
            ),
            ("AGENTS.md", include_str!("../AGENTS.md")),
            (
                "docs/agents-review-rubric.md",
                include_str!("../docs/agents-review-rubric.md"),
            ),
        ] {
            write(root.path(), path, text);
        }
        git(root.path(), &["add", "."]);
        git(
            root.path(),
            &[
                "-c",
                "user.name=Fixture",
                "-c",
                "user.email=fixture@example.invalid",
                "-c",
                "core.hooksPath=/dev/null",
                "commit",
                "-qm",
                "fixture",
            ],
        );
        if mode == "gate" {
            write(root.path(), "AGENTS.md", &"word ".repeat(1201));
        } else if mode == "rubric" {
            write(
                root.path(),
                "docs/agents-review-rubric.md",
                &format!(
                    "{}\nRubric change\n",
                    include_str!("../docs/agents-review-rubric.md")
                ),
            );
        } else if mode != "skip" {
            write(
                root.path(),
                "AGENTS.md",
                &format!("{}\nTEST_{mode}\n", include_str!("../AGENTS.md")),
            );
        } else {
            write(root.path(), "unrelated", "Unrelated change\n");
        }
        let expected = serde_json::json!({
            "AGENTS.md": fs::read_to_string(root.path().join("AGENTS.md")).unwrap().trim_end(),
            "docs/agents-review-rubric.md": fs::read_to_string(root.path().join("docs/agents-review-rubric.md")).unwrap().trim_end()
        });
        fs::write(tools.path().join("expected.json"), expected.to_string()).unwrap();
        write(
            tools.path(),
            "http-fixture",
            r#"#!/usr/bin/env python3
import json, os, pathlib, sys
here = pathlib.Path(__file__).parent
(here / 'called').write_text('yes')
assert os.environ['HANNES_MODEL'] == 'gemini-3.8-flash'
assert os.environ['GEMINI_API_KEY'] == 'offline-fixture'
prompt = sys.stdin.read()
assert 'response schema' in prompt
assert '<untrusted_diff>' not in prompt
assert 'inspect the listed entry files' not in prompt
line = next(x for x in prompt.splitlines() if x.startswith('Shared requirement excerpts,'))
excerpts = json.loads(line.split('(JSON data): ', 1)[1])
actual = {x['path']: x['text'] for x in excerpts}
assert actual == json.loads((here / 'expected.json').read_text()), actual
assert all(x['start_line'] == 1 for x in excerpts)
status = 'FAIL' if 'TEST_fail' in actual['AGENTS.md'] else 'PASS'
response = json.dumps({'reviews':[{'id':'agents-quality','status':status,'reason':'Guidance evaluated','findings':[{'message':'Concrete guidance evaluated','path':'AGENTS.md','line':1}]}]})
if 'TEST_malformed' in actual['AGENTS.md']:
    response = '```json\n' + response + '\n```'
import importlib.util
# Import the adapter without creating new files in the verified fixture.
sys.dont_write_bytecode = True
spec = importlib.util.spec_from_file_location('adapter', 'scripts/gemini-review.py')
adapter = importlib.util.module_from_spec(spec)
spec.loader.exec_module(adapter)
class Response:
    status = 200
    def __enter__(self): return self
    def __exit__(self, *args): pass
    def read(self, limit):
        return json.dumps({'candidates':[{'finishReason':'STOP','content':{'parts':[{'text':response}]}}]}).encode()
class Opener:
    def open(self, request, timeout):
        assert timeout == 600
        assert json.loads(request.data)['contents'][0]['parts'][0]['text'] == prompt
        return Response()
adapter.build_opener = lambda *args: Opener()
print(adapter.run(prompt))
"#,
        );
        fs::set_permissions(
            tools.path().join("http-fixture"),
            fs::Permissions::from_mode(0o755),
        )
        .unwrap();
        let provider_path = root.path().join(".hannes/providers/gemini-flash.toml");
        let provider_text = fs::read_to_string(&provider_path).unwrap().replace(
            "command = \"python3\"",
            &format!("command = {:?}", tools.path().join("http-fixture")),
        );
        fs::write(provider_path, provider_text).unwrap();
        let path = std::env::join_paths(
            std::iter::once(tools.path().to_path_buf())
                .chain(std::env::split_paths(&std::env::var_os("PATH").unwrap())),
        )
        .unwrap();
        let mut command = Command::new(env!("CARGO_BIN_EXE_hannes"));
        command
            .current_dir(root.path())
            .args(["mach", "was", "--at", "check"])
            .env("PATH", path)
            .env("HANNES_CHANGE_BASE", "HEAD")
            .env_remove("GEMINI_API_KEY");
        if mode != "credential" {
            command.env("GEMINI_API_KEY", "offline-fixture");
        }
        let output = command.output().unwrap();
        let saved = report::read(root.path()).unwrap();
        let expected_status = match mode {
            "gate" => Status::Fail,
            _ => Status::Pass,
        };
        assert_eq!(
            saved.overall, expected_status,
            "{mode}: {:?} {:?}",
            saved.results, output
        );
        assert_eq!(
            output.status.code(),
            Some(expected_status.exit_code()),
            "{mode}"
        );
        assert_eq!(
            tools.path().join("called").exists(),
            !matches!(mode, "credential" | "gate" | "skip" | "rubric"),
            "{mode}"
        );
        if matches!(mode, "skip" | "rubric") {
            assert!(
                saved
                    .plan
                    .omitted
                    .iter()
                    .any(|x| x.reason == "trigger_not_matched")
            );
            assert!(saved.results.iter().all(|x| x.kind != "review"));
        } else {
            let result = saved
                .results
                .iter()
                .find(|x| x.id == "agents-quality")
                .unwrap();
            let status = match mode {
                "fail" => Status::Fail,
                "malformed" | "credential" => Status::Error,
                "gate" => Status::NotRun,
                _ => Status::Pass,
            };
            assert_eq!(result.status, status, "{mode}");
            if mode != "gate" {
                assert!(!result.blocking);
            }
            if mode == "credential" {
                assert!(result.reason.contains("required environment variable"));
            }
        }
    }
}
