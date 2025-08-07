#![allow(unused_attributes)]

#[macro_use]
extern crate log;

#[macro_use]
pub use error::{XXError, XXResult};

pub mod context;
pub mod error;
pub mod file;
#[cfg(feature = "fslock")]
pub mod fslock;
pub mod git;
pub mod process;
mod regex;

#[cfg(any(
    feature = "archive_untar_gzip",
    feature = "archive_untar_bzip2",
    feature = "archive_untar_xz",
    feature = "archive_unzip",
    feature = "archive_ungz",
))]
pub mod archive;

#[cfg(feature = "hash")]
pub mod hash;
#[cfg(feature = "http")]
pub mod http;

#[cfg(test)]
pub mod test;
