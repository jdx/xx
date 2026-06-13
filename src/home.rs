use std::path::PathBuf;

#[cfg(not(target_family = "wasm"))]
pub(crate) fn home_dir() -> Option<PathBuf> {
    homedir::my_home().ok().flatten()
}

#[cfg(target_family = "wasm")]
pub(crate) fn home_dir() -> Option<PathBuf> {
    None
}
