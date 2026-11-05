use crate::{
    config::{Check, Project},
    process,
    report::{Item, Status},
};
use std::{
    process::{Command, Stdio},
    time::{Duration, Instant},
};
pub fn execute(project: &Project, check: &Check) -> Item {
    let start = Instant::now();
    let executable = if check.command[0].contains('/') || check.command[0].contains('\\') {
        project
            .root
            .join(&check.working_directory)
            .join(&check.command[0])
    } else {
        std::path::PathBuf::from(&check.command[0])
    };
    let mut command = Command::new(executable);
    command
        .args(&check.command[1..])
        .current_dir(project.root.join(&check.working_directory))
        .stdin(Stdio::null());
    let mut item = Item::simple(&check.id, "check", Status::Error, String::new());
    match process::run(&mut command, Duration::from_secs(check.timeout_seconds)) {
        Ok(output) => {
            item.status = if output.timed_out {
                Status::Error
            } else if output.status.success() {
                Status::Pass
            } else {
                Status::Fail
            };
            item.reason = if output.timed_out {
                "Command timed out".into()
            } else {
                format!("Command exited: {}", output.status)
            };
            item.exit_code = output.status.code();
            item.stdout = Some(output.stdout);
            item.stderr = Some(output.stderr);
        }
        Err(error) => item.reason = format!("{error:#}"),
    }
    item.duration_ms = start.elapsed().as_millis() as u64;
    item
}
