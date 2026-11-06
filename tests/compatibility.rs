//! COMPAT-001: replay frozen historical field sets, never synthesize old types from new ones.
mod support;
use hannes::{
    config,
    report::{self, Status},
    terminal,
};
use std::{fs, path::Path, process::Command};

fn copy_tree(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).unwrap();
    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let to = destination.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &to);
        } else {
            fs::copy(entry.path(), to).unwrap();
        }
    }
}
#[test]
fn historical_array_commands_are_rejected_but_reports_remain_readable() {
    for (version, schema) in [("v1", 1), ("v2", 2)] {
        let root = tempfile::tempdir().unwrap();
        assert!(
            Command::new("git")
                .args(["init", "--quiet"])
                .current_dir(root.path())
                .status()
                .unwrap()
                .success()
        );
        let fixtures = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/compat")
            .join(version);
        copy_tree(&fixtures.join("config"), &root.path().join(".hannes"));
        // Intentional pre-1.0 format break. Keep the original arrays frozen so
        // historical syntax cannot become accepted through a silent migration.
        let error = format!("{:#}", config::load(root.path()).unwrap_err());
        assert!(
            error.contains("checks/always-ok.toml") && error.contains("string"),
            "{error}"
        );
        let reports = root.path().join(".hannes/reports");
        fs::create_dir_all(&reports).unwrap();
        for name in if schema == 1 {
            vec!["report.json", "report-minimal.json", "report-review.json"]
        } else {
            vec!["report.json"]
        } {
            let value = support::historical_report(root.path(), &format!("{version}/{name}"));
            let serialized = serde_json::to_string(&value).unwrap();
            fs::write(reports.join("last.json"), &serialized).unwrap();
            let report = report::read(root.path()).unwrap();
            assert_eq!(report.schema_version, schema);
            assert_eq!(report.overall, Status::Pass);
            assert!(!report.checks_only);
            assert!(report.provider_usage.is_none());
            if schema == 1 {
                assert!(report.plan.is_none());
                assert!(terminal::render(&report, false).contains("Legacy-Report"));
            } else {
                let plan = report.plan.as_ref().unwrap();
                assert!(plan.gate && plan.selection_complete);
                assert_eq!(plan.required_ids, ["always-ok"]);
                assert_eq!(plan.profile, "local");
            }
            assert_eq!(
                fs::read_to_string(reports.join("last.json")).unwrap(),
                serialized
            );
        }
    }
}
