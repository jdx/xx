use std::path::PathBuf;

#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
pub(crate) fn home_dir() -> Option<PathBuf> {
    homedir::my_home().ok().flatten()
}

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
pub(crate) fn home_dir() -> Option<PathBuf> {
    None
}
