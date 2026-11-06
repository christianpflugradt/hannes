use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::{
    io::Read,
    process::{Command, ExitStatus, Stdio},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

pub use crate::limits::MAX_CAPTURE_BYTES as OUTPUT_LIMIT;
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Capture {
    pub text: String,
    pub total_bytes: u64,
    pub truncated: bool,
}
#[derive(Debug)]
pub struct Output {
    pub status: ExitStatus,
    pub stdout: Capture,
    pub stderr: Capture,
    pub timed_out: bool,
    pub duration_ms: u64,
}
type Observer = Box<dyn FnMut(&[u8]) + Send>;
fn drain(mut stream: impl Read, mut observer: Option<Observer>) -> Result<Capture> {
    let mut bytes = Vec::new();
    let mut total = 0;
    let mut buffer = [0; 8192];
    loop {
        let n = stream.read(&mut buffer)?;
        if let Some(observer) = &mut observer {
            observer(&buffer[..n]);
        }
        if n == 0 {
            break;
        }
        total += n as u64;
        bytes.extend_from_slice(&buffer[..n.min(OUTPUT_LIMIT - bytes.len())]);
    }
    Ok(Capture {
        text: String::from_utf8_lossy(&bytes).into_owned(),
        total_bytes: total,
        truncated: total > OUTPUT_LIMIT as u64,
    })
}
#[cfg(unix)]
fn terminate_tree(child: &mut std::process::Child) {
    // Each child starts its own process group. Never signal Hannes' group.
    unsafe {
        libc::kill(-(child.id() as i32), libc::SIGKILL);
    }
    let _ = child.kill();
}
#[cfg(windows)]
fn terminate_tree(child: &mut std::process::Child) {
    let _ = Command::new("taskkill")
        .args(["/F", "/T", "/PID", &child.id().to_string()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    let _ = child.kill();
}
#[cfg(not(any(unix, windows)))]
fn terminate_tree(child: &mut std::process::Child) {
    let _ = child.kill();
}

pub fn run(command: &mut Command, timeout: Duration) -> Result<Output> {
    run_observed(command, timeout, None)
}

/// Observe stdout while draining it, including bytes beyond the retained capture.
/// An empty slice signals EOF. The observer must retain only bounded data.
pub fn run_with_stdout_observer(
    command: &mut Command,
    timeout: Duration,
    observer: impl FnMut(&[u8]) + Send + 'static,
) -> Result<Output> {
    run_observed(command, timeout, Some(Box::new(observer)))
}

fn run_observed(
    command: &mut Command,
    timeout: Duration,
    mut observer: Option<Observer>,
) -> Result<Output> {
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    let start = Instant::now();
    let mut child = command
        .spawn()
        .context("cannot start command; check executable and PATH")?;
    let out = child.stdout.take().unwrap();
    let err = child.stderr.take().unwrap();
    let (sender, receiver) = mpsc::channel();
    for (is_out, stream) in [
        (true, Box::new(out) as Box<dyn Read + Send>),
        (false, Box::new(err) as Box<dyn Read + Send>),
    ] {
        let sender = sender.clone();
        let observer = if is_out { observer.take() } else { None };
        thread::spawn(move || {
            let _ = sender.send((is_out, drain(stream, observer)));
        });
    }
    drop(sender);
    let mut timed_out = false;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {}
            Err(e) => {
                terminate_tree(&mut child);
                let _ = child.wait();
                return Err(e.into());
            }
        }
        if start.elapsed() >= timeout {
            timed_out = true;
            terminate_tree(&mut child);
            break child.wait()?;
        }
        thread::sleep(Duration::from_millis(10));
    };
    // Clean up background descendants too; otherwise inherited pipes could hang forever.
    #[cfg(unix)]
    terminate_tree(&mut child);
    let mut stdout = None;
    let mut stderr = None;
    for _ in 0..2 {
        match receiver.recv_timeout(Duration::from_secs(2)) {
            Ok((true, result)) => stdout = Some(result?),
            Ok((false, result)) => stderr = Some(result?),
            Err(_) => {
                terminate_tree(&mut child);
                bail!("output capture incomplete: descendant kept a pipe open");
            }
        }
    }
    Ok(Output {
        status,
        stdout: stdout.unwrap(),
        stderr: stderr.unwrap(),
        timed_out,
        duration_ms: start.elapsed().as_millis() as u64,
    })
}
