use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use crate::{XXError, XXResult};

pub fn open(path: &Path) -> XXResult<fs::File> {
    debug!("open: {:?}", path);
    fs::File::open(path).map_err(|err| XXError::FileError(err, path.to_path_buf()))
}

pub fn create(path: &Path) -> XXResult<fs::File> {
    debug!("create: {:?}", path);
    fs::File::create(path).map_err(|err| XXError::FileError(err, path.to_path_buf()))
}

pub fn read_to_string<P: AsRef<Path>>(path: P) -> XXResult<String> {
    debug!("read_to_string: {:?}", path.as_ref());
    let path = path.as_ref();
    let contents =
        fs::read_to_string(path).map_err(|err| XXError::FileError(err, path.to_path_buf()))?;
    Ok(contents)
}

pub fn write<P: AsRef<Path>>(path: P, contents: &str) -> XXResult<()> {
    debug!("write: {:?}", path.as_ref());
    let path = path.as_ref();
    mkdirp(path.parent().unwrap())?;
    fs::write(path, contents).map_err(|err| XXError::FileError(err, path.to_path_buf()))?;
    Ok(())
}

pub fn mkdirp<P: AsRef<Path>>(path: P) -> XXResult<()> {
    let path = path.as_ref();
    if path.exists() {
        return Ok(());
    }
    debug!("mkdirp: {:?}", path);
    fs::create_dir_all(path).map_err(|err| XXError::FileError(err, path.to_path_buf()))?;
    Ok(())
}

pub fn touch_dir(dir: &Path) -> XXResult<()> {
    trace!("touch {}", dir.display());
    let now = filetime::FileTime::now();
    filetime::set_file_times(dir, now, now)
        .map_err(|err| XXError::FileError(err, dir.to_path_buf()))?;
    Ok(())
}

pub fn ls(path: &Path) -> XXResult<Vec<PathBuf>> {
    debug!("ls: {:?}", path);
    let entries = fs::read_dir(path).map_err(|err| XXError::FileError(err, path.to_path_buf()))?;
    let mut files = BTreeSet::new();
    for entry in entries {
        let entry = entry.map_err(|err| XXError::FileError(err, path.to_path_buf()))?;
        files.insert(entry.path());
    }
    Ok(files.into_iter().collect())
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_str_eq;

    use crate::test;

    use super::*;

    #[test]
    fn test_read_to_string() {
        let tmpdir = test::tempdir();
        let path = tmpdir.path().join("test.txt");
        write(&path, "Hello, world!").unwrap();
        assert_str_eq!(read_to_string(&path).unwrap(), "Hello, world!");
    }

    #[test]
    fn test_read_file_not_found() {
        let tmpdir = test::tempdir();
        let path = tmpdir.path().join("test.txt");
        let err = read_to_string(path).unwrap_err();
        assert_eq!(
            err.to_string().split_once('\n').unwrap().0,
            "No such file or directory (os error 2)"
        );
    }
}
