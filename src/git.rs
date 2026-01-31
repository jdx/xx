//! Git repository operations
//!
//! This module provides high-level git operations for working with repositories,
//! including cloning, fetching, committing, and querying repository state.
//!
//! ## Features
//!
//! - Clone and initialize repositories
//! - Stage files and create commits
//! - Push, pull, and fetch from remotes
//! - Query status, branches, tags, and diffs
//! - Reset and checkout operations
//! - Automatic safe.directory configuration
//!
//! ## Examples
//!
//! ### Cloning a repository
//!
//! ```rust,no_run
//! use xx::git::{clone, CloneOptions};
//!
//! # fn main() -> xx::XXResult<()> {
//! // Clone with default options (shallow clone of default branch)
//! let repo = clone("https://github.com/rust-lang/rust", "/tmp/rust", &CloneOptions::default())?;
//!
//! // Clone a specific branch
//! let options = CloneOptions::default().branch("stable");
//! let repo = clone("https://github.com/rust-lang/rust", "/tmp/rust-stable", &options)?;
//! # Ok(())
//! # }
//! ```
//!
//! ### Working with existing repositories
//!
//! ```rust,no_run
//! use xx::git::Git;
//! use std::path::PathBuf;
//!
//! # fn main() -> xx::XXResult<()> {
//! let git = Git::new(PathBuf::from("/path/to/repo"));
//!
//! // Check if directory is a git repository
//! if git.is_repo() {
//!     // Get current branch name
//!     let branch = git.current_branch()?;
//!     println!("Current branch: {}", branch);
//!
//!     // Get current commit SHA
//!     let sha = git.current_sha()?;
//!     println!("Current SHA: {}", sha);
//!
//!     // Get remote URL
//!     if let Some(url) = git.get_remote_url() {
//!         println!("Remote URL: {}", url);
//!     }
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ### Making commits
//!
//! ```rust,no_run
//! use xx::git::Git;
//! use std::path::PathBuf;
//!
//! # fn main() -> xx::XXResult<()> {
//! let git = Git::new(PathBuf::from("/path/to/repo"));
//!
//! // Stage specific files
//! git.add(&["src/main.rs", "Cargo.toml"])?;
//!
//! // Or stage all changes
//! git.add_all()?;
//!
//! // Create a commit
//! git.commit("feat: add new feature")?;
//!
//! // Push to remote
//! git.push()?;
//! # Ok(())
//! # }
//! ```
//!
//! ### Checking repository status
//!
//! ```rust,no_run
//! use xx::git::Git;
//! use std::path::PathBuf;
//!
//! # fn main() -> xx::XXResult<()> {
//! let git = Git::new(PathBuf::from("/path/to/repo"));
//!
//! let status = git.status()?;
//! println!("Staged files: {:?}", status.staged);
//! println!("Modified files: {:?}", status.modified);
//! println!("Untracked files: {:?}", status.untracked);
//! # Ok(())
//! # }
//! ```

use std::{
    path::{Path, PathBuf},
    vec,
};

use duct::{Expression, cmd};
use miette::{Result, miette};

use crate::{XXError, XXResult, file};

/// Status of a file in the git repository
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileStatus {
    /// File is staged for commit (added to index)
    Staged,
    /// File has been modified but not staged
    Modified,
    /// File is not tracked by git
    Untracked,
    /// File has been deleted
    Deleted,
    /// File has been renamed
    Renamed,
    /// File has been copied
    Copied,
    /// File has merge conflicts
    Conflicted,
}

/// Represents a file with its status in the repository
#[derive(Debug, Clone)]
pub struct StatusEntry {
    /// The path of the file relative to the repository root
    pub path: PathBuf,
    /// The status of the file
    pub status: FileStatus,
    /// For renamed files, the original path
    pub original_path: Option<PathBuf>,
}

/// Repository status containing categorized file lists
#[derive(Debug, Clone, Default)]
pub struct Status {
    /// Files staged for commit
    pub staged: Vec<StatusEntry>,
    /// Files modified but not staged
    pub modified: Vec<StatusEntry>,
    /// Untracked files
    pub untracked: Vec<StatusEntry>,
    /// Deleted files
    pub deleted: Vec<StatusEntry>,
    /// Files with merge conflicts
    pub conflicted: Vec<StatusEntry>,
}

impl Status {
    /// Returns true if the working directory is clean (no changes)
    pub fn is_clean(&self) -> bool {
        self.staged.is_empty()
            && self.modified.is_empty()
            && self.untracked.is_empty()
            && self.deleted.is_empty()
            && self.conflicted.is_empty()
    }

    /// Returns true if there are staged changes ready to commit
    pub fn has_staged(&self) -> bool {
        !self.staged.is_empty()
    }
}

/// Information about a git diff
#[derive(Debug, Clone, Default)]
pub struct DiffStat {
    /// Number of files changed
    pub files_changed: usize,
    /// Number of insertions
    pub insertions: usize,
    /// Number of deletions
    pub deletions: usize,
}

/// Represents a git branch
#[derive(Debug, Clone)]
pub struct Branch {
    /// Branch name
    pub name: String,
    /// Whether this is the current branch
    pub is_current: bool,
    /// The commit SHA the branch points to
    pub sha: Option<String>,
}

/// Represents a git tag
#[derive(Debug, Clone)]
pub struct Tag {
    /// Tag name
    pub name: String,
    /// The commit SHA the tag points to
    pub sha: Option<String>,
}

/// A git repository handle
pub struct Git {
    /// The directory containing the git repository
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
            &format!("+{gitref}:{gitref}"),
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

    /// Stage files for commit
    ///
    /// # Arguments
    ///
    /// * `paths` - Paths to stage, relative to the repository root
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use xx::git::Git;
    /// use std::path::PathBuf;
    ///
    /// let git = Git::new(PathBuf::from("/path/to/repo"));
    /// git.add(&["src/main.rs", "Cargo.toml"]).unwrap();
    /// ```
    pub fn add<P: AsRef<Path>>(&self, paths: &[P]) -> XXResult<()> {
        let path_strs: Vec<String> = paths
            .iter()
            .map(|p| p.as_ref().to_string_lossy().to_string())
            .collect();

        let safe = format!("safe.directory={}", self.dir.display());
        let mut args = vec![
            "-C".to_string(),
            self.dir.to_string_lossy().to_string(),
            "-c".to_string(),
            safe,
            "add".to_string(),
        ];
        args.extend(path_strs);

        let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        cmd("git", &args_refs)
            .run()
            .map_err(|err| XXError::GitError(err, self.dir.clone()))?;
        Ok(())
    }

    /// Stage all changes (including untracked files)
    ///
    /// Equivalent to `git add -A`
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use xx::git::Git;
    /// use std::path::PathBuf;
    ///
    /// let git = Git::new(PathBuf::from("/path/to/repo"));
    /// git.add_all().unwrap();
    /// ```
    pub fn add_all(&self) -> XXResult<()> {
        git_cmd!(&self.dir, "add", "-A")
            .run()
            .map_err(|err| XXError::GitError(err, self.dir.clone()))?;
        Ok(())
    }

    /// Create a commit with the given message
    ///
    /// # Arguments
    ///
    /// * `message` - The commit message
    ///
    /// # Returns
    ///
    /// The SHA of the created commit
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use xx::git::Git;
    /// use std::path::PathBuf;
    ///
    /// let git = Git::new(PathBuf::from("/path/to/repo"));
    /// git.add_all().unwrap();
    /// let sha = git.commit("feat: add new feature").unwrap();
    /// ```
    pub fn commit(&self, message: &str) -> XXResult<String> {
        git_cmd!(&self.dir, "commit", "-m", message)
            .run()
            .map_err(|err| XXError::GitError(err, self.dir.clone()))?;
        self.current_sha()
    }

    /// Create a commit, allowing empty commits
    ///
    /// # Arguments
    ///
    /// * `message` - The commit message
    ///
    /// # Returns
    ///
    /// The SHA of the created commit
    pub fn commit_allow_empty(&self, message: &str) -> XXResult<String> {
        git_cmd!(&self.dir, "commit", "--allow-empty", "-m", message)
            .run()
            .map_err(|err| XXError::GitError(err, self.dir.clone()))?;
        self.current_sha()
    }

    /// Push commits to the remote repository
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use xx::git::Git;
    /// use std::path::PathBuf;
    ///
    /// let git = Git::new(PathBuf::from("/path/to/repo"));
    /// git.push().unwrap();
    /// ```
    pub fn push(&self) -> XXResult<()> {
        git_cmd!(&self.dir, "push")
            .run()
            .map_err(|err| XXError::GitError(err, self.dir.clone()))?;
        Ok(())
    }

    /// Push commits to a specific remote and branch
    ///
    /// # Arguments
    ///
    /// * `remote` - The remote name (e.g., "origin")
    /// * `branch` - The branch name
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use xx::git::Git;
    /// use std::path::PathBuf;
    ///
    /// let git = Git::new(PathBuf::from("/path/to/repo"));
    /// git.push_to("origin", "main").unwrap();
    /// ```
    pub fn push_to(&self, remote: &str, branch: &str) -> XXResult<()> {
        git_cmd!(&self.dir, "push", remote, branch)
            .run()
            .map_err(|err| XXError::GitError(err, self.dir.clone()))?;
        Ok(())
    }

    /// Push and set upstream for the current branch
    ///
    /// # Arguments
    ///
    /// * `remote` - The remote name (e.g., "origin")
    pub fn push_set_upstream(&self, remote: &str) -> XXResult<()> {
        let branch = self.current_branch()?;
        git_cmd!(&self.dir, "push", "-u", remote, &branch)
            .run()
            .map_err(|err| XXError::GitError(err, self.dir.clone()))?;
        Ok(())
    }

    /// Pull changes from the remote repository
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use xx::git::Git;
    /// use std::path::PathBuf;
    ///
    /// let git = Git::new(PathBuf::from("/path/to/repo"));
    /// git.pull().unwrap();
    /// ```
    pub fn pull(&self) -> XXResult<()> {
        git_cmd!(&self.dir, "pull")
            .run()
            .map_err(|err| XXError::GitError(err, self.dir.clone()))?;
        Ok(())
    }

    /// Pull changes from a specific remote and branch
    ///
    /// # Arguments
    ///
    /// * `remote` - The remote name (e.g., "origin")
    /// * `branch` - The branch name
    pub fn pull_from(&self, remote: &str, branch: &str) -> XXResult<()> {
        git_cmd!(&self.dir, "pull", remote, branch)
            .run()
            .map_err(|err| XXError::GitError(err, self.dir.clone()))?;
        Ok(())
    }

    /// Fetch changes from the remote repository
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use xx::git::Git;
    /// use std::path::PathBuf;
    ///
    /// let git = Git::new(PathBuf::from("/path/to/repo"));
    /// git.fetch().unwrap();
    /// ```
    pub fn fetch(&self) -> XXResult<()> {
        git_cmd!(&self.dir, "fetch")
            .run()
            .map_err(|err| XXError::GitError(err, self.dir.clone()))?;
        Ok(())
    }

    /// Fetch changes from a specific remote
    ///
    /// # Arguments
    ///
    /// * `remote` - The remote name (e.g., "origin")
    pub fn fetch_remote(&self, remote: &str) -> XXResult<()> {
        git_cmd!(&self.dir, "fetch", remote)
            .run()
            .map_err(|err| XXError::GitError(err, self.dir.clone()))?;
        Ok(())
    }

    /// Fetch all remotes and prune deleted branches
    pub fn fetch_all(&self) -> XXResult<()> {
        git_cmd!(&self.dir, "fetch", "--all", "--prune")
            .run()
            .map_err(|err| XXError::GitError(err, self.dir.clone()))?;
        Ok(())
    }

    /// Checkout a branch or commit
    ///
    /// # Arguments
    ///
    /// * `ref_name` - The branch name, tag, or commit SHA to checkout
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use xx::git::Git;
    /// use std::path::PathBuf;
    ///
    /// let git = Git::new(PathBuf::from("/path/to/repo"));
    /// git.checkout("main").unwrap();
    /// ```
    pub fn checkout(&self, ref_name: &str) -> XXResult<()> {
        git_cmd!(&self.dir, "checkout", ref_name)
            .run()
            .map_err(|err| XXError::GitError(err, self.dir.clone()))?;
        Ok(())
    }

    /// Create and checkout a new branch
    ///
    /// # Arguments
    ///
    /// * `branch` - The name for the new branch
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use xx::git::Git;
    /// use std::path::PathBuf;
    ///
    /// let git = Git::new(PathBuf::from("/path/to/repo"));
    /// git.checkout_new_branch("feature/my-feature").unwrap();
    /// ```
    pub fn checkout_new_branch(&self, branch: &str) -> XXResult<()> {
        git_cmd!(&self.dir, "checkout", "-b", branch)
            .run()
            .map_err(|err| XXError::GitError(err, self.dir.clone()))?;
        Ok(())
    }

    /// Get the repository status
    ///
    /// # Returns
    ///
    /// A `Status` struct containing categorized file lists
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use xx::git::Git;
    /// use std::path::PathBuf;
    ///
    /// let git = Git::new(PathBuf::from("/path/to/repo"));
    /// let status = git.status().unwrap();
    /// if status.is_clean() {
    ///     println!("Working directory is clean");
    /// }
    /// ```
    pub fn status(&self) -> XXResult<Status> {
        let output = git_cmd!(&self.dir, "status", "--porcelain=v1", "-z")
            .read()
            .map_err(|err| XXError::GitError(err, self.dir.clone()))?;

        let mut status = Status::default();

        // Parse porcelain v1 format with NUL separators
        // Format: XY PATH\0 or XY ORIG_PATH\0PATH\0 for renames
        let parts: Vec<&str> = output.split('\0').collect();
        let mut i = 0;

        while i < parts.len() {
            let part = parts[i];
            if part.len() < 3 {
                i += 1;
                continue;
            }

            let index_status = part.chars().next().unwrap_or(' ');
            let worktree_status = part.chars().nth(1).unwrap_or(' ');
            let path = PathBuf::from(&part[3..]);

            // Handle renames which have an extra path component
            let original_path = if index_status == 'R' || index_status == 'C' {
                i += 1;
                if i < parts.len() {
                    Some(PathBuf::from(parts[i]))
                } else {
                    None
                }
            } else {
                None
            };

            // Categorize based on status codes
            match (index_status, worktree_status) {
                ('?', '?') => {
                    status.untracked.push(StatusEntry {
                        path,
                        status: FileStatus::Untracked,
                        original_path: None,
                    });
                }
                ('U', _) | (_, 'U') | ('A', 'A') | ('D', 'D') => {
                    status.conflicted.push(StatusEntry {
                        path,
                        status: FileStatus::Conflicted,
                        original_path: None,
                    });
                }
                (idx, wt) => {
                    // Staged changes (index status)
                    match idx {
                        'A' | 'M' | 'T' => {
                            status.staged.push(StatusEntry {
                                path: path.clone(),
                                status: FileStatus::Staged,
                                original_path: None,
                            });
                        }
                        'D' => {
                            status.staged.push(StatusEntry {
                                path: path.clone(),
                                status: FileStatus::Deleted,
                                original_path: None,
                            });
                        }
                        'R' => {
                            status.staged.push(StatusEntry {
                                path: path.clone(),
                                status: FileStatus::Renamed,
                                original_path: original_path.clone(),
                            });
                        }
                        'C' => {
                            status.staged.push(StatusEntry {
                                path: path.clone(),
                                status: FileStatus::Copied,
                                original_path: original_path.clone(),
                            });
                        }
                        _ => {}
                    }

                    // Worktree changes (not staged)
                    match wt {
                        'M' | 'T' => {
                            status.modified.push(StatusEntry {
                                path: path.clone(),
                                status: FileStatus::Modified,
                                original_path: None,
                            });
                        }
                        'D' => {
                            status.deleted.push(StatusEntry {
                                path,
                                status: FileStatus::Deleted,
                                original_path: None,
                            });
                        }
                        _ => {}
                    }
                }
            }

            i += 1;
        }

        Ok(status)
    }

    /// Get diff statistics between two refs or for uncommitted changes
    ///
    /// # Arguments
    ///
    /// * `base` - Optional base ref (if None, compares against HEAD)
    /// * `target` - Optional target ref (if None, compares working directory)
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use xx::git::Git;
    /// use std::path::PathBuf;
    ///
    /// let git = Git::new(PathBuf::from("/path/to/repo"));
    ///
    /// // Get diff stats for uncommitted changes
    /// let stats = git.diff_stat(None, None).unwrap();
    ///
    /// // Compare two branches
    /// let stats = git.diff_stat(Some("main"), Some("feature")).unwrap();
    /// ```
    pub fn diff_stat(&self, base: Option<&str>, target: Option<&str>) -> XXResult<DiffStat> {
        let output = match (base, target) {
            (Some(b), Some(t)) => git_cmd!(&self.dir, "diff", "--shortstat", b, t).read(),
            (Some(b), None) => git_cmd!(&self.dir, "diff", "--shortstat", b).read(),
            (None, Some(t)) => git_cmd!(&self.dir, "diff", "--shortstat", "HEAD", t).read(),
            (None, None) => git_cmd!(&self.dir, "diff", "--shortstat", "HEAD").read(),
        }
        .map_err(|err| XXError::GitError(err, self.dir.clone()))?;

        Ok(parse_diff_stat(&output))
    }

    /// Get diff between staged changes and HEAD
    pub fn diff_staged(&self) -> XXResult<DiffStat> {
        let output = git_cmd!(&self.dir, "diff", "--cached", "--shortstat")
            .read()
            .map_err(|err| XXError::GitError(err, self.dir.clone()))?;

        Ok(parse_diff_stat(&output))
    }

    /// Create a tag
    ///
    /// # Arguments
    ///
    /// * `name` - The tag name
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use xx::git::Git;
    /// use std::path::PathBuf;
    ///
    /// let git = Git::new(PathBuf::from("/path/to/repo"));
    /// git.tag("v1.0.0").unwrap();
    /// ```
    pub fn tag(&self, name: &str) -> XXResult<()> {
        git_cmd!(&self.dir, "tag", name)
            .run()
            .map_err(|err| XXError::GitError(err, self.dir.clone()))?;
        Ok(())
    }

    /// Create an annotated tag with a message
    ///
    /// # Arguments
    ///
    /// * `name` - The tag name
    /// * `message` - The tag message
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use xx::git::Git;
    /// use std::path::PathBuf;
    ///
    /// let git = Git::new(PathBuf::from("/path/to/repo"));
    /// git.tag_annotated("v1.0.0", "Release version 1.0.0").unwrap();
    /// ```
    pub fn tag_annotated(&self, name: &str, message: &str) -> XXResult<()> {
        git_cmd!(&self.dir, "tag", "-a", name, "-m", message)
            .run()
            .map_err(|err| XXError::GitError(err, self.dir.clone()))?;
        Ok(())
    }

    /// Delete a tag
    ///
    /// # Arguments
    ///
    /// * `name` - The tag name to delete
    pub fn tag_delete(&self, name: &str) -> XXResult<()> {
        git_cmd!(&self.dir, "tag", "-d", name)
            .run()
            .map_err(|err| XXError::GitError(err, self.dir.clone()))?;
        Ok(())
    }

    /// List all tags
    ///
    /// # Returns
    ///
    /// A vector of `Tag` structs
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use xx::git::Git;
    /// use std::path::PathBuf;
    ///
    /// let git = Git::new(PathBuf::from("/path/to/repo"));
    /// let tags = git.list_tags().unwrap();
    /// for tag in tags {
    ///     println!("{}: {}", tag.name, tag.sha.unwrap_or_default());
    /// }
    /// ```
    pub fn list_tags(&self) -> XXResult<Vec<Tag>> {
        // Use conditional format to get commit SHA for both lightweight and annotated tags.
        // For annotated tags, %(*objectname:short) gives the commit SHA.
        // For lightweight tags, %(objectname:short) gives the commit SHA directly.
        let output = git_cmd!(
            &self.dir,
            "tag",
            "--format=%(refname:short)\t%(if)%(*objectname:short)%(then)%(*objectname:short)%(else)%(objectname:short)%(end)"
        )
        .read()
        .map_err(|err| XXError::GitError(err, self.dir.clone()))?;

        let tags = output
            .lines()
            .filter(|line| !line.is_empty())
            .filter_map(|line| {
                let parts: Vec<&str> = line.split('\t').collect();
                let name = parts.first()?.to_string();
                // Skip tags with empty names
                if name.is_empty() {
                    return None;
                }
                Some(Tag {
                    name,
                    sha: parts
                        .get(1)
                        .filter(|s| !s.is_empty())
                        .map(|s| s.to_string()),
                })
            })
            .collect();

        Ok(tags)
    }

    /// List all branches
    ///
    /// # Arguments
    ///
    /// * `include_remote` - Whether to include remote branches
    ///
    /// # Returns
    ///
    /// A vector of `Branch` structs
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use xx::git::Git;
    /// use std::path::PathBuf;
    ///
    /// let git = Git::new(PathBuf::from("/path/to/repo"));
    /// let branches = git.list_branches(false).unwrap();
    /// for branch in branches {
    ///     let marker = if branch.is_current { "* " } else { "  " };
    ///     println!("{}{}", marker, branch.name);
    /// }
    /// ```
    pub fn list_branches(&self, include_remote: bool) -> XXResult<Vec<Branch>> {
        let output = if include_remote {
            git_cmd!(
                &self.dir,
                "branch",
                "-a",
                "--format=%(HEAD)\t%(refname:short)\t%(objectname:short)"
            )
            .read()
        } else {
            git_cmd!(
                &self.dir,
                "branch",
                "--format=%(HEAD)\t%(refname:short)\t%(objectname:short)"
            )
            .read()
        }
        .map_err(|err| XXError::GitError(err, self.dir.clone()))?;

        let branches = output
            .lines()
            .filter(|line| !line.is_empty())
            .filter_map(|line| {
                let parts: Vec<&str> = line.split('\t').collect();
                let name = parts.get(1)?.to_string();
                // Skip branches with empty names
                if name.is_empty() {
                    return None;
                }
                Some(Branch {
                    is_current: parts.first() == Some(&"*"),
                    name,
                    sha: parts
                        .get(2)
                        .filter(|s| !s.is_empty())
                        .map(|s| s.to_string()),
                })
            })
            .collect();

        Ok(branches)
    }

    /// Create a new branch (without checking it out)
    ///
    /// # Arguments
    ///
    /// * `name` - The branch name
    pub fn create_branch(&self, name: &str) -> XXResult<()> {
        git_cmd!(&self.dir, "branch", name)
            .run()
            .map_err(|err| XXError::GitError(err, self.dir.clone()))?;
        Ok(())
    }

    /// Delete a branch
    ///
    /// # Arguments
    ///
    /// * `name` - The branch name
    /// * `force` - Force delete even if not fully merged
    pub fn delete_branch(&self, name: &str, force: bool) -> XXResult<()> {
        let flag = if force { "-D" } else { "-d" };
        git_cmd!(&self.dir, "branch", flag, name)
            .run()
            .map_err(|err| XXError::GitError(err, self.dir.clone()))?;
        Ok(())
    }

    /// Reset to a specific ref
    ///
    /// # Arguments
    ///
    /// * `ref_name` - The ref to reset to (commit, branch, tag)
    /// * `mode` - The reset mode (soft, mixed, hard)
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use xx::git::{Git, ResetMode};
    /// use std::path::PathBuf;
    ///
    /// let git = Git::new(PathBuf::from("/path/to/repo"));
    /// git.reset("HEAD~1", ResetMode::Soft).unwrap();
    /// ```
    pub fn reset(&self, ref_name: &str, mode: ResetMode) -> XXResult<()> {
        let mode_flag = match mode {
            ResetMode::Soft => "--soft",
            ResetMode::Mixed => "--mixed",
            ResetMode::Hard => "--hard",
        };
        git_cmd!(&self.dir, "reset", mode_flag, ref_name)
            .run()
            .map_err(|err| XXError::GitError(err, self.dir.clone()))?;
        Ok(())
    }

    /// Stash current changes
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use xx::git::Git;
    /// use std::path::PathBuf;
    ///
    /// let git = Git::new(PathBuf::from("/path/to/repo"));
    /// git.stash().unwrap();
    /// ```
    pub fn stash(&self) -> XXResult<()> {
        git_cmd!(&self.dir, "stash")
            .run()
            .map_err(|err| XXError::GitError(err, self.dir.clone()))?;
        Ok(())
    }

    /// Stash current changes with a message
    pub fn stash_with_message(&self, message: &str) -> XXResult<()> {
        git_cmd!(&self.dir, "stash", "push", "-m", message)
            .run()
            .map_err(|err| XXError::GitError(err, self.dir.clone()))?;
        Ok(())
    }

    /// Pop the most recent stash
    pub fn stash_pop(&self) -> XXResult<()> {
        git_cmd!(&self.dir, "stash", "pop")
            .run()
            .map_err(|err| XXError::GitError(err, self.dir.clone()))?;
        Ok(())
    }

    /// Apply the most recent stash (without removing it)
    pub fn stash_apply(&self) -> XXResult<()> {
        git_cmd!(&self.dir, "stash", "apply")
            .run()
            .map_err(|err| XXError::GitError(err, self.dir.clone()))?;
        Ok(())
    }

    /// List all stashes
    pub fn stash_list(&self) -> XXResult<Vec<String>> {
        let output = git_cmd!(&self.dir, "stash", "list")
            .read()
            .map_err(|err| XXError::GitError(err, self.dir.clone()))?;

        Ok(output.lines().map(|s| s.to_string()).collect())
    }

    /// Get the root directory of the git repository
    pub fn root(&self) -> XXResult<PathBuf> {
        let output = git_cmd!(&self.dir, "rev-parse", "--show-toplevel")
            .read()
            .map_err(|err| XXError::GitError(err, self.dir.clone()))?;

        Ok(PathBuf::from(output.trim()))
    }

    /// Check if a ref exists
    pub fn ref_exists(&self, ref_name: &str) -> bool {
        git_cmd!(&self.dir, "rev-parse", "--verify", ref_name)
            .stderr_null()
            .stdout_null()
            .run()
            .is_ok()
    }

    /// Get the merge base between two refs
    pub fn merge_base(&self, ref1: &str, ref2: &str) -> XXResult<String> {
        let output = git_cmd!(&self.dir, "merge-base", ref1, ref2)
            .read()
            .map_err(|err| XXError::GitError(err, self.dir.clone()))?;

        Ok(output.trim().to_string())
    }

    /// Get the commit log
    ///
    /// # Arguments
    ///
    /// * `count` - Number of commits to return
    ///
    /// # Returns
    ///
    /// A vector of (sha, message) tuples
    pub fn log(&self, count: usize) -> XXResult<Vec<(String, String)>> {
        let output = git_cmd!(&self.dir, "log", &format!("-{}", count), "--format=%H\t%s")
            .read()
            .map_err(|err| XXError::GitError(err, self.dir.clone()))?;

        let commits = output
            .lines()
            .filter(|line| !line.is_empty())
            .map(|line| {
                let parts: Vec<&str> = line.splitn(2, '\t').collect();
                (
                    parts.first().unwrap_or(&"").to_string(),
                    parts.get(1).unwrap_or(&"").to_string(),
                )
            })
            .collect();

        Ok(commits)
    }

    /// Check if the repository has any commits
    pub fn has_commits(&self) -> bool {
        git_cmd!(&self.dir, "rev-parse", "HEAD")
            .stderr_null()
            .stdout_null()
            .run()
            .is_ok()
    }
}

/// Reset mode for git reset operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResetMode {
    /// Keep changes staged
    Soft,
    /// Keep changes but unstaged (default)
    Mixed,
    /// Discard all changes
    Hard,
}

/// Parse diff stat output
fn parse_diff_stat(output: &str) -> DiffStat {
    let mut stat = DiffStat::default();

    let output = output.trim();
    if output.is_empty() {
        return stat;
    }

    // Parse: "X files changed, Y insertions(+), Z deletions(-)"
    for part in output.split(", ") {
        let part = part.trim();
        if part.contains("file")
            && let Some(num) = part.split_whitespace().next()
        {
            stat.files_changed = num.parse().unwrap_or(0);
        } else if part.contains("insertion")
            && let Some(num) = part.split_whitespace().next()
        {
            stat.insertions = num.parse().unwrap_or(0);
        } else if part.contains("deletion")
            && let Some(num) = part.split_whitespace().next()
        {
            stat.deletions = num.parse().unwrap_or(0);
        }
    }

    stat
}

/// Initialize a new git repository
///
/// # Arguments
///
/// * `dir` - The directory to initialize as a git repository
///
/// # Returns
///
/// A `Git` handle to the new repository
///
/// # Example
///
/// ```rust,no_run
/// use xx::git::init;
///
/// # fn main() -> xx::XXResult<()> {
/// let repo = init("/tmp/my-project")?;
/// # Ok(())
/// # }
/// ```
pub fn init<D: AsRef<Path>>(dir: D) -> XXResult<Git> {
    let dir = dir.as_ref().to_path_buf();
    file::mkdirp(&dir)?;
    cmd!("git", "init", &dir)
        .run()
        .map_err(|err| XXError::GitError(err, dir.clone()))?;
    Ok(Git::new(dir))
}

/// Clone a repository from a URL
///
/// # Arguments
///
/// * `url` - The repository URL
/// * `dir` - The directory to clone into
/// * `clone_options` - Options for the clone operation
///
/// # Returns
///
/// A `Git` handle to the cloned repository
pub fn clone<D: AsRef<Path>>(url: &str, dir: D, clone_options: &CloneOptions) -> XXResult<Git> {
    let dir = dir.as_ref().to_path_buf();
    debug!("cloning {} to {}", url, dir.display());
    if let Some(parent) = dir.parent() {
        file::mkdirp(parent)?;
    }
    match get_git_version() {
        Ok(version) => trace!("git version: {version}"),
        Err(err) => warn!("failed to get git version: {err:#}\n Git is required to use mise."),
    }

    let dir_str = dir.to_string_lossy().to_string();
    let mut cmd_args = vec!["clone", "-q", "--depth", "1", &url, &dir_str];

    if let Some(branch) = clone_options.branch.as_ref() {
        cmd_args.push("--branch");
        cmd_args.push(branch);
        cmd_args.push("--single-branch");
        cmd_args.push("-c");
        cmd_args.push("advice.detachedHead=false");
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
        let tmp = tempfile::tempdir().unwrap();
        let git = Git::new(tmp.path().to_path_buf());
        assert!(!git.is_repo());
        assert_eq!(git.get_remote_url(), None);
        assert!(git.current_branch().is_err());
        assert!(git.current_sha().is_err());
        assert!(git.current_sha_short().is_err());
        assert!(git.current_abbrev_ref().is_err());

        let git = clone(
            "https://github.com/jdx/xx",
            &git.dir,
            &CloneOptions::default(),
        )
        .unwrap();
        assert!(git.is_repo());
        assert_eq!(
            git.get_remote_url(),
            Some("https://github.com/jdx/xx".to_string())
        );

        file::remove_dir_all(tmp.path()).unwrap();
    }

    #[test]
    fn test_git_with_options() {
        let tmp = tempfile::tempdir().unwrap();
        let git = Git::new(tmp.path().to_path_buf());
        assert!(!git.is_repo());
        assert_eq!(git.get_remote_url(), None);
        assert!(git.current_branch().is_err());
        assert!(git.current_sha().is_err());
        assert!(git.current_sha_short().is_err());
        assert!(git.current_abbrev_ref().is_err());

        let clone_options = CloneOptions::default().branch("v2.0.0");

        let git = clone("https://github.com/jdx/xx", &git.dir, &clone_options).unwrap();
        assert!(git.is_repo());
        assert!(
            git.current_sha()
                .is_ok_and(|s| s == "e5352617769f0edff7758713d05fff6b6ddf1266")
        );
        assert_eq!(
            git.get_remote_url(),
            Some("https://github.com/jdx/xx".to_string())
        );

        file::remove_dir_all("/tmp/xx").unwrap();
    }

    #[test]
    fn test_git_init() {
        let tmp = tempfile::tempdir().unwrap();
        let repo_path = tmp.path().join("new-repo");

        let git = init(&repo_path).unwrap();
        assert!(git.is_repo());
        assert!(!git.has_commits());

        // Root should return the repo path
        let root = git.root().unwrap();
        assert_eq!(root, repo_path.canonicalize().unwrap());
    }

    #[test]
    fn test_git_add_commit() {
        let tmp = tempfile::tempdir().unwrap();
        let repo_path = tmp.path().join("test-repo");

        let git = init(&repo_path).unwrap();

        // Configure git user for the test
        git_cmd!(&git.dir, "config", "user.email", "test@example.com")
            .run()
            .unwrap();
        git_cmd!(&git.dir, "config", "user.name", "Test User")
            .run()
            .unwrap();

        // Create a file and commit it
        let test_file = repo_path.join("test.txt");
        file::write(&test_file, "hello world").unwrap();

        git.add(&[&test_file]).unwrap();

        let status = git.status().unwrap();
        assert!(status.has_staged());
        assert_eq!(status.staged.len(), 1);
        assert_eq!(status.staged[0].path, PathBuf::from("test.txt"));

        let sha = git.commit("Initial commit").unwrap();
        assert!(!sha.is_empty());
        assert!(git.has_commits());

        // Status should be clean after commit
        let status = git.status().unwrap();
        assert!(status.is_clean());

        // Verify log
        let log = git.log(1).unwrap();
        assert_eq!(log.len(), 1);
        assert_eq!(log[0].1, "Initial commit");
    }

    #[test]
    fn test_git_add_all() {
        let tmp = tempfile::tempdir().unwrap();
        let repo_path = tmp.path().join("test-repo");

        let git = init(&repo_path).unwrap();

        // Configure git user for the test
        git_cmd!(&git.dir, "config", "user.email", "test@example.com")
            .run()
            .unwrap();
        git_cmd!(&git.dir, "config", "user.name", "Test User")
            .run()
            .unwrap();

        // Create multiple files
        file::write(repo_path.join("file1.txt"), "content1").unwrap();
        file::write(repo_path.join("file2.txt"), "content2").unwrap();
        file::mkdirp(repo_path.join("subdir")).unwrap();
        file::write(repo_path.join("subdir/file3.txt"), "content3").unwrap();

        // Check untracked files
        let status = git.status().unwrap();
        assert_eq!(status.untracked.len(), 3);

        // Add all
        git.add_all().unwrap();

        let status = git.status().unwrap();
        assert!(status.untracked.is_empty());
        assert_eq!(status.staged.len(), 3);
    }

    #[test]
    fn test_git_status_modified() {
        let tmp = tempfile::tempdir().unwrap();
        let repo_path = tmp.path().join("test-repo");

        let git = init(&repo_path).unwrap();

        // Configure git user for the test
        git_cmd!(&git.dir, "config", "user.email", "test@example.com")
            .run()
            .unwrap();
        git_cmd!(&git.dir, "config", "user.name", "Test User")
            .run()
            .unwrap();

        // Create and commit a file
        let test_file = repo_path.join("test.txt");
        file::write(&test_file, "initial content").unwrap();
        git.add_all().unwrap();
        git.commit("Initial commit").unwrap();

        // Modify the file
        file::write(&test_file, "modified content").unwrap();

        let status = git.status().unwrap();
        assert!(!status.is_clean());
        assert_eq!(status.modified.len(), 1);
        assert_eq!(status.modified[0].path, PathBuf::from("test.txt"));
    }

    #[test]
    fn test_git_branches() {
        let tmp = tempfile::tempdir().unwrap();
        let repo_path = tmp.path().join("test-repo");

        let git = init(&repo_path).unwrap();

        // Configure git user for the test
        git_cmd!(&git.dir, "config", "user.email", "test@example.com")
            .run()
            .unwrap();
        git_cmd!(&git.dir, "config", "user.name", "Test User")
            .run()
            .unwrap();

        // Need a commit to create branches
        file::write(repo_path.join("test.txt"), "content").unwrap();
        git.add_all().unwrap();
        git.commit("Initial commit").unwrap();

        // Create a new branch
        git.create_branch("feature").unwrap();

        let branches = git.list_branches(false).unwrap();
        assert_eq!(branches.len(), 2);

        let current = branches.iter().find(|b| b.is_current).unwrap();
        assert!(current.name == "master" || current.name == "main");

        let feature = branches.iter().find(|b| b.name == "feature").unwrap();
        assert!(!feature.is_current);

        // Checkout and verify
        git.checkout("feature").unwrap();
        let current_branch = git.current_branch().unwrap();
        assert_eq!(current_branch, "feature");

        // Delete branch (need to checkout another branch first)
        git.checkout("master")
            .or_else(|_| git.checkout("main"))
            .unwrap();
        git.delete_branch("feature", false).unwrap();

        let branches = git.list_branches(false).unwrap();
        assert_eq!(branches.len(), 1);
    }

    #[test]
    fn test_git_checkout_new_branch() {
        let tmp = tempfile::tempdir().unwrap();
        let repo_path = tmp.path().join("test-repo");

        let git = init(&repo_path).unwrap();

        // Configure git user for the test
        git_cmd!(&git.dir, "config", "user.email", "test@example.com")
            .run()
            .unwrap();
        git_cmd!(&git.dir, "config", "user.name", "Test User")
            .run()
            .unwrap();

        // Need a commit first
        file::write(repo_path.join("test.txt"), "content").unwrap();
        git.add_all().unwrap();
        git.commit("Initial commit").unwrap();

        // Create and checkout new branch in one step
        git.checkout_new_branch("feature/test").unwrap();

        let current_branch = git.current_branch().unwrap();
        assert_eq!(current_branch, "feature/test");
    }

    #[test]
    fn test_git_tags() {
        let tmp = tempfile::tempdir().unwrap();
        let repo_path = tmp.path().join("test-repo");

        let git = init(&repo_path).unwrap();

        // Configure git user for the test
        git_cmd!(&git.dir, "config", "user.email", "test@example.com")
            .run()
            .unwrap();
        git_cmd!(&git.dir, "config", "user.name", "Test User")
            .run()
            .unwrap();

        // Need a commit first
        file::write(repo_path.join("test.txt"), "content").unwrap();
        git.add_all().unwrap();
        git.commit("Initial commit").unwrap();

        // Create tags
        git.tag("v1.0.0").unwrap();
        git.tag_annotated("v1.1.0", "Release 1.1.0").unwrap();

        let tags = git.list_tags().unwrap();
        assert_eq!(tags.len(), 2);

        let tag_names: Vec<&str> = tags.iter().map(|t| t.name.as_str()).collect();
        assert!(tag_names.contains(&"v1.0.0"));
        assert!(tag_names.contains(&"v1.1.0"));

        // Delete a tag
        git.tag_delete("v1.0.0").unwrap();

        let tags = git.list_tags().unwrap();
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].name, "v1.1.0");
    }

    #[test]
    fn test_git_reset() {
        let tmp = tempfile::tempdir().unwrap();
        let repo_path = tmp.path().join("test-repo");

        let git = init(&repo_path).unwrap();

        // Configure git user for the test
        git_cmd!(&git.dir, "config", "user.email", "test@example.com")
            .run()
            .unwrap();
        git_cmd!(&git.dir, "config", "user.name", "Test User")
            .run()
            .unwrap();

        // Create two commits
        file::write(repo_path.join("test.txt"), "first").unwrap();
        git.add_all().unwrap();
        let first_sha = git.commit("First commit").unwrap();

        file::write(repo_path.join("test.txt"), "second").unwrap();
        git.add_all().unwrap();
        git.commit("Second commit").unwrap();

        // Reset soft to first commit
        git.reset(&first_sha, ResetMode::Soft).unwrap();

        // Changes should be staged
        let status = git.status().unwrap();
        assert!(status.has_staged());

        // Reset mixed (default) to first commit
        git.reset(&first_sha, ResetMode::Mixed).unwrap();

        // Changes should be unstaged
        let status = git.status().unwrap();
        assert!(!status.has_staged());
        assert!(!status.modified.is_empty());
    }

    #[test]
    fn test_git_stash() {
        let tmp = tempfile::tempdir().unwrap();
        let repo_path = tmp.path().join("test-repo");

        let git = init(&repo_path).unwrap();

        // Configure git user for the test
        git_cmd!(&git.dir, "config", "user.email", "test@example.com")
            .run()
            .unwrap();
        git_cmd!(&git.dir, "config", "user.name", "Test User")
            .run()
            .unwrap();

        // Create initial commit
        file::write(repo_path.join("test.txt"), "initial").unwrap();
        git.add_all().unwrap();
        git.commit("Initial commit").unwrap();

        // Make changes
        file::write(repo_path.join("test.txt"), "modified").unwrap();

        // Stash changes
        git.stash_with_message("WIP: test changes").unwrap();

        // Working directory should be clean
        let status = git.status().unwrap();
        assert!(status.is_clean());

        // File should have original content
        let content = file::read_to_string(repo_path.join("test.txt")).unwrap();
        assert_eq!(content, "initial");

        // List stashes
        let stashes = git.stash_list().unwrap();
        assert_eq!(stashes.len(), 1);
        assert!(stashes[0].contains("WIP: test changes"));

        // Pop stash
        git.stash_pop().unwrap();

        // Changes should be back
        let content = file::read_to_string(repo_path.join("test.txt")).unwrap();
        assert_eq!(content, "modified");
    }

    #[test]
    fn test_git_diff_stat() {
        let tmp = tempfile::tempdir().unwrap();
        let repo_path = tmp.path().join("test-repo");

        let git = init(&repo_path).unwrap();

        // Configure git user for the test
        git_cmd!(&git.dir, "config", "user.email", "test@example.com")
            .run()
            .unwrap();
        git_cmd!(&git.dir, "config", "user.name", "Test User")
            .run()
            .unwrap();

        // Create initial commit
        file::write(repo_path.join("test.txt"), "line1\nline2\nline3\n").unwrap();
        git.add_all().unwrap();
        git.commit("Initial commit").unwrap();

        // Modify file
        file::write(
            repo_path.join("test.txt"),
            "line1\nmodified\nline3\nnew line\n",
        )
        .unwrap();

        // Check diff stat
        let stat = git.diff_stat(None, None).unwrap();
        assert_eq!(stat.files_changed, 1);
        assert!(stat.insertions > 0 || stat.deletions > 0);
    }

    #[test]
    fn test_git_ref_exists() {
        let tmp = tempfile::tempdir().unwrap();
        let repo_path = tmp.path().join("test-repo");

        let git = init(&repo_path).unwrap();

        // Configure git user for the test
        git_cmd!(&git.dir, "config", "user.email", "test@example.com")
            .run()
            .unwrap();
        git_cmd!(&git.dir, "config", "user.name", "Test User")
            .run()
            .unwrap();

        // No refs yet
        assert!(!git.ref_exists("HEAD"));

        // Create a commit
        file::write(repo_path.join("test.txt"), "content").unwrap();
        git.add_all().unwrap();
        git.commit("Initial commit").unwrap();

        // HEAD should exist now
        assert!(git.ref_exists("HEAD"));
        assert!(!git.ref_exists("nonexistent-ref"));
    }

    #[test]
    fn test_git_log() {
        let tmp = tempfile::tempdir().unwrap();
        let repo_path = tmp.path().join("test-repo");

        let git = init(&repo_path).unwrap();

        // Configure git user for the test
        git_cmd!(&git.dir, "config", "user.email", "test@example.com")
            .run()
            .unwrap();
        git_cmd!(&git.dir, "config", "user.name", "Test User")
            .run()
            .unwrap();

        // Create multiple commits
        file::write(repo_path.join("test.txt"), "1").unwrap();
        git.add_all().unwrap();
        git.commit("First commit").unwrap();

        file::write(repo_path.join("test.txt"), "2").unwrap();
        git.add_all().unwrap();
        git.commit("Second commit").unwrap();

        file::write(repo_path.join("test.txt"), "3").unwrap();
        git.add_all().unwrap();
        git.commit("Third commit").unwrap();

        let log = git.log(2).unwrap();
        assert_eq!(log.len(), 2);
        assert_eq!(log[0].1, "Third commit");
        assert_eq!(log[1].1, "Second commit");
    }

    #[test]
    fn test_git_merge_base() {
        let tmp = tempfile::tempdir().unwrap();
        let repo_path = tmp.path().join("test-repo");

        let git = init(&repo_path).unwrap();

        // Configure git user for the test
        git_cmd!(&git.dir, "config", "user.email", "test@example.com")
            .run()
            .unwrap();
        git_cmd!(&git.dir, "config", "user.name", "Test User")
            .run()
            .unwrap();

        // Create base commit
        file::write(repo_path.join("test.txt"), "base").unwrap();
        git.add_all().unwrap();
        let base_sha = git.commit("Base commit").unwrap();

        // Create branch and add a commit
        git.checkout_new_branch("feature").unwrap();
        file::write(repo_path.join("feature.txt"), "feature").unwrap();
        git.add_all().unwrap();
        git.commit("Feature commit").unwrap();

        // Go back to main and add a commit
        git.checkout("master")
            .or_else(|_| git.checkout("main"))
            .unwrap();
        file::write(repo_path.join("main.txt"), "main").unwrap();
        git.add_all().unwrap();
        git.commit("Main commit").unwrap();

        // Find merge base
        let main_branch = git.current_branch().unwrap();
        let merge_base = git.merge_base(&main_branch, "feature").unwrap();
        assert_eq!(merge_base, base_sha);
    }

    #[test]
    fn test_parse_diff_stat() {
        // Test parsing various diff stat formats
        let stat = parse_diff_stat(" 3 files changed, 10 insertions(+), 5 deletions(-)");
        assert_eq!(stat.files_changed, 3);
        assert_eq!(stat.insertions, 10);
        assert_eq!(stat.deletions, 5);

        let stat = parse_diff_stat(" 1 file changed, 1 insertion(+)");
        assert_eq!(stat.files_changed, 1);
        assert_eq!(stat.insertions, 1);
        assert_eq!(stat.deletions, 0);

        let stat = parse_diff_stat("");
        assert_eq!(stat.files_changed, 0);
        assert_eq!(stat.insertions, 0);
        assert_eq!(stat.deletions, 0);
    }

    #[test]
    fn test_status_is_clean() {
        let status = Status::default();
        assert!(status.is_clean());

        let mut status = Status::default();
        status.modified.push(StatusEntry {
            path: PathBuf::from("test.txt"),
            status: FileStatus::Modified,
            original_path: None,
        });
        assert!(!status.is_clean());
    }

    // ========================================================================
    // Error case tests
    // ========================================================================

    #[test]
    fn test_clone_invalid_url() {
        let tmp = tempfile::tempdir().unwrap();
        let result = clone(
            "https://github.com/nonexistent-user-12345/nonexistent-repo-67890",
            tmp.path(),
            &CloneOptions::default(),
        );
        // Should fail because the repo doesn't exist
        assert!(result.is_err());
    }

    #[test]
    fn test_clone_invalid_branch() {
        let tmp = tempfile::tempdir().unwrap();
        let clone_options = CloneOptions::default().branch("nonexistent-branch-12345");
        let result = clone("https://github.com/jdx/xx", tmp.path(), &clone_options);
        // Should fail because the branch doesn't exist
        assert!(result.is_err());
    }

    #[test]
    fn test_current_branch_not_a_repo() {
        let tmp = tempfile::tempdir().unwrap();
        let git = Git::new(tmp.path().to_path_buf());
        // Should fail because it's not a git repo
        let result = git.current_branch();
        assert!(result.is_err());
    }

    #[test]
    fn test_current_sha_not_a_repo() {
        let tmp = tempfile::tempdir().unwrap();
        let git = Git::new(tmp.path().to_path_buf());
        // Should fail because it's not a git repo
        let result = git.current_sha();
        assert!(result.is_err());
    }

    #[test]
    fn test_current_sha_short_not_a_repo() {
        let tmp = tempfile::tempdir().unwrap();
        let git = Git::new(tmp.path().to_path_buf());
        // Should fail because it's not a git repo
        let result = git.current_sha_short();
        assert!(result.is_err());
    }

    #[test]
    fn test_current_abbrev_ref_not_a_repo() {
        let tmp = tempfile::tempdir().unwrap();
        let git = Git::new(tmp.path().to_path_buf());
        // Should fail because it's not a git repo
        let result = git.current_abbrev_ref();
        assert!(result.is_err());
    }

    #[test]
    fn test_get_remote_url_not_a_repo() {
        let tmp = tempfile::tempdir().unwrap();
        let git = Git::new(tmp.path().to_path_buf());
        // Should return None because it's not a git repo
        assert_eq!(git.get_remote_url(), None);
    }

    #[test]
    fn test_get_remote_url_nonexistent_dir() {
        let git = Git::new(PathBuf::from("/nonexistent/path/12345"));
        // Should return None because the directory doesn't exist
        assert_eq!(git.get_remote_url(), None);
    }

    // ========================================================================
    // Utility function tests
    // ========================================================================

    #[test]
    fn test_split_url_and_ref_with_ref() {
        let (url, gitref) = Git::split_url_and_ref("https://github.com/user/repo#v1.0.0");
        assert_eq!(url, "https://github.com/user/repo");
        assert_eq!(gitref, Some("v1.0.0".to_string()));
    }

    #[test]
    fn test_split_url_and_ref_without_ref() {
        let (url, gitref) = Git::split_url_and_ref("https://github.com/user/repo");
        assert_eq!(url, "https://github.com/user/repo");
        assert_eq!(gitref, None);
    }

    #[test]
    fn test_split_url_and_ref_with_branch_name() {
        let (url, gitref) = Git::split_url_and_ref("https://github.com/user/repo#feature/branch");
        assert_eq!(url, "https://github.com/user/repo");
        assert_eq!(gitref, Some("feature/branch".to_string()));
    }

    #[test]
    fn test_is_repo_true() {
        let tmp = tempfile::tempdir().unwrap();
        // Clone a repo first
        let git = clone(
            "https://github.com/jdx/xx",
            tmp.path(),
            &CloneOptions::default(),
        )
        .unwrap();
        assert!(git.is_repo());
    }

    #[test]
    fn test_is_repo_false() {
        let tmp = tempfile::tempdir().unwrap();
        let git = Git::new(tmp.path().to_path_buf());
        assert!(!git.is_repo());
    }

    #[test]
    fn test_is_repo_nonexistent() {
        let git = Git::new(PathBuf::from("/nonexistent/path/12345"));
        assert!(!git.is_repo());
    }

    // ========================================================================
    // Update tests
    // ========================================================================

    #[test]
    fn test_update() {
        let tmp = tempfile::tempdir().unwrap();

        // Clone the repo
        let git = clone(
            "https://github.com/jdx/xx",
            tmp.path(),
            &CloneOptions::default(),
        )
        .unwrap();

        // Get initial state
        let initial_sha = git.current_sha().unwrap();

        // Update should succeed (even if no changes)
        let result = git.update(None);
        assert!(result.is_ok());

        let (prev_rev, post_rev) = result.unwrap();
        assert!(!prev_rev.is_empty());
        assert!(!post_rev.is_empty());
        // prev_rev should match what we had before the update
        assert_eq!(prev_rev, initial_sha);
        // post_rev may differ if upstream changed, but should be a valid SHA
        assert_eq!(post_rev.len(), 40); // Full SHA length
    }

    #[test]
    fn test_clone_options_builder() {
        let options = CloneOptions::default();
        assert!(options.branch.is_none());

        let options = CloneOptions::default().branch("main");
        assert_eq!(options.branch, Some("main".to_string()));
    }
}

#[derive(Default)]
pub struct CloneOptions {
    branch: Option<String>,
}

impl CloneOptions {
    pub fn branch(mut self, branch: &str) -> Self {
        self.branch = Some(branch.to_string());
        self
    }
}
