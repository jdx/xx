use std::path::PathBuf;

use miette::Diagnostic;
use thiserror::Error;

#[derive(Error, Diagnostic, Debug)]
pub enum XXError {
    #[error("{0}\nFile: {1}")]
    #[diagnostic(code(xx::file), url(docsrs))]
    FileError(std::io::Error, PathBuf),

    #[error("{0}\nGit: {1}")]
    #[diagnostic(code(xx::git), url(docsrs))]
    GitError(std::io::Error, PathBuf),

    #[error("{0}\n{1}")]
    #[diagnostic(code(xx::process), url(docsrs))]
    ProcessError(std::io::Error, String),

    #[cfg(any(
        feature = "archive_untar_gzip",
        feature = "archive_untar_bzip2",
        feature = "archive_untar_xz",
        feature = "archive_unzip",
    ))]
    #[error("{0}\n{1}")]
    #[diagnostic(code(xx::archive), url(docsrs))]
    ArchiveIOError(std::io::Error, PathBuf),

    #[cfg(feature = "archive_unzip")]
    #[error("{0}\n{1}")]
    #[diagnostic(code(xx::archive), url(docsrs))]
    ArchiveZipError(zip::result::ZipError, PathBuf),

    #[cfg(feature = "glob")]
    #[error("{0}\n{1}")]
    #[diagnostic(code(xx::glob), url(docsrs))]
    GlobwalkError(globwalk::GlobError, PathBuf),
}

pub type XXResult<T> = Result<T, XXError>;
