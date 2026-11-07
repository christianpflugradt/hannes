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
    execute_with_context(project, check, None)
}

pub fn execute_with_context(
    project: &Project,
    check: &Check,
    hook: Option<&crate::reconciliation::HookContext>,
) -> Item {
    let start = Instant::now();
    let mut item = Item::simple(&check.id, "check", Status::Error, String::new());
    item.blocking = check.blocking;
    let mut command = if let Some(interpreter) = &check.interpreter {
        let mut command = Command::new(interpreter);
        command.arg(&check.entrypoint);
        command
    } else {
        Command::new(&check.entrypoint)
    };
    command.env("HANNES_RULE_DIRECTORY", check.entrypoint.parent().unwrap());
    command.current_dir(&project.root).stdin(Stdio::null());
    if let Some(hook) = hook {
        hook.configure(&mut command);
    }
    match process::run(&mut command, Duration::from_secs(check.timeout_seconds)) {
        Ok(output) => {
            item.status = if output.timed_out || output.status.code() == Some(2) {
                Status::Error
            } else if output.status.success() {
                Status::Pass
            } else {
                Status::Fail
            };
            item.reason = if output.timed_out {
                "Command timed out".into()
            } else if item.status == Status::Error {
                "Check reported an execution error (exit 2)".into()
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
