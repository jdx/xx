/// Archive file handling functions.
use std::path::Path;

use crate::{file, XXError, XXResult};

/// Unpack a .tar.gz archive to a destination directory.
#[cfg(feature = "archive_untar_gzip")]
pub fn untar_gz(archive: &Path, destination: &Path) -> XXResult<()> {
    let file = file::open(archive)?;
    let mut a = tar::Archive::new(flate2::read::GzDecoder::new(file));
    a.unpack(destination)
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
    let mut a = tar::Archive::new(xz::read::XzDecoder::new(file));
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

#[cfg(test)]
mod tests {
    use std::fs;
    use test_log::test;

    use super::*;

    #[cfg(feature = "archive_untar_gzip")]
    #[test]
    fn test_untar_gz() {
        let archive = Path::new("test/data/foo.tar.gz");
        let destination = Path::new("/tmp/test_untar_gz");
        let _ = fs::remove_dir_all(destination);
        untar_gz(archive, destination).unwrap();
        assert!(destination.exists());
        assert!(destination.join("foo/test.txt").exists());
        assert_eq!(
            fs::read_to_string(destination.join("foo/test.txt")).unwrap(),
            "yep\n"
        );
        fs::remove_dir_all(destination).unwrap();
    }

    #[cfg(feature = "archive_untar_bzip2")]
    #[test]
    fn test_untar_bz2() {
        let archive = Path::new("test/data/foo.tar.bz2");
        let destination = Path::new("/tmp/test_untar_bz2");
        let _ = fs::remove_dir_all(destination);
        untar_bz2(archive, destination).unwrap();
        assert!(destination.exists());
        assert!(destination.join("foo/test.txt").exists());
        assert_eq!(
            fs::read_to_string(destination.join("foo/test.txt")).unwrap(),
            "yep\n"
        );
        fs::remove_dir_all(destination).unwrap();
    }

    #[cfg(feature = "archive_untar_xz")]
    #[test]
    fn test_untar_xz() {
        let archive = Path::new("test/data/foo.tar.xz");
        let destination = Path::new("/tmp/test_untar_xz");
        let _ = fs::remove_dir_all(destination);
        untar_xz(archive, destination).unwrap();
        assert!(destination.exists());
        assert!(destination.join("foo/test.txt").exists());
        assert_eq!(
            fs::read_to_string(destination.join("foo/test.txt")).unwrap(),
            "yep\n"
        );
        fs::remove_dir_all(destination).unwrap();
    }

    #[cfg(feature = "archive_unzip")]
    #[test]
    fn test_unzip() {
        let archive = Path::new("test/data/foo.zip");
        let destination = Path::new("/tmp/test_unzip");
        let _ = fs::remove_dir_all(destination);
        unzip(archive, destination).unwrap();
        assert!(destination.exists());
        assert!(destination.join("foo/test.txt").exists());
        assert_eq!(
            fs::read_to_string(destination.join("foo/test.txt")).unwrap(),
            "yep\n"
        );
        fs::remove_dir_all(destination).unwrap();
    }
}
