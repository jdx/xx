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
        #[cfg(unix)]
        verify_no_symlink_ancestors(&self.path)?;
        if let Some(parent) = self.path.parent() {
            file::mkdirp(parent)?;
        }
        #[cfg(unix)]
        ensure_shared_lockfile(&self.path)?;
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

/// Verify that the immediate parent directory of the lockfile path
/// is not a symlink, preventing symlink-based redirection attacks.
/// Only checks the fslock directory itself, not ancestors like /tmp
/// which may legitimately be symlinks on some platforms (e.g. macOS).
#[cfg(unix)]
fn verify_no_symlink_ancestors(path: &Path) -> XXResult<()> {
    if let Some(parent) = path.parent()
        && parent.is_symlink()
    {
        return Err(XXError::FSLockError(
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("parent is a symlink: {}", parent.display()),
            ),
            format!("lockfile {}", path.display()),
        ));
    }
    Ok(())
}

/// Pre-create the lock file with shared permissions (0o666) using O_NOFOLLOW
/// to prevent symlink redirection. Uses fd-based set_permissions to avoid
/// TOCTOU races with path-based chmod.
#[cfg(unix)]
fn ensure_shared_lockfile(path: &Path) -> XXResult<()> {
    use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

    let mut opts = std::fs::OpenOptions::new();
    opts.write(true).create_new(true).mode(0o666);
    opts.custom_flags(libc::O_NOFOLLOW);

    match opts.open(path) {
        Ok(file) => {
            file.set_permissions(std::fs::Permissions::from_mode(0o666))
                .unwrap_or_else(|e| {
                    debug!(
                        "failed to set lockfile permissions on {}: {e}",
                        path.display()
                    );
                });
        }
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            let mut opts = std::fs::OpenOptions::new();
            opts.read(true).write(true).custom_flags(libc::O_NOFOLLOW);
            match opts.open(path) {
                Ok(file) => {
                    let mode = file.metadata().map(|m| m.permissions().mode()).unwrap_or(0);
                    if mode & 0o666 != 0o666 {
                        file.set_permissions(std::fs::Permissions::from_mode(0o666))
                            .unwrap_or_else(|e| {
                                debug!(
                                    "failed to set lockfile permissions on {}: {e}",
                                    path.display()
                                );
                            });
                    }
                }
                Err(e) if e.raw_os_error() == Some(libc::ELOOP) => {
                    return Err(XXError::FSLockError(
                        e,
                        format!("lockfile {}", path.display()),
                    ));
                }
                Err(e) => {
                    return Err(XXError::FSLockError(
                        e,
                        format!("lockfile {}", path.display()),
                    ));
                }
            }
        }
        Err(e) if e.raw_os_error() == Some(libc::ELOOP) => {
            return Err(XXError::FSLockError(
                e,
                format!("lockfile {}", path.display()),
            ));
        }
        Err(e) => {
            return Err(XXError::FSLockError(
                e,
                format!("lockfile {}", path.display()),
            ));
        }
    }
    Ok(())
}

fn open_lockfile(path: &Path) -> XXResult<fslock::LockFile> {
    fslock::LockFile::open(path)
        .map_err(|e| XXError::FSLockError(e, format!("lockfile {}", path.display())))
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
