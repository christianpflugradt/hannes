use serde_json::Value;
use std::{fs, path::Path};

/// Substitute only installation identity; the historical field set stays frozen.
pub fn historical_report(root: &Path, fixture: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/compat")
        .join(fixture);
    let mut value: Value = serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
    value["repository"] = root
        .canonicalize()
        .unwrap()
        .to_string_lossy()
        .as_ref()
        .into();
    value
}
