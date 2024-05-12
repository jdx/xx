use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use crate::{XXError, XXResult};

/// Open a file for reading
/// # Arguments
/// * `path` - A path to a file
/// # Returns
/// A file handle
/// # Errors
/// Returns an error if the file does not exist
/// # Example
/// ```
/// use xx::file::open;
/// let file = open("src/lib.rs").unwrap();
/// ```
pub fn open<P: AsRef<Path>>(path: P) -> XXResult<fs::File> {
    let path = path.as_ref();
    debug!("open: {:?}", path);
    fs::File::open(path).map_err(|err| XXError::FileError(err, path.to_path_buf()))
}

/// Create a file for writing
/// # Arguments
/// * `path` - A path to a file
/// # Returns
/// A file handle
/// # Errors
/// Returns an error if the file cannot be created
/// # Example
/// ```
/// use xx::file::create;
/// let tmp = tempfile::tempdir().unwrap();
/// let file = create(tmp.path().join("test.txt")).unwrap();
/// ```
pub fn create<P: AsRef<Path>>(path: P) -> XXResult<fs::File> {
    let path = path.as_ref();
    debug!("create: {:?}", path);
    fs::File::create(path).map_err(|err| XXError::FileError(err, path.to_path_buf()))
}

/// Read a file to a string
/// # Arguments
/// * `path` - A path to a file
/// # Returns
/// A string with the file contents
/// # Errors
/// Returns an error if the file does not exist
/// # Example
/// ```
/// use xx::file::read_to_string;
/// let contents = read_to_string("src/lib.rs").unwrap();
/// ```
pub fn read_to_string<P: AsRef<Path>>(path: P) -> XXResult<String> {
    debug!("read_to_string: {:?}", path.as_ref());
    let path = path.as_ref();
    let contents =
        fs::read_to_string(path).map_err(|err| XXError::FileError(err, path.to_path_buf()))?;
    Ok(contents)
}

/// Write a string to a file
/// # Arguments
/// * `path` - A path to a file
/// * `contents` - A string with the file contents
/// # Returns
/// A result
/// # Errors
/// Returns an error if the file cannot be written
/// # Example
/// ```
/// use xx::file::write;
/// let tmpdir = tempfile::tempdir().unwrap();
/// let path = tmpdir.path().join("test.txt");
/// write(&path, "Hello, world!").unwrap();
/// ```
pub fn write<P: AsRef<Path>>(path: P, contents: &str) -> XXResult<()> {
    debug!("write: {:?}", path.as_ref());
    let path = path.as_ref();
    mkdirp(path.parent().unwrap())?;
    fs::write(path, contents).map_err(|err| XXError::FileError(err, path.to_path_buf()))?;
    Ok(())
}

/// Create a directory and any missing parent directories
/// # Arguments
/// * `path` - A path to a directory
/// # Returns
/// A result
/// # Errors
/// Returns an error if the directory cannot be created
/// # Example
/// ```
/// use xx::file::mkdirp;
/// mkdirp("src").unwrap();
/// ```
pub fn mkdirp<P: AsRef<Path>>(path: P) -> XXResult<()> {
    let path = path.as_ref();
    if path.exists() {
        return Ok(());
    }
    debug!("mkdirp: {:?}", path);
    fs::create_dir_all(path).map_err(|err| XXError::FileError(err, path.to_path_buf()))?;
    Ok(())
}

/// Update a directory's last modified time
/// # Arguments
/// * `dir` - A path to a directory
/// # Returns
/// A result
/// # Errors
/// Returns an error if the directory does not exist
/// # Example
/// ```
/// use xx::file::touch_dir;
/// touch_dir("src").unwrap();
/// ```
pub fn touch_dir<P: AsRef<Path>>(dir: P) -> XXResult<()> {
    let dir = dir.as_ref().to_path_buf();
    trace!("touch {}", dir.display());
    mkdirp(&dir)?;
    let now = filetime::FileTime::now();
    filetime::set_file_times(&dir, now, now).map_err(|err| XXError::FileError(err, dir.clone()))?;
    Ok(())
}

/// List files in a directory
/// # Arguments
/// * `path` - A path to a directory
/// # Returns
/// A vector of paths in the directory
/// # Errors
/// Returns an error if the directory does not exist
/// # Example
/// ```
/// use xx::file::ls;
/// let files = ls("src").unwrap();
/// ```
pub fn ls<P: AsRef<Path>>(path: P) -> XXResult<Vec<PathBuf>> {
    let path = path.as_ref().to_path_buf();
    debug!("ls: {:?}", &path);
    let entries = fs::read_dir(&path).map_err(|err| XXError::FileError(err, path.clone()))?;
    let mut files = BTreeSet::new();
    for entry in entries {
        let entry = entry.map_err(|err| XXError::FileError(err, path.clone()))?;
        files.insert(entry.path());
    }
    Ok(files.into_iter().collect())
}

#[cfg(feature = "glob")]
use globwalk::GlobWalkerBuilder;

#[cfg(feature = "glob")]
/// Glob for files matching the given pattern
/// # Arguments
/// * `input` - A path with a glob pattern
/// # Returns
/// A vector of paths matching the glob pattern
/// # Errors
/// Returns an error if the glob pattern is invalid
/// # Example
/// ```
/// use xx::file::glob;
/// let files = glob("src/*.rs").unwrap();
/// for file in files {
///    println!("{}", file.display());
/// }
/// ```
pub fn glob<P: Into<PathBuf>>(input: P) -> XXResult<Vec<PathBuf>> {
    let input = input.into();
    debug!("glob: {:?}", input);
    // Use the longest path without any glob pattern character as root
    let root = input
        .ancestors()
        .skip(1)
        .find(|a| !"*[{?".chars().any(|c| a.to_str().unwrap().contains(c)))
        .unwrap()
        .to_path_buf();
    let pattern = input.strip_prefix(&root).unwrap();
    let files = if pattern.to_string_lossy().contains('*') {
        GlobWalkerBuilder::new(root, pattern.to_string_lossy())
            .follow_links(true)
            .build()
            .map_err(|err| XXError::GlobwalkError(err, input))?
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .map(|e| e.into_path())
            .collect()
    } else {
        vec![root.join(pattern)]
    };
    Ok(files)
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

    #[cfg(feature = "glob")]
    #[test]
    fn test_glob() {
        let tmpdir = test::tempdir();
        let dir = tmpdir.path().join("dir");
        fs::create_dir(&dir).unwrap();
        let file1 = dir.join("file1.txt");
        let file2 = dir.join("file2.txt");
        write(&file1, "Hello, world!").unwrap();
        write(&file2, "Goodbye, world!").unwrap();
        let files = glob(dir.join("*.txt")).unwrap();
        assert_eq!(files.len(), 2);
        assert!(files.contains(&file1));
        assert!(files.contains(&file2));
    }
}
