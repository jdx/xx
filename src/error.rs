use std::path::PathBuf;

use miette::Diagnostic;
use thiserror::Error;

#[derive(Error, Diagnostic, Debug)]
pub enum XXError {
    #[error("{0}\nFile: {1}")]
    #[diagnostic(code(xxerr::file), url(docsrs))]
    FileError(std::io::Error, PathBuf),
}

pub type XXResult<T> = Result<T, XXError>;
