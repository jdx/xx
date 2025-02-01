use std::{
    path::{Path, PathBuf},
    vec,
};

use duct::{cmd, Expression};
use miette::{miette, Result};

use crate::{file, XXError, XXResult};

pub struct Git {
    pub dir: PathBuf,
}

macro_rules! git_cmd {
    ( $dir:expr $(, $arg:expr )* $(,)? ) => {
        {
            let safe = format!("safe.directory={}", $dir.display());
            cmd!("git", "-C", $dir, "-c", safe $(, $arg)*)
        }
    }
}

impl Git {
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    pub fn is_repo(&self) -> bool {
        self.dir.join(".git").is_dir()
    }

    pub fn update(&self, gitref: Option<String>) -> Result<(String, String)> {
        let gitref = gitref.map_or_else(|| self.current_branch(), Ok)?;
        debug!("updating {} to {}", self.dir.display(), gitref);
        let exec = |cmd: Expression| match cmd.stderr_to_stdout().stdout_capture().unchecked().run()
        {
            Ok(res) => {
                if res.status.success() {
                    Ok(())
                } else {
                    Err(miette!(
                        "git failed: {cmd:?} {}",
                        String::from_utf8(res.stdout).unwrap()
                    ))
                }
            }
            Err(err) => Err(miette!("git failed: {cmd:?} {err:#}")),
        };
        exec(git_cmd!(
            &self.dir,
            "fetch",
            "--prune",
            "--update-head-ok",
            "origin",
            &format!("{}:{}", gitref, gitref),
        ))?;
        let prev_rev = self.current_sha()?;
        exec(git_cmd!(
            &self.dir,
            "-c",
            "advice.detachedHead=false",
            "-c",
            "advice.objectNameWarning=false",
            "checkout",
            "--force",
            &gitref
        ))?;
        let post_rev = self.current_sha()?;
        file::touch_dir(&self.dir)?;

        Ok((prev_rev, post_rev))
    }

    pub fn current_branch(&self) -> XXResult<String> {
        let branch = git_cmd!(&self.dir, "branch", "--show-current")
            .read()
            .map_err(|err| XXError::GitError(err, self.dir.clone()))?;
        debug!("current branch for {}: {}", self.dir.display(), &branch);
        Ok(branch)
    }
    pub fn current_sha(&self) -> XXResult<String> {
        let sha = git_cmd!(&self.dir, "rev-parse", "HEAD")
            .read()
            .map_err(|err| XXError::GitError(err, self.dir.clone()))?;
        debug!("current sha for {}: {}", self.dir.display(), &sha);
        Ok(sha)
    }

    pub fn current_sha_short(&self) -> XXResult<String> {
        let sha = git_cmd!(&self.dir, "rev-parse", "--short", "HEAD")
            .read()
            .map_err(|err| XXError::GitError(err, self.dir.clone()))?;
        debug!("current sha for {}: {}", self.dir.display(), &sha);
        Ok(sha)
    }

    pub fn current_abbrev_ref(&self) -> XXResult<String> {
        let aref = git_cmd!(&self.dir, "rev-parse", "--abbrev-ref", "HEAD")
            .read()
            .map_err(|err| XXError::GitError(err, self.dir.clone()))?;
        debug!("current abbrev ref for {}: {}", self.dir.display(), &aref);
        Ok(aref)
    }

    pub fn get_remote_url(&self) -> Option<String> {
        if !self.dir.exists() {
            return None;
        }
        let res = git_cmd!(&self.dir, "config", "--get", "remote.origin.url").read();
        match res {
            Ok(url) => {
                debug!("remote url for {}: {}", self.dir.display(), &url);
                Some(url)
            }
            Err(err) => {
                warn!(
                    "failed to get remote url for {}: {:#}",
                    self.dir.display(),
                    err
                );
                None
            }
        }
    }

    pub fn split_url_and_ref(url: &str) -> (String, Option<String>) {
        match url.split_once('#') {
            Some((url, _ref)) => (url.to_string(), Some(_ref.to_string())),
            None => (url.to_string(), None),
        }
    }
}

pub fn clone<D: AsRef<Path>>(url: &str, dir: D, branch: Option<String>) -> XXResult<Git> {
    let dir = dir.as_ref().to_path_buf();
    debug!("cloning {} to {}", url, dir.display());
    if let Some(parent) = dir.parent() {
        file::mkdirp(parent)?;
    }
    match get_git_version() {
        Ok(version) => trace!("git version: {}", version),
        Err(err) => warn!(
            "failed to get git version: {:#}\n Git is required to use mise.",
            err
        ),
    }

    let mut cmd_args: Vec<String> = vec![
        "clone".to_string(),
        "-q".to_string(),
        "--depth".to_string(),
        "1".to_string(),
        url.to_string(),
        dir.to_string_lossy().to_string(),
    ];

    if let Some(value) = branch {
        cmd_args.push("--branch".to_string());
        cmd_args.push(value);
    }

    cmd("git", &cmd_args)
        .run()
        .map_err(|err| XXError::GitError(err, dir.clone()))?;

    Ok(Git::new(dir))
}

fn get_git_version() -> Result<String> {
    let version = cmd!("git", "--version")
        .read()
        .map_err(|err| XXError::GitError(err, PathBuf::new()))?;
    Ok(version.trim().into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_log::test;

    #[test]
    fn test_git() {
        file::remove_dir_all("/tmp/xx").unwrap_or(());
        let git = Git::new(PathBuf::from("/tmp/xx"));
        assert!(!git.is_repo());
        assert_eq!(git.get_remote_url(), None);
        assert!(git.current_branch().is_err());
        assert!(git.current_sha().is_err());
        assert!(git.current_sha_short().is_err());
        assert!(git.current_abbrev_ref().is_err());

        let git = clone("https://github.com/jdx/xx", &git.dir, None).unwrap();
        assert!(git.is_repo());
        assert_eq!(
            git.get_remote_url(),
            Some("https://github.com/jdx/xx".to_string())
        );

        file::remove_dir_all("/tmp/xx").unwrap();
    }
}
