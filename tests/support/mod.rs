#![allow(dead_code)]
use std::{fs, path::Path};

pub fn write(root: &Path, path: &str, text: &str) {
    let path = root.join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

pub fn check(root: &Path, id: &str, at: &[&str], script: &str) {
    let interpreter = if cfg!(windows) { "cmd" } else { "sh" };
    write(
        root,
        &format!(".hannes/checks/{id}/rule.toml"),
        &format!(
            "id = {id:?}\ndescription = 'Test check'\nenabled = true\nrun_at = {at:?}\n[check]\nentrypoint = 'entrypoint'\ninterpreter = {interpreter:?}\ntimeout_seconds = 10\n"
        ),
    );
    write(root, &format!(".hannes/checks/{id}/entrypoint"), script);
}

pub fn review_path(root: &Path, id: &str) -> std::path::PathBuf {
    let directory = root.join(".hannes/reviews").join(id);
    fs::create_dir_all(&directory).unwrap();
    fs::copy(
        root.join(".hannes/reviews/product-consistency/prompt.md"),
        directory.join("prompt.md"),
    )
    .unwrap();
    directory.join("rule.toml")
}
