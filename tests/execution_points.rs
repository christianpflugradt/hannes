use hannes::{
    config,
    rules::{self, Assignment},
};
use std::path::PathBuf;
fn assignment(id: &str, enabled: bool, at: &[&str]) -> Assignment {
    Assignment {
        id: id.into(),
        enabled,
        run_at: at.iter().map(|s| s.to_string()).collect(),
        source: PathBuf::from(format!(".hannes/{id}.toml")),
    }
}
fn set(id: &str, at: &[&str], ids: &[&str]) -> (PathBuf, config::RuleSet) {
    (
        format!(".hannes/{id}.toml").into(),
        config::RuleSet {
            description: "Rule set".into(),
            id: id.into(),
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
fn resolution_obeys_membership_override_union_and_disabled_rules() {
    let definitions = [
        assignment("a", true, &["pre-commit"]),
        assignment("b", true, &[]),
        assignment("c", false, &[]),
    ];
    let sets = [
        set("one", &["pre-push", "check"], &["a", "b", "c"]),
        set("two", &["pre-commit", "check"], &["b"]),
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
    assert!(rules::resolve(&points(), &[assignment("a", false, &[])], &[]).is_ok());
}
#[test]
fn invalid_assignments_are_rejected_even_when_disabled_or_overridden() {
    let a = assignment("a", true, &[]);
    for sets in [
        vec![],
        vec![set("off", &[], &["a"])],
        vec![set("empty", &[], &["a"])],
        vec![set("bad", &["check"], &["missing"])],
        vec![set("nested", &["check"], &["nested"])],
        vec![set("dup", &["check"], &["a", "a"])],
        vec![set("same", &[], &[]), set("same", &[], &[])],
        vec![set("bad", &["chekc"], &[])],
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
        assert!(rules::resolve(&points(), &[rule], &[set("active", &["check"], &["a"])]).is_err());
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
