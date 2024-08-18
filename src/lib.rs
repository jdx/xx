#![allow(unused_attributes)]

#[macro_use]
extern crate log;

#[macro_use]
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

#[cfg(feature = "hash")]
pub mod hash;
#[cfg(feature = "http")]
pub mod http;

#[cfg(test)]
pub mod test;
