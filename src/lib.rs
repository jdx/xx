#[macro_use]
extern crate log;

pub use error::{XXError, XXResult};

pub mod context;
pub mod error;
pub mod file;

#[cfg(test)]
pub mod test;
