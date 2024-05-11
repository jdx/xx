#[macro_use]
extern crate log;

pub use error::{XXError, XXResult};

pub mod context;
pub mod error;
pub mod file;
pub mod git;
pub mod process;
mod regex;

#[cfg(all(
    feature = "archive_untar_gzip",
    feature = "archive_untar_bzip2",
    feature = "archive_untar_xz",
    feature = "archive_unzip",
))]
pub mod archive;

#[cfg(test)]
pub mod test;
