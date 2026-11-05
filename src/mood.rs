//! User-wide patience state; independent of project verification.
use anyhow::{Context, Result, bail, ensure};
use std::{
    collections::VecDeque,
    env,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    thread,
    time::{Duration, Instant},
};

pub const MAX_LEVEL: u8 = 4;

#[derive(Default)]
pub struct Reaction {
    pub level: u8,
    pub leave: bool,
    pub quote_index: Option<usize>,
}

#[derive(Default, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
struct State {
    level: u8,
    previous_unknown: bool,
    #[serde(default)]
    screaming_streak: u8,
    #[serde(default)]
    recent_quotes: VecDeque<usize>,
}

fn decode(bytes: &[u8]) -> State {
    if let Ok(mut state) = serde_json::from_slice::<State>(bytes) {
        if state.level <= MAX_LEVEL && state.screaming_streak <= 3 {
            // Keep the latest valid unique indices if an old/edited history is oversized.
            let mut recent = VecDeque::new();
            for &index in state.recent_quotes.iter().rev() {
                if index < crate::quotes::all().len() && !recent.contains(&index) {
                    recent.push_front(index);
                    if recent.len() == crate::quotes::HISTORY_LIMIT {
                        break;
                    }
                }
            }
            state.recent_quotes = recent;
            return state;
        }
    }
    // Migrate the original integer state without resetting an existing mood.
    if let Some(level) = std::str::from_utf8(bytes)
        .ok()
        .and_then(|s| s.trim().parse::<u8>().ok())
        .filter(|level| *level <= MAX_LEVEL)
    {
        return State {
            level,
            previous_unknown: level > 0,
            screaming_streak: 0,
            recent_quotes: VecDeque::new(),
        };
    }
    State::default()
}

pub fn state_directory() -> Result<PathBuf> {
    if let Some(directory) = env::var_os("HANNES_STATE_DIR") {
        let directory = PathBuf::from(directory);
        ensure!(
            directory.is_absolute(),
            "HANNES_STATE_DIR must be an absolute directory"
        );
        return Ok(directory);
    }
    #[cfg(target_os = "macos")]
    let directory = PathBuf::from(env::var_os("HOME").context("HOME is unavailable")?)
        .join("Library/Application Support/Hannes");
    #[cfg(windows)]
    let directory =
        PathBuf::from(env::var_os("LOCALAPPDATA").context("LOCALAPPDATA is unavailable")?)
            .join("Hannes");
    #[cfg(not(any(target_os = "macos", windows)))]
    let directory = match env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
    {
        Some(base) => base.join("hannes"),
        None => PathBuf::from(env::var_os("HOME").context("HOME is unavailable")?)
            .join(".local/state/hannes"),
    };
    ensure!(
        directory.is_absolute(),
        "Hannes state directory must be absolute"
    );
    Ok(directory)
}

pub fn record(unknown: bool) -> Result<u8> {
    record_in(&state_directory()?, unknown)
}

pub fn react(unknown: bool) -> Result<Reaction> {
    update_in(&state_directory()?, unknown)
}

/// Serialize small read/modify/write updates; a crash cannot leave a partial state.
pub fn record_in(directory: &Path, unknown: bool) -> Result<u8> {
    update_in(directory, unknown).map(|reaction| reaction.level)
}

fn update_in(directory: &Path, unknown: bool) -> Result<Reaction> {
    ensure_external(directory)?;
    fs::create_dir_all(directory).context("cannot create Hannes state directory")?;
    ensure_external(directory)?;
    if let Ok(metadata) = fs::symlink_metadata(directory.join("patience.lock")) {
        ensure!(
            metadata.file_type().is_file(),
            "Hannes patience lock must be a regular file"
        );
    }
    let _lock = lock(directory)?;
    let path = directory.join("patience");
    if let Ok(metadata) = fs::symlink_metadata(&path) {
        ensure!(
            metadata.file_type().is_file(),
            "Hannes patience must be a regular file"
        );
    }
    let mut state = match File::open(&path) {
        Ok(file) => {
            let mut bytes = Vec::new();
            file.take(513).read_to_end(&mut bytes)?;
            if bytes.len() > 512 {
                State::default()
            } else {
                decode(&bytes)
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => State::default(),
        Err(error) => return Err(error).context("cannot read Hannes patience"),
    };
    if unknown {
        // A fresh error shows the current reaction. Only successive errors escalate.
        // A valid command's one-step recovery therefore remains visible next time.
        if state.previous_unknown {
            state.level = state.level.saturating_add(1).min(MAX_LEVEL);
        }
        state.screaming_streak = if state.level == MAX_LEVEL {
            state.screaming_streak.saturating_add(1).min(3)
        } else {
            0
        };
    } else {
        state.level = state.level.saturating_sub(1);
        state.screaming_streak = 0;
    }
    state.previous_unknown = unknown;
    // Choose and reserve a saying under the same lock as mood updates, so
    // concurrent invocations cannot independently pick the same recent quote.
    // Valid commands and the face-free farewell do not consume a saying.
    let leave = state.screaming_streak == 3;
    let quote_index = if unknown && !leave {
        let index = crate::quotes::choose(&state.recent_quotes);
        if state.recent_quotes.len() == crate::quotes::HISTORY_LIMIT {
            state.recent_quotes.pop_front();
        }
        state.recent_quotes.push_back(index);
        Some(index)
    } else {
        None
    };
    let mut temporary = tempfile::NamedTempFile::new_in(directory)?;
    serde_json::to_writer(&mut temporary, &state)?;
    writeln!(temporary)?;
    temporary.as_file().sync_all()?;
    temporary
        .persist(&path)
        .context("cannot save Hannes patience")?;
    Ok(Reaction {
        level: state.level,
        leave,
        quote_index,
    })
}

/// Never interpret a repository, .git or .Hannes directory as disposable state,
/// including when an override reaches it through a symlink.
fn ensure_external(directory: &Path) -> Result<()> {
    ensure!(
        directory.is_absolute(),
        "Hannes state directory must be absolute"
    );
    let protected = |path: &Path| {
        path.components().any(|part| {
            part.as_os_str().to_str().is_some_and(|name| {
                name.eq_ignore_ascii_case(".hannes") || name.eq_ignore_ascii_case(".git")
            })
        })
    };
    ensure!(
        !protected(directory),
        "Hannes state must be outside repositories and .Hannes"
    );
    let mut existing = directory;
    while !existing.try_exists()? {
        existing = existing
            .parent()
            .context("cannot resolve Hannes state directory")?;
    }
    let resolved = existing.canonicalize()?;
    ensure!(
        !protected(&resolved),
        "Hannes state must be outside repositories and .Hannes"
    );
    for ancestor in resolved.ancestors() {
        ensure!(
            !ancestor.join(".git").try_exists()?,
            "Hannes state must be outside repositories"
        );
    }
    Ok(())
}

/// Delete only Hannes-owned state files. Never recursively remove a directory.
pub(crate) fn remove_state() -> Result<()> {
    let directory = state_directory()?;
    ensure_external(&directory)?;
    if !directory.try_exists()? {
        return Ok(());
    }
    let directory = directory.canonicalize()?;
    ensure_external(&directory)?;
    let lock_path = directory.join("patience.lock");
    if let Ok(metadata) = fs::symlink_metadata(&lock_path) {
        ensure!(
            metadata.is_file() && !metadata.file_type().is_symlink(),
            "Hannes patience lock must be a regular file"
        );
    }
    let lock = lock(&directory)?;
    match fs::remove_file(directory.join("patience")) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error).context("cannot remove Hannes patience"),
    }
    // Windows requires the lock handle to close before its file can be removed.
    drop(lock);
    fs::remove_file(lock_path).context("cannot remove Hannes patience lock")?;
    // Foreign files are preserved. Remove the state directory only if now empty.
    if fs::read_dir(&directory)?.next().is_none() {
        fs::remove_dir(directory).context("cannot remove empty Hannes state directory")?;
    }
    Ok(())
}

#[cfg(unix)]
fn lock(directory: &Path) -> Result<File> {
    use std::os::fd::AsRawFd;
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(directory.join("patience.lock"))?;
    let start = Instant::now();
    loop {
        // The OS releases the lock when the file closes, including after a crash.
        if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } == 0 {
            return Ok(file);
        }
        let error = std::io::Error::last_os_error();
        if !matches!(
            error.kind(),
            std::io::ErrorKind::WouldBlock | std::io::ErrorKind::Interrupted
        ) {
            return Err(error.into());
        }
        if start.elapsed() >= Duration::from_secs(2) {
            bail!("Hannes patience state is busy");
        }
        thread::sleep(Duration::from_millis(10));
    }
}
#[cfg(windows)]
fn lock(directory: &Path) -> Result<File> {
    use std::os::windows::fs::OpenOptionsExt;
    let start = Instant::now();
    loop {
        match OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .share_mode(0)
            .open(directory.join("patience.lock"))
        {
            Ok(file) => return Ok(file),
            Err(error) if matches!(error.raw_os_error(), Some(32..=33)) => {
                if start.elapsed() >= Duration::from_secs(2) {
                    bail!("Hannes patience state is busy");
                }
                thread::sleep(Duration::from_millis(10));
            }
            Err(error) => return Err(error.into()),
        }
    }
}
#[cfg(not(any(unix, windows)))]
fn lock(directory: &Path) -> Result<File> {
    Ok(OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .open(directory.join("patience.lock"))?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_states_preserve_mood_and_edited_histories_keep_only_latest_valid_indices() {
        let old = decode(br#"{"level":3,"previous_unknown":true,"screaming_streak":1}"#);
        assert_eq!(old.level, 3);
        assert!(old.recent_quotes.is_empty());
        let mut indices: Vec<usize> = (0..30).collect();
        indices.extend([999, 20, 25]);
        let json = serde_json::json!({"level":3,"previous_unknown":true,"screaming_streak":1,"recent_quotes":indices});
        let normalized = decode(json.to_string().as_bytes());
        assert_eq!(normalized.level, 3);
        assert_eq!(normalized.screaming_streak, 1);
        assert_eq!(normalized.recent_quotes.len(), 20);
        assert_eq!(normalized.recent_quotes.back(), Some(&25));
        assert!(
            normalized
                .recent_quotes
                .iter()
                .all(|index| *index < crate::quotes::all().len())
        );
        assert_eq!(
            normalized
                .recent_quotes
                .iter()
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            20
        );
    }

    #[test]
    fn farewell_does_not_reserve_an_unshown_quote() {
        let directory = tempfile::tempdir().unwrap();
        for _ in 0..6 {
            assert!(!update_in(directory.path(), true).unwrap().leave);
        }
        let before = fs::read_to_string(directory.path().join("patience")).unwrap();
        let reaction = update_in(directory.path(), true).unwrap();
        assert!(reaction.leave && reaction.quote_index.is_none());
        let before: serde_json::Value = serde_json::from_str(&before).unwrap();
        let after: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(directory.path().join("patience")).unwrap())
                .unwrap();
        assert_eq!(before["recent_quotes"], after["recent_quotes"]);
    }
}
