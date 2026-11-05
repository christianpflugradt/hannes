//! Remove the running executable and external patience data; preserve projects.
use anyhow::{Context, Result};

pub fn current() -> Result<()> {
    let executable = std::env::current_exe().context("cannot locate Hannes executable")?;
    // Clear the old streak before removing the binary, so a reinstall is friendly.
    // If state cleanup fails, keep the executable and report the failure.
    crate::mood::remove_state().context("cannot remove external Hannes state")?;
    remove(&executable)
}

#[cfg(not(windows))]
fn remove(executable: &std::path::Path) -> Result<()> {
    std::fs::remove_file(executable).context("cannot remove Hannes executable")
}

#[cfg(windows)]
fn remove(executable: &std::path::Path) -> Result<()> {
    use std::{
        os::windows::process::CommandExt,
        process::{Command, Stdio},
    };
    // Windows cannot unlink a mapped executable. Remove its installed name now,
    // then have a hidden helper delete the uniquely named file after we exit.
    let temporary = tempfile::Builder::new()
        .prefix(".hannes-uninstalled-")
        .suffix(".exe")
        .tempfile_in(executable.parent().context("executable has no parent")?)?
        .into_temp_path();
    std::fs::rename(executable, &temporary).context("cannot remove installed Hannes name")?;
    let helper = Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command",
            "Wait-Process -Id ([int]$env:HANNES_EXIT_PID) -ErrorAction SilentlyContinue; for ($i=0; $i -lt 100; $i++) { try { Remove-Item -LiteralPath $env:HANNES_REMOVED_EXE -Force -ErrorAction Stop; exit 0 } catch { Start-Sleep -Milliseconds 100 } }; exit 1"])
        .env("HANNES_EXIT_PID", std::process::id().to_string())
        .env("HANNES_REMOVED_EXE", temporary.as_os_str())
        .stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null())
        .creation_flags(0x08000000)
        .spawn();
    match helper {
        Ok(_) => {
            temporary.keep()?;
            Ok(())
        }
        Err(error) => {
            std::fs::rename(&temporary, executable)
                .context("cannot restore executable after cleanup helper failed")?;
            Err(error).context("cannot start Hannes cleanup helper")
        }
    }
}
