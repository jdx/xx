use std::path::PathBuf;

use miette::Diagnostic;
use thiserror::Error;

#[derive(Error, Diagnostic, Debug)]
pub enum XXError {
    #[error("{0}\nFile: {1}")]
    #[diagnostic(code(xxerr::file), url(docsrs))]
    FileError(std::io::Error, PathBuf),

    #[error("{0}\n{1}")]
    #[diagnostic(code(xxerr::process), url(docsrs))]
    ProcessError(std::io::Error, String),
}

pub type XXResult<T> = Result<T, XXError>;
