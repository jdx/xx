use crate::error::XXResult;
use std::env;
use std::path::{Path, PathBuf};

use crate::file::display_path;
use crate::hash::hash_to_str;
use crate::{XXError, file};

pub type OnLockedFn = Box<dyn Fn(&Path)>;

pub struct FSLock {
    path: PathBuf,
    on_locked: Option<OnLockedFn>,
}

impl FSLock {
    pub fn new(path: &Path) -> Self {
        let normalized = normalize_path(path);
        Self {
            path: env::temp_dir()
                .join("fslock")
                .join(hash_to_str(&normalized)),
            on_locked: None,
        }
    }

    pub fn with_callback<F>(mut self, cb: F) -> Self
    where
        F: Fn(&Path) + 'static,
    {
        self.on_locked = Some(Box::new(cb));
        self
    }

    pub fn lock(self) -> XXResult<fslock::LockFile> {
        if let Some(parent) = self.path.parent() {
            file::mkdirp(parent)?;
            #[cfg(unix)]
            set_sticky_dir(parent);
        }
        #[cfg(unix)]
        ensure_shared_lockfile(&self.path);
        let mut lock = open_lockfile(&self.path)?;
        if !lock
            .try_lock()
            .map_err(|e| XXError::FSLockError(e, format!("lockfile {}", self.path.display())))?
        {
            if let Some(f) = self.on_locked {
                f(&self.path)
            }
            lock.lock().map_err(|e| {
                XXError::FSLockError(e, format!("lockfile {}", self.path.display()))
            })?;
        }
        Ok(lock)
    }
}

fn normalize_path(path: &Path) -> PathBuf {
    if let Ok(canonical) = path.canonicalize() {
        return canonical;
    }
    if let Ok(absolute) = std::path::absolute(path) {
        return absolute;
    }
    path.to_path_buf()
}

/// Set directory permissions to 0o1777 (sticky bit, like /tmp itself).
/// Ensures all users can create lock files regardless of who created the directory.
#[cfg(unix)]
fn set_sticky_dir(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let mode = std::fs::metadata(path).ok().and_then(|m| {
        let mode = m.permissions().mode();
        (mode & 0o1777 != 0o1777).then_some(0o1777)
    });
    if let Some(mode) = mode {
        if let Err(e) = std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)) {
            debug!("failed to set sticky bit on {}: {e}", path.display());
        }
    }
}

/// Pre-create the lock file with shared permissions (0o666) before LockFile::open.
/// LockFile::open uses O_CREAT which respects umask -- root with umask 0o077 would
/// create the file as 0o600, blocking non-root users. By pre-creating the file,
/// LockFile::open finds an existing file and skips creation with umask.
#[cfg(unix)]
fn ensure_shared_lockfile(path: &Path) {
    use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

    let newly_created = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o666)
        .open(path)
        .is_ok();

    let needs_chmod = if newly_created {
        // We created the file exclusively via O_EXCL -- no other process could have
        // opened it yet. Immediately fix permissions since umask may have restricted
        // them. The race window is limited to our own create+chmod sequence.
        Some(0o666)
    } else {
        // File already exists -- ensure shared permissions.
        // Fixes files created by root with restrictive umask in prior runs.
        std::fs::metadata(path).ok().and_then(|m| {
            let mode = m.permissions().mode();
            (mode & 0o066 != 0o066).then_some(0o666)
        })
    };

    if let Some(mode) = needs_chmod {
        if let Err(e) = std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)) {
            debug!(
                "failed to set lockfile permissions on {}: {e}",
                path.display()
            );
        }
    }
}

/// Open the lock file with retry on PermissionDenied.
/// Handles the remaining race where another process just created the file
/// with umask-restricted permissions and hasn't chmod'd it yet.
fn open_lockfile(path: &Path) -> XXResult<fslock::LockFile> {
    let mut attempts = 0u32;
    loop {
        match fslock::LockFile::open(path) {
            Ok(lock) => return Ok(lock),
            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied && attempts < 3 => {
                attempts += 1;
                debug!(
                    "permission denied opening lockfile {}, retrying ({}/3)",
                    path.display(),
                    attempts
                );
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            Err(e) => {
                return Err(XXError::FSLockError(
                    e,
                    format!("lockfile {}", path.display()),
                ));
            }
        }
    }
}

pub fn get(path: &Path, force: bool) -> XXResult<Option<fslock::LockFile>> {
    let lock = if force {
        None
    } else {
        let lock = FSLock::new(path)
            .with_callback(|l| {
                debug!("waiting for lock on {}", display_path(l));
            })
            .lock()?;
        Some(lock)
    };
    Ok(lock)
}
