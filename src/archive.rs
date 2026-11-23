//! Archive file handling functions.
//!
//! This module provides functions for creating, extracting, and inspecting archive files.
//!
//! ## Extraction Features
//!
//! - `archive_untar_gzip`: Extract .tar.gz files
//! - `archive_untar_bzip2`: Extract .tar.bz2 files
//! - `archive_untar_xz`: Extract .tar.xz files
//! - `archive_unzip`: Extract .zip files
//! - `archive_ungz`: Decompress .gz files
//!
//! ## Creation Features
//!
//! - `archive_tar_gzip`: Create .tar.gz files
//! - `archive_tar_bzip2`: Create .tar.bz2 files
//! - `archive_tar_xz`: Create .tar.xz files
//! - `archive_zip`: Create .zip files
//!
//! ## Examples
//!
//! ### Extracting archives
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
//!
//! ### Creating archives
//!
//! ```rust,no_run
//! use xx::archive;
//! use std::path::Path;
//!
//! // Create a tar.gz archive from a directory
//! archive::tar_gz(Path::new("/path/to/dir"), Path::new("archive.tar.gz")).unwrap();
//!
//! // Create a zip archive from a directory
//! archive::zip(Path::new("my-project/"), Path::new("archive.zip")).unwrap();
//!
//! // Create a zip archive from multiple paths
//! archive::zip_multi(&[Path::new("file1.txt"), Path::new("dir/")], Path::new("multi.zip")).unwrap();
//! ```

use std::path::Path;

use crate::{XXError, XXResult, file};

/// Trait for compression writers that need explicit finishing
#[cfg(any(
    feature = "archive_tar_gzip",
    feature = "archive_tar_bzip2",
    feature = "archive_tar_xz"
))]
trait FinishableWriter: std::io::Write {
    fn finish(self) -> std::io::Result<()>;
}

#[cfg(feature = "archive_tar_gzip")]
impl<W: std::io::Write> FinishableWriter for flate2::write::GzEncoder<W> {
    fn finish(self) -> std::io::Result<()> {
        flate2::write::GzEncoder::finish(self).map(|_| ())
    }
}

#[cfg(feature = "archive_tar_bzip2")]
impl<W: std::io::Write> FinishableWriter for bzip2::write::BzEncoder<W> {
    fn finish(self) -> std::io::Result<()> {
        bzip2::write::BzEncoder::finish(self).map(|_| ())
    }
}

#[cfg(feature = "archive_tar_xz")]
impl<W: std::io::Write> FinishableWriter for xz2::write::XzEncoder<W> {
    fn finish(self) -> std::io::Result<()> {
        xz2::write::XzEncoder::finish(self).map(|_| ())
    }
}

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

/// Unzip a ZIP archive to a destination directory.
///
/// Entry paths must stay below the destination. Existing symbolic links in
/// extraction paths are not followed. The caller must control the destination's
/// ancestors and prevent concurrent changes to the extraction tree.
#[cfg(feature = "archive_unzip")]
pub fn unzip(archive: &Path, destination: &Path) -> XXResult<()> {
    unzip_with_entry_names(archive, destination, |file| file.enclosed_name())
}

#[cfg(feature = "archive_unzip")]
fn unzip_with_entry_names(
    archive: &Path,
    destination: &Path,
    entry_name: impl Fn(&zip::read::ZipFile<'_, std::fs::File>) -> Option<std::path::PathBuf>,
) -> XXResult<()> {
    let file = file::open(archive)?;
    let mut a = zip::ZipArchive::new(file)
        .map_err(|err| XXError::ArchiveZipError(err, archive.to_path_buf()))?;
    // Check the same path we will use for extraction. A trailing separator
    // or dot component would otherwise make symlink_metadata follow a link.
    let output_root: std::path::PathBuf = destination.components().collect();
    check_zip_output_component(&output_root, false)
        .map_err(|err| XXError::ArchiveIOError(err, output_root.clone()))?;
    for i in 0..a.len() {
        let mut file = a
            .by_index(i)
            .map_err(|err| XXError::ArchiveZipError(err, archive.to_path_buf()))?;
        let name = entry_name(&file).ok_or_else(|| {
            XXError::ArchiveZipError(
                zip::result::ZipError::InvalidArchive("invalid extraction path".into()),
                archive.to_path_buf(),
            )
        })?;
        let outpath = zip_output_path(&output_root, &name, file.is_dir())
            .map_err(|err| XXError::ArchiveIOError(err, output_root.clone()))?;
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

#[cfg(feature = "archive_unzip")]
fn zip_output_path(
    destination: &Path,
    name: &Path,
    is_dir: bool,
) -> std::io::Result<std::path::PathBuf> {
    use std::io::ErrorKind;
    use std::path::Component;

    let mut outpath = destination.to_path_buf();
    let mut components = name.components().peekable();
    while let Some(component) = components.next() {
        match component {
            Component::CurDir => continue,
            Component::Normal(segment) => outpath.push(segment),
            _ => {
                return Err(std::io::Error::new(
                    ErrorKind::InvalidData,
                    "invalid ZIP extraction path",
                ));
            }
        }
        check_zip_output_component(&outpath, !is_dir && components.peek().is_none())?;
    }
    if !is_dir && outpath == destination {
        return Err(std::io::Error::new(
            ErrorKind::InvalidData,
            "ZIP file entry has no filename",
        ));
    }
    Ok(outpath)
}

#[cfg(feature = "archive_unzip")]
fn check_zip_output_component(path: &Path, _overwrite_file: bool) -> std::io::Result<()> {
    use std::io::ErrorKind;

    let metadata = match file::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(()),
        Err(err) => return Err(err),
    };
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
        if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return Err(std::io::Error::new(
                ErrorKind::InvalidData,
                "ZIP extraction path contains a reparse point",
            ));
        }
    }
    if metadata.file_type().is_symlink() {
        return Err(std::io::Error::new(
            ErrorKind::InvalidData,
            "ZIP extraction path contains a symbolic link",
        ));
    }
    #[cfg(unix)]
    if _overwrite_file && metadata.is_file() {
        use std::os::unix::fs::MetadataExt;
        if metadata.nlink() > 1 {
            return Err(std::io::Error::new(
                ErrorKind::InvalidData,
                "ZIP extraction would overwrite a hard-linked file",
            ));
        }
    }
    Ok(())
}

// ============================================================================
// Archive Creation Functions
// ============================================================================

/// Create a .tar.gz archive from a source path.
///
/// If the source is a directory, all its contents will be included.
/// If the source is a file, only that file will be included.
///
/// # Arguments
///
/// * `source` - The file or directory to archive
/// * `archive` - The path for the output archive file
///
/// # Example
///
/// ```rust,no_run
/// use xx::archive;
/// use std::path::Path;
///
/// // Create archive from a directory
/// archive::tar_gz(Path::new("my-project/"), Path::new("my-project.tar.gz")).unwrap();
/// ```
#[cfg(feature = "archive_tar_gzip")]
pub fn tar_gz(source: &Path, archive: &Path) -> XXResult<()> {
    if let Some(parent) = archive.parent() {
        file::mkdirp(parent)?;
    }
    let file = file::create(archive)?;
    let encoder = flate2::write::GzEncoder::new(file, flate2::Compression::default());
    create_tar_inner(source, encoder, archive)
}

/// Create a .tar.gz archive from multiple source paths.
///
/// # Arguments
///
/// * `sources` - The files and/or directories to archive
/// * `archive` - The path for the output archive file
///
/// # Example
///
/// ```rust,no_run
/// use xx::archive;
/// use std::path::Path;
///
/// archive::tar_gz_multi(
///     &[Path::new("src/"), Path::new("Cargo.toml")],
///     Path::new("project.tar.gz")
/// ).unwrap();
/// ```
#[cfg(feature = "archive_tar_gzip")]
pub fn tar_gz_multi(sources: &[&Path], archive: &Path) -> XXResult<()> {
    if let Some(parent) = archive.parent() {
        file::mkdirp(parent)?;
    }
    let file = file::create(archive)?;
    let encoder = flate2::write::GzEncoder::new(file, flate2::Compression::default());
    create_tar_multi_inner(sources, encoder, archive)
}

/// Create a .tar.bz2 archive from a source path.
///
/// # Arguments
///
/// * `source` - The file or directory to archive
/// * `archive` - The path for the output archive file
#[cfg(feature = "archive_tar_bzip2")]
pub fn tar_bz2(source: &Path, archive: &Path) -> XXResult<()> {
    if let Some(parent) = archive.parent() {
        file::mkdirp(parent)?;
    }
    let file = file::create(archive)?;
    let encoder = bzip2::write::BzEncoder::new(file, bzip2::Compression::default());
    create_tar_inner(source, encoder, archive)
}

/// Create a .tar.bz2 archive from multiple source paths.
#[cfg(feature = "archive_tar_bzip2")]
pub fn tar_bz2_multi(sources: &[&Path], archive: &Path) -> XXResult<()> {
    if let Some(parent) = archive.parent() {
        file::mkdirp(parent)?;
    }
    let file = file::create(archive)?;
    let encoder = bzip2::write::BzEncoder::new(file, bzip2::Compression::default());
    create_tar_multi_inner(sources, encoder, archive)
}

/// Create a .tar.xz archive from a source path.
///
/// # Arguments
///
/// * `source` - The file or directory to archive
/// * `archive` - The path for the output archive file
#[cfg(feature = "archive_tar_xz")]
pub fn tar_xz(source: &Path, archive: &Path) -> XXResult<()> {
    if let Some(parent) = archive.parent() {
        file::mkdirp(parent)?;
    }
    let file = file::create(archive)?;
    let encoder = xz2::write::XzEncoder::new(file, 6); // compression level 6 (default)
    create_tar_inner(source, encoder, archive)
}

/// Create a .tar.xz archive from multiple source paths.
#[cfg(feature = "archive_tar_xz")]
pub fn tar_xz_multi(sources: &[&Path], archive: &Path) -> XXResult<()> {
    if let Some(parent) = archive.parent() {
        file::mkdirp(parent)?;
    }
    let file = file::create(archive)?;
    let encoder = xz2::write::XzEncoder::new(file, 6);
    create_tar_multi_inner(sources, encoder, archive)
}

/// Internal helper for creating tar archives
#[cfg(any(
    feature = "archive_tar_gzip",
    feature = "archive_tar_bzip2",
    feature = "archive_tar_xz"
))]
fn create_tar_inner<W: FinishableWriter>(source: &Path, writer: W, archive: &Path) -> XXResult<()> {
    let mut builder = tar::Builder::new(writer);

    if source.is_dir() {
        // Use the directory name as the archive prefix
        let dir_name = source
            .file_name()
            .unwrap_or(std::ffi::OsStr::new("archive"));
        builder
            .append_dir_all(dir_name, source)
            .map_err(|err| XXError::ArchiveIOError(err, archive.to_path_buf()))?;
    } else {
        // Add single file with just its name
        let file_name = source.file_name().unwrap_or(std::ffi::OsStr::new("file"));
        let mut f = file::open(source)?;
        builder
            .append_file(file_name, &mut f)
            .map_err(|err| XXError::ArchiveIOError(err, archive.to_path_buf()))?;
    }

    // Finish the tar archive and get the encoder back
    let encoder = builder
        .into_inner()
        .map_err(|err| XXError::ArchiveIOError(err, archive.to_path_buf()))?;

    // Finish the compression encoder to ensure all data is flushed
    encoder
        .finish()
        .map_err(|err| XXError::ArchiveIOError(err, archive.to_path_buf()))?;

    Ok(())
}

/// Internal helper for creating tar archives from multiple sources
#[cfg(any(
    feature = "archive_tar_gzip",
    feature = "archive_tar_bzip2",
    feature = "archive_tar_xz"
))]
fn create_tar_multi_inner<W: FinishableWriter>(
    sources: &[&Path],
    writer: W,
    archive: &Path,
) -> XXResult<()> {
    let mut builder = tar::Builder::new(writer);

    for source in sources {
        if source.is_dir() {
            let dir_name = source.file_name().unwrap_or(std::ffi::OsStr::new("dir"));
            builder
                .append_dir_all(dir_name, source)
                .map_err(|err| XXError::ArchiveIOError(err, archive.to_path_buf()))?;
        } else {
            let file_name = source.file_name().unwrap_or(std::ffi::OsStr::new("file"));
            let mut f = file::open(source)?;
            builder
                .append_file(file_name, &mut f)
                .map_err(|err| XXError::ArchiveIOError(err, archive.to_path_buf()))?;
        }
    }

    // Finish the tar archive and get the encoder back
    let encoder = builder
        .into_inner()
        .map_err(|err| XXError::ArchiveIOError(err, archive.to_path_buf()))?;

    // Finish the compression encoder to ensure all data is flushed
    encoder
        .finish()
        .map_err(|err| XXError::ArchiveIOError(err, archive.to_path_buf()))?;

    Ok(())
}

/// Create a .zip archive from a source path.
///
/// If the source is a directory, all its contents will be included recursively.
/// If the source is a file, only that file will be included.
///
/// # Arguments
///
/// * `source` - The file or directory to archive
/// * `archive` - The path for the output archive file
///
/// # Example
///
/// ```rust,no_run
/// use xx::archive;
/// use std::path::Path;
///
/// archive::zip(Path::new("my-project/"), Path::new("my-project.zip")).unwrap();
/// ```
#[cfg(feature = "archive_zip")]
pub fn zip(source: &Path, archive: &Path) -> XXResult<()> {
    zip_multi(&[source], archive)
}

/// Create a .zip archive from multiple source paths.
///
/// # Arguments
///
/// * `sources` - The files and/or directories to archive
/// * `archive` - The path for the output archive file
///
/// # Example
///
/// ```rust,no_run
/// use xx::archive;
/// use std::path::Path;
///
/// archive::zip_multi(
///     &[Path::new("src/"), Path::new("Cargo.toml")],
///     Path::new("project.zip")
/// ).unwrap();
/// ```
#[cfg(feature = "archive_zip")]
pub fn zip_multi(sources: &[&Path], archive: &Path) -> XXResult<()> {
    use zip::write::SimpleFileOptions;

    if let Some(parent) = archive.parent() {
        file::mkdirp(parent)?;
    }

    let file = file::create(archive)?;
    let mut zip_writer = zip::ZipWriter::new(file);
    let options = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o644);

    for source in sources {
        if source.is_dir() {
            add_dir_to_zip(&mut zip_writer, source, source, archive, options)?;
        } else {
            add_file_to_zip(
                &mut zip_writer,
                source,
                source.file_name().unwrap_or(std::ffi::OsStr::new("file")),
                archive,
                options,
            )?;
        }
    }

    zip_writer
        .finish()
        .map_err(|err| XXError::ArchiveZipError(err, archive.to_path_buf()))?;
    Ok(())
}

/// Internal helper to add a directory recursively to a zip archive
///
/// Note: Symbolic links are skipped to prevent symlink cycles and security issues.
#[cfg(feature = "archive_zip")]
fn add_dir_to_zip<W: std::io::Write + std::io::Seek>(
    zip_writer: &mut zip::ZipWriter<W>,
    dir: &Path,
    base: &Path,
    archive: &Path,
    options: zip::write::SimpleFileOptions,
) -> XXResult<()> {
    let dir_name = base
        .file_name()
        .unwrap_or(std::ffi::OsStr::new("dir"))
        .to_string_lossy();

    for entry in
        std::fs::read_dir(dir).map_err(|err| XXError::ArchiveIOError(err, dir.to_path_buf()))?
    {
        let entry = entry.map_err(|err| XXError::ArchiveIOError(err, dir.to_path_buf()))?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|err| XXError::ArchiveIOError(err, path.clone()))?;

        // Skip symbolic links to prevent cycles and security issues
        if file_type.is_symlink() {
            trace!("Skipping symlink: {}", path.display());
            continue;
        }

        // Calculate relative path from base
        let relative = path
            .strip_prefix(base)
            .unwrap_or(&path)
            .to_string_lossy()
            .to_string();
        let archive_path = format!("{}/{}", dir_name, relative);

        if file_type.is_dir() {
            // Add directory entry with execute permissions
            let dir_options = options.unix_permissions(0o755);
            zip_writer
                .add_directory(&format!("{}/", archive_path), dir_options)
                .map_err(|err| XXError::ArchiveZipError(err, archive.to_path_buf()))?;

            // Recurse into subdirectory
            add_dir_to_zip(zip_writer, &path, base, archive, options)?;
        } else if file_type.is_file() {
            add_file_to_zip(
                zip_writer,
                &path,
                std::ffi::OsStr::new(&archive_path),
                archive,
                options,
            )?;
        }
        // Other file types (block devices, etc.) are silently skipped
    }

    Ok(())
}

/// Internal helper to add a file to a zip archive
///
/// Uses streaming to avoid loading entire files into memory.
#[cfg(feature = "archive_zip")]
fn add_file_to_zip<W: std::io::Write + std::io::Seek>(
    zip_writer: &mut zip::ZipWriter<W>,
    file_path: &Path,
    archive_name: &std::ffi::OsStr,
    archive: &Path,
    options: zip::write::SimpleFileOptions,
) -> XXResult<()> {
    let archive_name_str = archive_name.to_string_lossy();

    // Open file first to avoid corrupting the archive if the file can't be read
    let mut f = file::open(file_path)?;

    // Get file permissions on Unix
    #[cfg(unix)]
    let options = {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(metadata) = std::fs::metadata(file_path) {
            options.unix_permissions(metadata.permissions().mode())
        } else {
            options
        }
    };

    zip_writer
        .start_file(&*archive_name_str, options)
        .map_err(|err| XXError::ArchiveZipError(err, archive.to_path_buf()))?;

    // Use streaming copy to avoid loading large files into memory
    std::io::copy(&mut f, zip_writer)
        .map_err(|err| XXError::ArchiveIOError(err, file_path.to_path_buf()))?;

    Ok(())
}

/// Compress a file to .gz format.
///
/// # Arguments
///
/// * `source` - The file to compress
/// * `archive` - The path for the output .gz file
///
/// # Example
///
/// ```rust,no_run
/// use xx::archive;
/// use std::path::Path;
///
/// archive::gz(Path::new("large-file.txt"), Path::new("large-file.txt.gz")).unwrap();
/// ```
#[cfg(feature = "archive_gz")]
pub fn gz(source: &Path, archive: &Path) -> XXResult<()> {
    use std::io::{Read, Write};

    if let Some(parent) = archive.parent() {
        file::mkdirp(parent)?;
    }

    let mut input = file::open(source)?;
    let output = file::create(archive)?;
    let mut encoder = flate2::write::GzEncoder::new(output, flate2::Compression::default());

    let mut buffer = [0u8; 8192];
    loop {
        let bytes_read = input
            .read(&mut buffer)
            .map_err(|err| XXError::ArchiveIOError(err, source.to_path_buf()))?;
        if bytes_read == 0 {
            break;
        }
        encoder
            .write_all(&buffer[..bytes_read])
            .map_err(|err| XXError::ArchiveIOError(err, archive.to_path_buf()))?;
    }

    encoder
        .finish()
        .map_err(|err| XXError::ArchiveIOError(err, archive.to_path_buf()))?;
    Ok(())
}

// ============================================================================
// Archive Information Types
// ============================================================================

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

        let name = file
            .name()
            .map_err(|err| XXError::ArchiveZipError(err, archive.to_path_buf()))?
            .into_owned();

        entries.push(ArchiveEntry {
            path: name,
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
    use std::io::Write;
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

    #[cfg(feature = "archive_unzip")]
    #[test]
    fn test_unzip_into_new_destination() {
        let archive = Path::new(env!("CARGO_MANIFEST_DIR")).join("test/data/foo.zip");
        let tmp = tempfile::tempdir().unwrap();
        let destination = tmp.path().join("output");
        unzip(&archive, &destination.join(".")).unwrap();
        assert_eq!(
            fs::read_to_string(destination.join("foo/test.txt")).unwrap(),
            "yep\n"
        );
    }

    #[cfg(all(feature = "archive_unzip", unix))]
    #[test]
    fn test_unzip_rejects_aliased_destination_link() {
        use std::io::ErrorKind;
        use std::os::unix::fs::symlink;

        let archive = Path::new(env!("CARGO_MANIFEST_DIR")).join("test/data/foo.zip");
        let tmp = tempfile::tempdir().unwrap();
        let directory = tmp.path().join("real-output");
        let link = tmp.path().join("output");
        fs::create_dir(&directory).unwrap();
        symlink(&directory, &link).unwrap();

        for suffix in ["", "/", "/."] {
            let mut path = link.as_os_str().to_owned();
            path.push(suffix);
            let err = unzip(&archive, Path::new(&path)).unwrap_err();
            assert!(matches!(
                err,
                XXError::ArchiveIOError(err, ref path)
                    if err.kind() == ErrorKind::InvalidData && path == &link
            ));
        }
        assert_eq!(fs::read_dir(directory).unwrap().count(), 0);
    }

    #[cfg(feature = "archive_unzip")]
    #[test]
    fn test_unzip_stops_on_rejected_entry_name() {
        let archive = Path::new(env!("CARGO_MANIFEST_DIR")).join("test/data/foo.zip");
        let tmp = tempfile::tempdir().unwrap();

        // Exercise a rejected name through the extraction loop without an
        // archive containing an invalid name.
        let error = unzip_with_entry_names(&archive, tmp.path(), |_| None).unwrap_err();
        assert!(matches!(
            error,
            XXError::ArchiveZipError(zip::result::ZipError::InvalidArchive(reason), path)
                if reason == "invalid extraction path" && path == archive
        ));
        assert_eq!(fs::read_dir(tmp.path()).unwrap().count(), 0);
    }

    #[cfg(all(feature = "archive_unzip", unix))]
    #[test]
    fn test_unzip_rejects_existing_entry_links() {
        use std::io::ErrorKind;
        use std::os::unix::fs::symlink;

        let archive = Path::new(env!("CARGO_MANIFEST_DIR")).join("test/data/foo.zip");
        for link_at_directory in [true, false] {
            let tmp = tempfile::tempdir().unwrap();
            let destination = tmp.path().join("output");
            let peer = tmp.path().join("peer");
            fs::create_dir(&destination).unwrap();
            fs::create_dir(&peer).unwrap();
            let sentinel = peer.join("test.txt");
            fs::write(&sentinel, "unchanged").unwrap();

            let directory = destination.join("foo");
            let link = if link_at_directory {
                symlink(&peer, &directory).unwrap();
                directory
            } else {
                fs::create_dir(&directory).unwrap();
                let file = directory.join("test.txt");
                symlink(&sentinel, &file).unwrap();
                file
            };

            let error = unzip(&archive, &destination).unwrap_err();
            assert!(matches!(
                error,
                XXError::ArchiveIOError(err, path)
                    if err.kind() == ErrorKind::InvalidData && path == destination
            ));
            assert!(fs::symlink_metadata(&link).unwrap().is_symlink());
            assert_eq!(fs::read_to_string(sentinel).unwrap(), "unchanged");
            assert_eq!(fs::read_dir(peer).unwrap().count(), 1);
        }
    }

    #[cfg(feature = "archive_unzip")]
    #[test]
    fn test_zip_output_path_validation() {
        use std::io::ErrorKind;

        let tmp = tempfile::tempdir().unwrap();
        assert_eq!(
            zip_output_path(tmp.path(), Path::new("./nested/file.txt"), false).unwrap(),
            tmp.path().join("nested/file.txt")
        );
        for path in [tmp.path(), Path::new("."), Path::new("nested/..")] {
            assert_eq!(
                zip_output_path(tmp.path(), path, false).unwrap_err().kind(),
                ErrorKind::InvalidData
            );
        }
    }

    #[cfg(all(feature = "archive_unzip", unix))]
    #[test]
    fn test_zip_output_path_rejects_existing_links() {
        use std::io::ErrorKind;
        use std::os::unix::fs::symlink;

        let tmp = tempfile::tempdir().unwrap();
        let directory = tmp.path().join("directory");
        fs::create_dir(&directory).unwrap();
        fs::write(directory.join("file.txt"), "unchanged").unwrap();
        symlink(&directory, tmp.path().join("directory-link")).unwrap();
        symlink(directory.join("file.txt"), tmp.path().join("file-link")).unwrap();
        fs::hard_link(directory.join("file.txt"), tmp.path().join("hard-link")).unwrap();

        for path in ["directory-link/child", "file-link", "hard-link"] {
            assert_eq!(
                zip_output_path(tmp.path(), Path::new(path), false)
                    .unwrap_err()
                    .kind(),
                ErrorKind::InvalidData
            );
        }
    }

    #[cfg(feature = "archive_ungz")]
    #[test]
    fn test_ungz() {
        use flate2::Compression;
        use flate2::write::GzEncoder;

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

    // ========================================================================
    // Archive Creation Tests
    // ========================================================================

    #[cfg(all(feature = "archive_tar_gzip", feature = "archive_untar_gzip"))]
    #[test]
    fn test_tar_gz_create() {
        let tmpdir = tempfile::tempdir().unwrap();

        // Create test directory structure
        let source_dir = tmpdir.path().join("source");
        fs::create_dir_all(&source_dir).unwrap();
        fs::write(source_dir.join("file1.txt"), "content1").unwrap();
        fs::create_dir_all(source_dir.join("subdir")).unwrap();
        fs::write(source_dir.join("subdir/file2.txt"), "content2").unwrap();

        // Create archive
        let archive_path = tmpdir.path().join("test.tar.gz");
        tar_gz(&source_dir, &archive_path).unwrap();
        assert!(archive_path.exists());

        // Extract and verify
        let extract_dir = tmpdir.path().join("extracted");
        untar_gz(&archive_path, &extract_dir).unwrap();

        assert!(extract_dir.join("source/file1.txt").exists());
        assert!(extract_dir.join("source/subdir/file2.txt").exists());
        assert_eq!(
            fs::read_to_string(extract_dir.join("source/file1.txt")).unwrap(),
            "content1"
        );
    }

    #[cfg(all(feature = "archive_tar_gzip", feature = "archive_untar_gzip"))]
    #[test]
    fn test_tar_gz_single_file() {
        let tmpdir = tempfile::tempdir().unwrap();

        // Create test file
        let source_file = tmpdir.path().join("single.txt");
        fs::write(&source_file, "single file content").unwrap();

        // Create archive
        let archive_path = tmpdir.path().join("single.tar.gz");
        tar_gz(&source_file, &archive_path).unwrap();
        assert!(archive_path.exists());

        // Extract and verify
        let extract_dir = tmpdir.path().join("extracted");
        untar_gz(&archive_path, &extract_dir).unwrap();

        assert!(extract_dir.join("single.txt").exists());
        assert_eq!(
            fs::read_to_string(extract_dir.join("single.txt")).unwrap(),
            "single file content"
        );
    }

    #[cfg(all(feature = "archive_tar_gzip", feature = "archive_untar_gzip"))]
    #[test]
    fn test_tar_gz_multi() {
        let tmpdir = tempfile::tempdir().unwrap();

        // Create multiple source files/dirs
        let file1 = tmpdir.path().join("file1.txt");
        fs::write(&file1, "file1 content").unwrap();

        let dir1 = tmpdir.path().join("mydir");
        fs::create_dir_all(&dir1).unwrap();
        fs::write(dir1.join("nested.txt"), "nested content").unwrap();

        // Create archive from multiple sources
        let archive_path = tmpdir.path().join("multi.tar.gz");
        tar_gz_multi(&[file1.as_path(), dir1.as_path()], &archive_path).unwrap();
        assert!(archive_path.exists());

        // List and verify contents
        let entries = list_tar_gz(&archive_path).unwrap();
        assert!(entries.iter().any(|e| e.path.contains("file1.txt")));
        assert!(entries.iter().any(|e| e.path.contains("mydir")));
    }

    #[cfg(all(feature = "archive_tar_bzip2", feature = "archive_untar_bzip2"))]
    #[test]
    fn test_tar_bz2_create() {
        let tmpdir = tempfile::tempdir().unwrap();

        // Create test directory
        let source_dir = tmpdir.path().join("source");
        fs::create_dir_all(&source_dir).unwrap();
        fs::write(source_dir.join("test.txt"), "bz2 test").unwrap();

        // Create archive
        let archive_path = tmpdir.path().join("test.tar.bz2");
        tar_bz2(&source_dir, &archive_path).unwrap();
        assert!(archive_path.exists());

        // Extract and verify
        let extract_dir = tmpdir.path().join("extracted");
        untar_bz2(&archive_path, &extract_dir).unwrap();
        assert!(extract_dir.join("source/test.txt").exists());
        assert_eq!(
            fs::read_to_string(extract_dir.join("source/test.txt")).unwrap(),
            "bz2 test"
        );
    }

    #[cfg(all(feature = "archive_tar_xz", feature = "archive_untar_xz"))]
    #[test]
    fn test_tar_xz_create() {
        let tmpdir = tempfile::tempdir().unwrap();

        // Create test directory
        let source_dir = tmpdir.path().join("source");
        fs::create_dir_all(&source_dir).unwrap();
        fs::write(source_dir.join("test.txt"), "xz test").unwrap();

        // Create archive
        let archive_path = tmpdir.path().join("test.tar.xz");
        tar_xz(&source_dir, &archive_path).unwrap();
        assert!(archive_path.exists());

        // Extract and verify
        let extract_dir = tmpdir.path().join("extracted");
        untar_xz(&archive_path, &extract_dir).unwrap();
        assert!(extract_dir.join("source/test.txt").exists());
        assert_eq!(
            fs::read_to_string(extract_dir.join("source/test.txt")).unwrap(),
            "xz test"
        );
    }

    #[cfg(all(feature = "archive_zip", feature = "archive_unzip"))]
    #[test]
    fn test_zip_create() {
        let tmpdir = tempfile::tempdir().unwrap();

        // Create test directory structure
        let source_dir = tmpdir.path().join("source");
        fs::create_dir_all(&source_dir).unwrap();
        fs::write(source_dir.join("file1.txt"), "zip content1").unwrap();
        fs::create_dir_all(source_dir.join("subdir")).unwrap();
        fs::write(source_dir.join("subdir/file2.txt"), "zip content2").unwrap();

        // Create archive
        let archive_path = tmpdir.path().join("test.zip");
        zip(&source_dir, &archive_path).unwrap();
        assert!(archive_path.exists());

        // Extract and verify
        let extract_dir = tmpdir.path().join("extracted");
        unzip(&archive_path, &extract_dir).unwrap();

        assert!(extract_dir.join("source/file1.txt").exists());
        assert!(extract_dir.join("source/subdir/file2.txt").exists());
        assert_eq!(
            fs::read_to_string(extract_dir.join("source/file1.txt")).unwrap(),
            "zip content1"
        );
    }

    #[cfg(all(feature = "archive_zip", feature = "archive_unzip"))]
    #[test]
    fn test_zip_single_file() {
        let tmpdir = tempfile::tempdir().unwrap();

        // Create test file
        let source_file = tmpdir.path().join("single.txt");
        fs::write(&source_file, "single zip content").unwrap();

        // Create archive
        let archive_path = tmpdir.path().join("single.zip");
        zip(&source_file, &archive_path).unwrap();
        assert!(archive_path.exists());

        // Extract and verify
        let extract_dir = tmpdir.path().join("extracted");
        unzip(&archive_path, &extract_dir).unwrap();

        assert!(extract_dir.join("single.txt").exists());
        assert_eq!(
            fs::read_to_string(extract_dir.join("single.txt")).unwrap(),
            "single zip content"
        );
    }

    #[cfg(all(feature = "archive_zip", feature = "archive_unzip"))]
    #[test]
    fn test_zip_multi() {
        let tmpdir = tempfile::tempdir().unwrap();

        // Create multiple source files
        let file1 = tmpdir.path().join("a.txt");
        fs::write(&file1, "a content").unwrap();

        let file2 = tmpdir.path().join("b.txt");
        fs::write(&file2, "b content").unwrap();

        // Create archive
        let archive_path = tmpdir.path().join("multi.zip");
        zip_multi(&[file1.as_path(), file2.as_path()], &archive_path).unwrap();
        assert!(archive_path.exists());

        // List and verify contents
        let entries = list_zip(&archive_path).unwrap();
        assert!(entries.iter().any(|e| e.path.contains("a.txt")));
        assert!(entries.iter().any(|e| e.path.contains("b.txt")));
    }

    #[cfg(all(feature = "archive_gz", feature = "archive_ungz"))]
    #[test]
    fn test_gz_create() {
        let tmpdir = tempfile::tempdir().unwrap();

        // Create test file
        let source_file = tmpdir.path().join("data.txt");
        let content = "This is test content for gz compression.\n".repeat(100);
        fs::write(&source_file, &content).unwrap();

        // Compress
        let archive_path = tmpdir.path().join("data.txt.gz");
        gz(&source_file, &archive_path).unwrap();
        assert!(archive_path.exists());

        // Verify compressed file is smaller
        let original_size = fs::metadata(&source_file).unwrap().len();
        let compressed_size = fs::metadata(&archive_path).unwrap().len();
        assert!(compressed_size < original_size);

        // Decompress and verify
        let extracted_path = tmpdir.path().join("extracted.txt");
        ungz(&archive_path, &extracted_path).unwrap();
        assert_eq!(fs::read_to_string(&extracted_path).unwrap(), content);
    }
}
