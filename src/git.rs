use std::path::PathBuf;

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

    pub fn clone(&self, url: &str) -> XXResult<()> {
        debug!("cloning {} to {}", url, self.dir.display());
        if let Some(parent) = self.dir.parent() {
            file::mkdirp(parent)?;
        }
        match get_git_version() {
            Ok(version) => trace!("git version: {}", version),
            Err(err) => warn!(
                "failed to get git version: {:#}\n Git is required to use mise.",
                err
            ),
        }
        cmd!("git", "clone", "-q", "--depth", "1", url, &self.dir)
            .run()
            .map_err(|err| XXError::GitError(err, self.dir.clone()))?;
        Ok(())
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

fn get_git_version() -> Result<String> {
    let version = cmd!("git", "--version")
        .read()
        .map_err(|err| XXError::GitError(err, PathBuf::new()))?;
    Ok(version.trim().into())
}
