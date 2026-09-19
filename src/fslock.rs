use crate::error::XXResult;
use std::env;
use std::path::{Path, PathBuf};

use crate::file::display_path;
use crate::hash::hash_to_str;
use crate::{XXError, file};

pub type OnLockedFn = Box<dyn Fn(&Path)>;

#[derive(Debug)]
pub struct LockFile {
    #[cfg(unix)]
    fd: libc::c_int,
    #[cfg(not(unix))]
    inner: fslock::LockFile,
    locked: bool,
}

impl LockFile {
    #[cfg(unix)]
    fn open(path: &Path) -> XXResult<Self> {
        use std::os::unix::fs::PermissionsExt;
        use std::os::unix::io::IntoRawFd;

        let file = open_shared(path)
            .map_err(|e| XXError::FSLockError(e, format!("lockfile {}", path.display())))?;

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

        let fd = file.into_raw_fd();
        Ok(Self { fd, locked: false })
    }

    #[cfg(not(unix))]
    fn open(path: &Path) -> XXResult<Self> {
        let inner = fslock::LockFile::open(path)
            .map_err(|e| XXError::FSLockError(e, format!("lockfile {}", path.display())))?;
        Ok(Self {
            inner,
            locked: false,
        })
    }

    pub fn try_lock(&mut self) -> XXResult<bool> {
        if self.locked {
            panic!("Cannot lock if already owning a lock");
        }
        #[cfg(unix)]
        loop {
            let res = unsafe { libc::flock(self.fd, libc::LOCK_EX | libc::LOCK_NB) };
            if res >= 0 {
                self.locked = true;
                return Ok(true);
            }
            let err = std::io::Error::last_os_error();
            let code = err.raw_os_error().unwrap_or(0);
            if code == libc::EINTR {
                continue;
            }
            if code == libc::EWOULDBLOCK {
                return Ok(false);
            }
            return Err(XXError::FSLockError(err, "flock try_lock".into()));
        }
        #[cfg(not(unix))]
        {
            let locked = self
                .inner
                .try_lock()
                .map_err(|e| XXError::FSLockError(e, "fslock try_lock".into()))?;
            if locked {
                self.locked = true;
            }
            Ok(locked)
        }
    }

    pub fn lock(&mut self) -> XXResult<()> {
        if self.locked {
            panic!("Cannot lock if already owning a lock");
        }
        #[cfg(unix)]
        loop {
            let res = unsafe { libc::flock(self.fd, libc::LOCK_EX) };
            if res >= 0 {
                self.locked = true;
                return Ok(());
            }
            let err = std::io::Error::last_os_error();
            let code = err.raw_os_error().unwrap_or(0);
            if code == libc::EINTR {
                continue;
            }
            return Err(XXError::FSLockError(err, "flock lock".into()));
        }
        #[cfg(not(unix))]
        {
            self.inner
                .lock()
                .map_err(|e| XXError::FSLockError(e, "fslock lock".into()))?;
            self.locked = true;
            Ok(())
        }
    }

    pub fn unlock(&mut self) -> XXResult<()> {
        if !self.locked {
            panic!("Attempted to unlock already unlocked lockfile");
        }
        #[cfg(unix)]
        {
            let res = unsafe { libc::flock(self.fd, libc::LOCK_UN) };
            if res < 0 {
                return Err(XXError::FSLockError(
                    std::io::Error::last_os_error(),
                    "flock unlock".into(),
                ));
            }
            self.locked = false;
            if unsafe { libc::ftruncate(self.fd, 0) } < 0 {
                debug!("failed to truncate lockfile after unlock");
            }
        }
        #[cfg(not(unix))]
        {
            self.inner
                .unlock()
                .map_err(|e| XXError::FSLockError(e, "fslock unlock".into()))?;
            self.locked = false;
        }
        Ok(())
    }
}

impl Drop for LockFile {
    fn drop(&mut self) {
        if self.locked
            && let Err(e) = self.unlock()
        {
            debug!("failed to unlock lockfile in Drop: {e}");
        }
        #[cfg(unix)]
        unsafe {
            libc::close(self.fd);
        }
    }
}

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

    pub fn lock(self) -> XXResult<LockFile> {
        #[cfg(unix)]
        verify_no_symlink_ancestors(&self.path)?;
        if let Some(parent) = self.path.parent() {
            file::mkdirp(parent)?;
            #[cfg(unix)]
            share_lock_dir(parent);
        }
        let mut lock = LockFile::open(&self.path)?;
        if !lock.try_lock()? {
            if let Some(f) = self.on_locked {
                f(&self.path)
            }
            lock.lock()?;
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

/// Opens a lock file that other users may have created.
///
/// Opens an existing file without `O_CREAT`: with `fs.protected_regular`
/// enabled, Linux rejects `O_CREAT` on another user's file in a
/// world-writable sticky directory, even when its mode allows the open.
#[cfg(unix)]
fn open_shared(path: &Path) -> std::io::Result<std::fs::File> {
    use std::io::ErrorKind;
    use std::os::unix::fs::OpenOptionsExt;

    let open = |create: bool| {
        std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(create)
            .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW)
            .open(path)
    };
    match open(false) {
        Err(e) if e.kind() == ErrorKind::NotFound => match open(true) {
            Err(e) if e.kind() == ErrorKind::AlreadyExists => open(false),
            res => res,
        },
        res => res,
    }
}

/// Makes the shared lock directory writable by every user, like `/tmp`.
///
/// Lock files are keyed by the locked path, so a second user usually needs a
/// file the first user never created. Only the directory's owner or root can
/// change its mode; for anyone else this is a no-op. The directory is opened
/// with `O_NOFOLLOW` and changed through its descriptor so a symlink swapped
/// in after `mkdirp` cannot redirect the change to another directory.
#[cfg(unix)]
fn share_lock_dir(dir: &Path) {
    use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

    let set = || -> std::io::Result<()> {
        let dir = std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_CLOEXEC | libc::O_DIRECTORY | libc::O_NOFOLLOW)
            .open(dir)?;
        let mode = dir.metadata()?.permissions().mode();
        if mode & 0o1777 != 0o1777 {
            dir.set_permissions(std::fs::Permissions::from_mode(mode | 0o1777))?;
        }
        Ok(())
    };
    if let Err(e) = set() {
        debug!("failed to share lock directory {}: {e}", dir.display());
    }
}

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

pub fn get(path: &Path, force: bool) -> XXResult<Option<LockFile>> {
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

#[cfg(all(test, unix))]
mod tests {
    use std::os::unix::fs::PermissionsExt;

    use test_log::test;

    use crate::test;

    use super::*;

    fn mode(path: &Path) -> u32 {
        std::fs::symlink_metadata(path)
            .unwrap()
            .permissions()
            .mode()
            & 0o7777
    }

    #[test]
    fn share_lock_dir_lets_every_user_create_locks() {
        let tmp = test::tempdir();
        let dir = tmp.path().join("fslock");
        std::fs::create_dir(&dir).unwrap();
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755)).unwrap();

        share_lock_dir(&dir);

        assert_eq!(mode(&dir), 0o1777);
    }

    #[test]
    fn share_lock_dir_does_not_follow_symlinks() {
        let tmp = test::tempdir();
        let target = tmp.path().join("private");
        std::fs::create_dir(&target).unwrap();
        std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o700)).unwrap();
        let link = tmp.path().join("fslock");
        std::os::unix::fs::symlink(&target, &link).unwrap();

        share_lock_dir(&link);

        assert_eq!(mode(&target), 0o700);
    }

    #[test]
    fn open_shared_creates_then_reuses_lock_file() {
        let tmp = test::tempdir();
        let path = tmp.path().join("lock");

        let mut first = LockFile::open(&path).unwrap();
        assert!(first.try_lock().unwrap());
        assert_eq!(mode(&path), 0o666);

        let mut second = LockFile::open(&path).unwrap();
        assert!(!second.try_lock().unwrap());
        drop(first);
        assert!(second.try_lock().unwrap());
    }

    #[test]
    fn open_shared_does_not_follow_symlinks() {
        let tmp = test::tempdir();
        let target = tmp.path().join("target");
        std::fs::write(&target, "").unwrap();
        let link = tmp.path().join("lock");
        std::os::unix::fs::symlink(&target, &link).unwrap();

        assert!(open_shared(&link).is_err());
    }
}
