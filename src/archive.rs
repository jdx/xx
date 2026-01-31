//! Archive file handling functions.
//!
//! This module provides functions for extracting and inspecting archive files.
//!
//! ## Features
//!
//! - `archive_untar_gzip`: Extract .tar.gz files
//! - `archive_untar_bzip2`: Extract .tar.bz2 files
//! - `archive_untar_xz`: Extract .tar.xz files
//! - `archive_unzip`: Extract .zip files
//! - `archive_ungz`: Decompress .gz files
//!
//! ## Examples
//!
//! ```rust,no_run
//! use xx::archive;
//! use std::path::Path;
//!
//! // Extract a tar.gz archive
//! archive::untar_gz(Path::new("archive.tar.gz"), Path::new("/tmp/dest")).unwrap();
//!
//! // List contents of a zip file
//! let entries = archive::list_zip(Path::new("archive.zip")).unwrap();
//! for entry in entries {
//!     println!("{}", entry.path);
//! }
//! ```

use std::path::Path;

use crate::{XXError, XXResult, file};

/// Unpack a .tar.gz archive to a destination directory.
#[cfg(feature = "archive_untar_gzip")]
pub fn untar_gz(archive: &Path, destination: &Path) -> XXResult<()> {
    let file = file::open(archive)?;
    let mut a = tar::Archive::new(flate2::read::GzDecoder::new(file));
    a.unpack(destination)
        .map_err(|err| XXError::ArchiveIOError(err, archive.to_path_buf()))?;
    Ok(())
}

#[cfg(feature = "archive_ungz")]
pub fn ungz(archive: &Path, destination: &Path) -> XXResult<()> {
    let file = file::open(archive)?;
    let mut decoder = flate2::read::GzDecoder::new(file);

    if let Some(parent) = destination.parent() {
        file::mkdirp(parent)?;
    }
    let mut output_file = file::create(destination)?;

    std::io::copy(&mut decoder, &mut output_file)
        .map_err(|err| XXError::ArchiveIOError(err, archive.to_path_buf()))?;

    Ok(())
}

/// Unpack a .tar.bz2 archive to a destination directory.
#[cfg(feature = "archive_untar_bzip2")]
pub fn untar_bz2(archive: &Path, destination: &Path) -> XXResult<()> {
    let file = file::open(archive)?;
    let mut a = tar::Archive::new(bzip2::read::BzDecoder::new(file));
    a.unpack(destination)
        .map_err(|err| XXError::ArchiveIOError(err, archive.to_path_buf()))?;
    Ok(())
}

/// Unpack a .tar.xz archive to a destination directory.
#[cfg(feature = "archive_untar_xz")]
pub fn untar_xz(archive: &Path, destination: &Path) -> XXResult<()> {
    let file = file::open(archive)?;
    let mut a = tar::Archive::new(xz2::read::XzDecoder::new(file));
    a.unpack(destination)
        .map_err(|err| XXError::ArchiveIOError(err, archive.to_path_buf()))?;
    Ok(())
}

/// Unzip a zip archive to a destination directory.
#[cfg(feature = "archive_unzip")]
pub fn unzip(archive: &Path, destination: &Path) -> XXResult<()> {
    let file = file::open(archive)?;
    let mut a = zip::ZipArchive::new(file)
        .map_err(|err| XXError::ArchiveZipError(err, archive.to_path_buf()))?;
    for i in 0..a.len() {
        let mut file = a
            .by_index(i)
            .map_err(|err| XXError::ArchiveZipError(err, archive.to_path_buf()))?;
        let outpath = destination.join(file.name());
        if file.is_dir() {
            file::mkdirp(&outpath)?;
        } else {
            if let Some(p) = outpath.parent() {
                file::mkdirp(p)?;
            }
            let mut outfile = file::create(&outpath)?;
            std::io::copy(&mut file, &mut outfile)
                .map_err(|err| XXError::ArchiveIOError(err, outpath.to_path_buf()))?;

            #[cfg(unix)]
            if let Some(mode) = file.unix_mode() {
                file::chmod(&outpath, mode)?;
            }
        }
    }
    Ok(())
}

/// Information about an archive entry
#[derive(Debug, Clone)]
pub struct ArchiveEntry {
    /// Path within the archive
    pub path: String,
    /// Size in bytes (uncompressed)
    pub size: u64,
    /// Whether this is a directory
    pub is_dir: bool,
    /// Whether this is a symbolic link
    pub is_symlink: bool,
    /// Unix mode if available
    #[cfg(unix)]
    pub mode: Option<u32>,
}

/// List contents of a tar.gz archive without extracting
#[cfg(feature = "archive_untar_gzip")]
pub fn list_tar_gz(archive: &Path) -> XXResult<Vec<ArchiveEntry>> {
    let file = file::open(archive)?;
    let decoder = flate2::read::GzDecoder::new(file);
    list_tar_inner(decoder, archive)
}

/// List contents of a tar.bz2 archive without extracting
#[cfg(feature = "archive_untar_bzip2")]
pub fn list_tar_bz2(archive: &Path) -> XXResult<Vec<ArchiveEntry>> {
    let file = file::open(archive)?;
    let decoder = bzip2::read::BzDecoder::new(file);
    list_tar_inner(decoder, archive)
}

/// List contents of a tar.xz archive without extracting
#[cfg(feature = "archive_untar_xz")]
pub fn list_tar_xz(archive: &Path) -> XXResult<Vec<ArchiveEntry>> {
    let file = file::open(archive)?;
    let decoder = xz2::read::XzDecoder::new(file);
    list_tar_inner(decoder, archive)
}

#[cfg(any(
    feature = "archive_untar_gzip",
    feature = "archive_untar_bzip2",
    feature = "archive_untar_xz"
))]
fn list_tar_inner<R: std::io::Read>(reader: R, archive: &Path) -> XXResult<Vec<ArchiveEntry>> {
    let mut a = tar::Archive::new(reader);
    let mut entries = Vec::new();

    for entry in a
        .entries()
        .map_err(|err| XXError::ArchiveIOError(err, archive.to_path_buf()))?
    {
        let entry = entry.map_err(|err| XXError::ArchiveIOError(err, archive.to_path_buf()))?;
        let header = entry.header();

        let path = entry
            .path()
            .map_err(|err| XXError::ArchiveIOError(err, archive.to_path_buf()))?
            .to_string_lossy()
            .to_string();

        entries.push(ArchiveEntry {
            path,
            size: header.size().unwrap_or(0),
            is_dir: header.entry_type().is_dir(),
            is_symlink: header.entry_type().is_symlink(),
            #[cfg(unix)]
            mode: header.mode().ok(),
        });
    }

    Ok(entries)
}

/// List contents of a zip archive without extracting
#[cfg(feature = "archive_unzip")]
pub fn list_zip(archive: &Path) -> XXResult<Vec<ArchiveEntry>> {
    let file = file::open(archive)?;
    let mut a = zip::ZipArchive::new(file)
        .map_err(|err| XXError::ArchiveZipError(err, archive.to_path_buf()))?;

    let mut entries = Vec::new();

    for i in 0..a.len() {
        let file = a
            .by_index(i)
            .map_err(|err| XXError::ArchiveZipError(err, archive.to_path_buf()))?;

        entries.push(ArchiveEntry {
            path: file.name().to_string(),
            size: file.size(),
            is_dir: file.is_dir(),
            is_symlink: file.is_symlink(),
            #[cfg(unix)]
            mode: file.unix_mode(),
        });
    }

    Ok(entries)
}

/// Check if an archive contains a single top-level directory
///
/// This is useful for detecting archives that need component stripping during extraction.
#[cfg(any(
    feature = "archive_untar_gzip",
    feature = "archive_untar_bzip2",
    feature = "archive_untar_xz",
    feature = "archive_unzip"
))]
pub fn has_single_root_dir(entries: &[ArchiveEntry]) -> Option<String> {
    let mut roots = std::collections::HashSet::new();

    for entry in entries {
        // Get the first path component
        if let Some(root) = entry.path.split('/').next() {
            if !root.is_empty() {
                roots.insert(root.to_string());
            }
        }
    }

    if roots.len() == 1 {
        roots.into_iter().next()
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use test_log::test;

    use super::*;

    #[cfg(feature = "archive_untar_gzip")]
    #[test]
    fn test_untar_gz() {
        let archive = Path::new(env!("CARGO_MANIFEST_DIR")).join("test/data/foo.tar.gz");
        let tmpdir = tempfile::tempdir().unwrap();
        let destination = tmpdir.path();
        untar_gz(&archive, destination).unwrap();
        assert!(destination.exists());
        assert!(destination.join("foo/test.txt").exists());
        assert_eq!(
            fs::read_to_string(destination.join("foo/test.txt")).unwrap(),
            "yep\n"
        );
        // tmpdir cleanup on drop
    }

    #[cfg(feature = "archive_untar_bzip2")]
    #[test]
    fn test_untar_bz2() {
        let archive = Path::new(env!("CARGO_MANIFEST_DIR")).join("test/data/foo.tar.bz2");
        let tmpdir = tempfile::tempdir().unwrap();
        let destination = tmpdir.path();
        untar_bz2(&archive, destination).unwrap();
        assert!(destination.exists());
        assert!(destination.join("foo/test.txt").exists());
        assert_eq!(
            fs::read_to_string(destination.join("foo/test.txt")).unwrap(),
            "yep\n"
        );
        // tmpdir cleanup on drop
    }

    #[cfg(feature = "archive_untar_xz")]
    #[test]
    fn test_untar_xz() {
        let archive = Path::new(env!("CARGO_MANIFEST_DIR")).join("test/data/foo.tar.xz");
        let tmpdir = tempfile::tempdir().unwrap();
        let destination = tmpdir.path();
        untar_xz(&archive, destination).unwrap();
        assert!(destination.exists());
        assert!(destination.join("foo/test.txt").exists());
        assert_eq!(
            fs::read_to_string(destination.join("foo/test.txt")).unwrap(),
            "yep\n"
        );
        // tmpdir cleanup on drop
    }

    #[cfg(feature = "archive_unzip")]
    #[test]
    fn test_unzip() {
        let archive = Path::new(env!("CARGO_MANIFEST_DIR")).join("test/data/foo.zip");
        let tmpdir = tempfile::tempdir().unwrap();
        let destination = tmpdir.path();
        unzip(&archive, destination).unwrap();
        assert!(destination.exists());
        assert!(destination.join("foo/test.txt").exists());
        assert_eq!(
            fs::read_to_string(destination.join("foo/test.txt")).unwrap(),
            "yep\n"
        );
        // tmpdir cleanup on drop
    }

    #[cfg(feature = "archive_ungz")]
    #[test]
    fn test_ungz() {
        use flate2::Compression;
        use flate2::write::GzEncoder;
        use std::io::Write;

        // Create a temporary gzip file for testing
        let test_content = "Hello, gzip world!\n";
        let tmpdir = std::env::temp_dir();
        let archive_path = tmpdir.join("test_ungz.gz");
        let destination_path = tmpdir.join("test_ungz_output.txt");

        // Create the gzip file
        {
            let file = fs::File::create(&archive_path).unwrap();
            let mut encoder = GzEncoder::new(file, Compression::default());
            encoder.write_all(test_content.as_bytes()).unwrap();
            encoder.finish().unwrap();
        }

        // Test the ungz function
        ungz(&archive_path, &destination_path).unwrap();
        assert!(destination_path.exists());
        assert_eq!(fs::read_to_string(&destination_path).unwrap(), test_content);

        // Clean up
        fs::remove_file(&archive_path).unwrap();
        fs::remove_file(&destination_path).unwrap();
    }

    #[cfg(feature = "archive_untar_gzip")]
    #[test]
    fn test_list_tar_gz() {
        let archive = Path::new(env!("CARGO_MANIFEST_DIR")).join("test/data/foo.tar.gz");
        let entries = list_tar_gz(&archive).unwrap();
        assert!(!entries.is_empty());
        // Should contain the test file
        assert!(entries.iter().any(|e| e.path.contains("test.txt")));
    }

    #[cfg(feature = "archive_unzip")]
    #[test]
    fn test_list_zip() {
        let archive = Path::new(env!("CARGO_MANIFEST_DIR")).join("test/data/foo.zip");
        let entries = list_zip(&archive).unwrap();
        assert!(!entries.is_empty());
        // Should contain the test file
        assert!(entries.iter().any(|e| e.path.contains("test.txt")));
    }

    #[cfg(feature = "archive_untar_gzip")]
    #[test]
    fn test_has_single_root_dir() {
        let archive = Path::new(env!("CARGO_MANIFEST_DIR")).join("test/data/foo.tar.gz");
        let entries = list_tar_gz(&archive).unwrap();
        let root = has_single_root_dir(&entries);
        assert!(root.is_some());
        assert_eq!(root.unwrap(), "foo");
    }
}
