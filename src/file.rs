use std::fs;
use std::path::Path;

use crate::{XXError, XXResult};

pub fn read_to_string<P: AsRef<Path>>(path: P) -> XXResult<String> {
    let path = path.as_ref();
    let contents =
        fs::read_to_string(path).map_err(|err| XXError::FileError(err, path.to_path_buf()))?;
    Ok(contents)
}

pub fn write<P: AsRef<Path>>(path: P, contents: &str) -> XXResult<()> {
    let path = path.as_ref();
    fs::write(path, contents).map_err(|err| XXError::FileError(err, path.to_path_buf()))?;
    Ok(())
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
