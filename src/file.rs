use std::collections::BTreeSet;
use std::fs;
#[cfg(unix)]
use std::os::unix::prelude::*;
use std::path::Path;
use std::path::PathBuf;

#[cfg(feature = "glob")]
use globwalk::GlobWalkerBuilder;

use crate::{XXError, XXResult};

pub use std::fs::*;

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
///
pub fn write<P: AsRef<Path>, C: AsRef<[u8]>>(path: P, contents: C) -> XXResult<()> {
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

/// Move a file or directory
/// # Arguments
/// * `from` - A path to a file or directory
/// * `to` - A path to move the file or directory to
/// # Example
/// ```
/// xx::file::create("/tmp/foo").unwrap();
/// xx::file::mv("/tmp/foo", "/tmp/bar").unwrap();
/// ```
pub fn mv<P: AsRef<Path>, Q: AsRef<Path>>(from: P, to: Q) -> XXResult<()> {
    let from = from.as_ref();
    let to = to.as_ref();
    debug!("mv: {:?} -> {:?}", from, to);
    mkdirp(to.parent().unwrap())?;
    fs::rename(from, to).map_err(|err| XXError::FileError(err, from.to_path_buf()))?;
    Ok(())
}

/// Remove a directory and all its contents
/// # Arguments
/// * `path` - A path to a directory
/// # Example
/// ```
/// use xx::file::remove_dir_all;
/// remove_dir_all("/tmp/foo").unwrap();
/// ```
pub fn remove_dir_all<P: AsRef<Path>>(path: P) -> XXResult<()> {
    let path = path.as_ref();
    if path.exists() {
        debug!("remove_dir_all: {:?}", path);
        fs::remove_dir_all(path).map_err(|err| XXError::FileError(err, path.to_path_buf()))?;
    }
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

/// replaces $HOME with "~"
/// # Arguments
/// * `path` - A path
/// # Returns
/// A string with the path
/// # Example
/// ```
/// use xx::file::display_path;
/// display_path("/home/user/foo"); // "~/foo"
/// display_path("/tmp/foo"); // "/tmp/foo"
/// ```
pub fn display_path<P: AsRef<Path>>(path: P) -> String {
    let home = homedir::my_home().unwrap_or_default();
    let home = home.unwrap_or("/".into()).to_string_lossy().to_string();
    let path = path.as_ref();
    match path.starts_with(&home) && home != "/" {
        true => path.to_string_lossy().replacen(&home, "~", 1),
        false => path.display().to_string(),
    }
}

#[cfg(unix)]
/// Change the mode of a file
/// # Arguments
/// * `path` - A path to a file
/// * `mode` - A mode as an octal number
/// # Returns
/// A result
/// # Errors
/// Returns an error if the mode cannot be changed
/// # Example
/// ```
/// use xx::file::chmod;
/// chmod("src/lib.rs", 0o644).unwrap();
/// ```
pub fn chmod<P: AsRef<Path>>(path: P, mode: u32) -> XXResult<()> {
    let path = path.as_ref();
    debug!("chmod: {mode:o} {path:?}");
    fs::set_permissions(path, fs::Permissions::from_mode(mode))
        .map_err(|err| XXError::FileError(err, path.to_path_buf()))?;
    Ok(())
}

/// Find a file in the current directory or any parent directories
pub fn find_up<FN: AsRef<str>>(from: &Path, filenames: &[FN]) -> Option<PathBuf> {
    let mut current = from.to_path_buf();
    loop {
        for filename in filenames {
            let path = current.join(filename.as_ref());
            if path.exists() {
                return Some(path);
            }
        }
        if !current.pop() {
            return None;
        }
    }
}

pub fn find_up_all<FN: AsRef<str>>(from: &Path, filenames: &[FN]) -> Vec<PathBuf> {
    let mut current = from.to_path_buf();
    let mut paths = vec![];
    loop {
        for filename in filenames {
            let path = current.join(filename.as_ref());
            if path.exists() {
                paths.push(path);
            }
        }
        if !current.pop() {
            return paths;
        }
    }
}

#[cfg(unix)]
/// Make a file executable
/// # Arguments
/// * `path` - A path to a file
/// # Returns
/// A result
/// # Errors
/// Returns an error if the file cannot be made executable
/// # Example
/// ```
/// use xx::file::make_executable;
/// make_executable("src/lib.rs").unwrap();
/// ```
pub fn make_executable<P: AsRef<Path>>(path: P) -> XXResult<()> {
    let path = path.as_ref();
    let metadata = fs::metadata(path).map_err(|err| XXError::FileError(err, path.to_path_buf()))?;
    let mode = metadata.permissions().mode();
    if mode != 0o111 {
        chmod(path, mode | 0o111)?;
    }
    Ok(())
}

#[cfg(windows)]
pub fn make_executable<P: AsRef<Path>>(_path: P) -> XXResult<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_str_eq;
    use test_log::test;

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

    #[cfg(unix)]
    #[test]
    fn test_chmod() {
        let tmpdir = test::tempdir();
        let path = tmpdir.path().join("test.txt");
        write(&path, "Hello, world!").unwrap();
        chmod(&path, 0o755).unwrap();
        let metadata = fs::metadata(&path).unwrap();
        let mode = metadata.permissions().mode();
        assert_eq!(format!("{mode:o}"), "100755");
    }

    #[cfg(unix)]
    #[test]
    fn test_make_executable() {
        let tmpdir = test::tempdir();
        let path = tmpdir.path().join("test.txt");
        write(&path, "Hello, world!").unwrap();
        chmod(&path, 0o644).unwrap();
        make_executable(&path).unwrap();
        let metadata = fs::metadata(&path).unwrap();
        let mode = metadata.permissions().mode();
        assert_eq!(format!("{mode:o}"), "100755");
    }
}
