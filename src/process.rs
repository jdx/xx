//! Process execution utilities
//!
//! This module provides convenient functions and builders for executing external processes
//! with better ergonomics than the standard library's `std::process` module.
//!
//! ## Features
//!
//! - Simple shell command execution with `sh()`
//! - Builder pattern for complex command construction
//! - Automatic stdout/stderr capture options
//! - Enhanced error messages that include the command that failed
//!
//! ## Examples
//!
//! ### Simple shell command
//!
//! ```rust,no_run
//! use xx::process;
//!
//! # fn main() -> xx::XXResult<()> {
//! // Run a shell command and get stdout as a string
//! let output = process::sh("echo hello world")?;
//! assert_eq!(output.trim(), "hello world");
//! # Ok(())
//! # }
//! ```
//!
//! ### Command builder
//!
//! ```rust,no_run
//! use xx::process;
//!
//! # fn main() -> xx::XXResult<()> {
//! // Build a command with arguments
//! let output = process::cmd("git", &["status", "--short"])
//!     .read()?;
//!
//! // Capture stdout and stderr separately
//! let result = process::cmd("make", &["test"])
//!     .stdout_capture()
//!     .stderr_capture()
//!     .run()?;
//! # Ok(())
//! # }
//! ```

use std::collections::HashMap;
use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::sync::Arc;
use std::thread;
use std::{ffi::OsString, fmt, io, process::Output};

type LineHandler = dyn Fn(&str) + Send + Sync + 'static;

use duct::IntoExecutablePath;

use crate::{XXError, XXResult};

pub fn sh(script: &str) -> XXResult<String> {
    let output = Command::new("sh")
        .arg("-c")
        .arg(script)
        .stdin(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .output()
        .map_err(|err| XXError::ProcessError(err, format!("sh -c {script}")))?;

    check_status(output.status)
        .map_err(|err| XXError::ProcessError(err, format!("sh -c {script}")))?;
    let stdout = String::from_utf8(output.stdout).expect("stdout is not utf-8");
    Ok(stdout)
}

pub fn check_status(status: ExitStatus) -> io::Result<()> {
    if status.success() {
        return Ok(());
    }
    let msg = if let Some(code) = status.code() {
        format!("exited with code {code}")
    } else {
        "terminated by signal".to_string()
    };
    Err(io::Error::other(msg))
}

#[derive(Default)]
pub struct XXExpression {
    program: OsString,
    args: Vec<OsString>,
    stdout_capture: bool,
    stderr_capture: bool,
    stdout_handler: Option<Arc<LineHandler>>,
    stderr_handler: Option<Arc<LineHandler>>,
    env_vars: HashMap<OsString, OsString>,
    env_clear: bool,
    cwd: Option<PathBuf>,
    stdin_data: Option<Vec<u8>>,
    unchecked: bool,
}

pub fn cmd<T, U>(program: T, args: U) -> XXExpression
where
    T: IntoExecutablePath,
    U: IntoIterator,
    U::Item: Into<OsString>,
{
    let program = program.to_executable();
    let args = args.into_iter().map(|arg| arg.into()).collect::<Vec<_>>();
    XXExpression {
        program,
        args,
        ..Default::default()
    }
}

impl XXExpression {
    pub fn stdout_capture(mut self) -> Self {
        self.stdout_capture = true;
        self
    }

    pub fn stderr_capture(mut self) -> Self {
        self.stderr_capture = true;
        self
    }

    pub fn arg(mut self, arg: impl Into<OsString>) -> Self {
        self.args.push(arg.into());
        self
    }

    pub fn args(mut self, args: impl IntoIterator<Item = impl Into<OsString>>) -> Self {
        self.args.extend(args.into_iter().map(|arg| arg.into()));
        self
    }

    pub fn run(&self) -> XXResult<Output> {
        debug!("$ {self}");
        if self.stdout_handler.is_some() || self.stderr_handler.is_some() {
            // Inline streaming behavior previously provided by `run_streaming`
            let mut cmd = Command::new(&self.program);
            cmd.args(&self.args)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());

            // Handle stdin
            if let Some(stdin_data) = &self.stdin_data {
                cmd.stdin(Stdio::piped());
                // We'll write stdin data after spawning
                let _ = stdin_data; // suppress unused warning, handled below
            } else {
                cmd.stdin(Stdio::inherit());
            }

            // Handle environment
            if self.env_clear {
                cmd.env_clear();
            }
            for (k, v) in &self.env_vars {
                cmd.env(k, v);
            }

            // Handle working directory
            if let Some(cwd) = &self.cwd {
                cmd.current_dir(cwd);
            }

            let mut child = cmd
                .spawn()
                .map_err(|err| XXError::ProcessError(err, self.to_string()))?;

            // Write stdin data if provided
            if let Some(stdin_data) = &self.stdin_data
                && let Some(mut stdin) = child.stdin.take()
            {
                use std::io::Write;
                let _ = stdin.write_all(stdin_data);
            }

            let mut stdout = child
                .stdout
                .take()
                .ok_or_else(|| io::Error::other("failed to capture stdout"))
                .map_err(|err| XXError::ProcessError(err, self.to_string()))?;
            let mut stderr = child
                .stderr
                .take()
                .ok_or_else(|| io::Error::other("failed to capture stderr"))
                .map_err(|err| XXError::ProcessError(err, self.to_string()))?;

            let out_h = self.stdout_handler.clone();
            let stdout_handle = thread::spawn(move || {
                let mut reader = io::BufReader::new(&mut stdout);
                let mut line = String::with_capacity(1024);
                loop {
                    line.clear();
                    match reader.read_line(&mut line) {
                        Ok(0) => break,
                        Ok(_) => {
                            if line.ends_with('\n') {
                                line.pop();
                                if line.ends_with('\r') {
                                    line.pop();
                                }
                            } else if line.ends_with('\r') {
                                line.pop();
                            }
                            // this can be removed in rust 1.88
                            #[allow(clippy::collapsible_if)]
                            if !line.is_empty() {
                                if let Some(h) = &out_h {
                                    (h)(&line);
                                }
                            }
                        }
                        Err(_) => break,
                    }
                }
            });

            let err_h = self.stderr_handler.clone();
            let stderr_handle = thread::spawn(move || {
                let mut reader = io::BufReader::new(&mut stderr);
                let mut line = String::with_capacity(1024);
                loop {
                    line.clear();
                    match reader.read_line(&mut line) {
                        Ok(0) => break,
                        Ok(_) => {
                            if line.ends_with('\n') {
                                line.pop();
                                if line.ends_with('\r') {
                                    line.pop();
                                }
                            } else if line.ends_with('\r') {
                                line.pop();
                            }
                            // this can be removed in rust 1.88
                            #[allow(clippy::collapsible_if)]
                            if !line.is_empty() {
                                if let Some(h) = &err_h {
                                    (h)(&line);
                                }
                            }
                        }
                        Err(_) => break,
                    }
                }
            });

            let status = child
                .wait()
                .map_err(|err| XXError::ProcessError(err, self.to_string()))?;

            let _ = stdout_handle.join();
            let _ = stderr_handle.join();

            if !self.unchecked {
                check_status(status).map_err(|err| XXError::ProcessError(err, self.to_string()))?;
            }
            return Ok(Output {
                status,
                stdout: vec![],
                stderr: vec![],
            });
        }
        let expr = self.build_expr();
        expr.run()
            .map_err(|err| XXError::ProcessError(err, self.to_string()))
    }

    pub fn read(&self) -> XXResult<String> {
        debug!("$ {self}");
        if self.stdout_handler.is_some() || self.stderr_handler.is_some() {
            let mut cmd = Command::new(&self.program);
            cmd.args(&self.args)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());

            // Handle stdin
            if self.stdin_data.is_some() {
                cmd.stdin(Stdio::piped());
            } else {
                cmd.stdin(Stdio::inherit());
            }

            // Handle environment
            if self.env_clear {
                cmd.env_clear();
            }
            for (k, v) in &self.env_vars {
                cmd.env(k, v);
            }

            // Handle working directory
            if let Some(cwd) = &self.cwd {
                cmd.current_dir(cwd);
            }

            let mut child = cmd
                .spawn()
                .map_err(|err| XXError::ProcessError(err, self.to_string()))?;

            // Write stdin data if provided
            if let Some(stdin_data) = &self.stdin_data
                && let Some(mut stdin) = child.stdin.take()
            {
                use std::io::Write;
                let _ = stdin.write_all(stdin_data);
            }

            let mut stderr = child
                .stderr
                .take()
                .ok_or_else(|| io::Error::other("failed to capture stderr"))
                .map_err(|err| XXError::ProcessError(err, self.to_string()))?;

            // Drain stderr on a background thread, invoking handler if present
            let err_h = self.stderr_handler.clone();
            let stderr_handle = thread::spawn(move || {
                let mut reader = io::BufReader::new(&mut stderr);
                let mut line = String::with_capacity(1024);
                loop {
                    line.clear();
                    match reader.read_line(&mut line) {
                        Ok(0) => break,
                        Ok(_) => {
                            if line.ends_with('\n') {
                                line.pop();
                                if line.ends_with('\r') {
                                    line.pop();
                                }
                            } else if line.ends_with('\r') {
                                line.pop();
                            }
                            // this can be removed in rust 1.88
                            #[allow(clippy::collapsible_if)]
                            if !line.is_empty() {
                                if let Some(h) = &err_h {
                                    (h)(&line);
                                }
                            }
                        }
                        Err(_) => break,
                    }
                }
            });

            // Read stdout line-by-line in the current thread, optionally emitting handler,
            // while reconstructing the full stdout for return
            let mut stdout = child
                .stdout
                .take()
                .ok_or_else(|| io::Error::other("failed to capture stdout"))
                .map_err(|err| XXError::ProcessError(err, self.to_string()))?;
            let out_h = self.stdout_handler.clone();
            let mut reader = io::BufReader::new(&mut stdout);
            let mut acc = String::new();
            let mut line = String::with_capacity(1024);
            loop {
                line.clear();
                match reader.read_line(&mut line) {
                    Ok(0) => break,
                    Ok(_) => {
                        let mut had_nl = false;
                        if line.ends_with('\n') {
                            had_nl = true;
                            line.pop();
                            if line.ends_with('\r') {
                                line.pop();
                            }
                        } else if line.ends_with('\r') {
                            line.pop();
                        }
                        if !line.is_empty() {
                            if let Some(h) = &out_h {
                                (h)(&line);
                            }
                            acc.push_str(&line);
                        }
                        if had_nl {
                            acc.push('\n');
                        }
                    }
                    Err(_) => break,
                }
            }

            let status = child
                .wait()
                .map_err(|err| XXError::ProcessError(err, self.to_string()))?;
            let _ = stderr_handle.join();
            if !self.unchecked {
                check_status(status).map_err(|err| XXError::ProcessError(err, self.to_string()))?;
            }
            // Match duct's `read()` behavior: trim a single trailing newline
            if acc.ends_with('\n') {
                let _ = acc.pop();
            }
            return Ok(acc);
        }
        let expr = self.build_expr();
        expr.read()
            .map_err(|err| XXError::ProcessError(err, self.to_string()))
    }

    // run_streaming removed; streaming logic is now handled inline in `run()`

    /// Register a line-by-line stdout handler. When set, `run()` will stream output lines
    /// to this handler instead of capturing stdout.
    pub fn on_stdout_line<F>(mut self, handler: F) -> Self
    where
        F: Fn(&str) + Send + Sync + 'static,
    {
        self.stdout_handler = Some(Arc::new(handler));
        self
    }

    /// Register a line-by-line stderr handler. When set, `run()` will stream error lines
    /// to this handler instead of capturing stderr.
    pub fn on_stderr_line<F>(mut self, handler: F) -> Self
    where
        F: Fn(&str) + Send + Sync + 'static,
    {
        self.stderr_handler = Some(Arc::new(handler));
        self
    }

    /// Set an environment variable for this command
    /// # Example
    /// ```
    /// use xx::process;
    /// let output = process::cmd("sh", ["-c", "echo $MY_VAR"])
    ///     .env("MY_VAR", "hello")
    ///     .read()
    ///     .unwrap();
    /// assert_eq!(output, "hello");
    /// ```
    pub fn env<K, V>(mut self, key: K, value: V) -> Self
    where
        K: Into<OsString>,
        V: Into<OsString>,
    {
        self.env_vars.insert(key.into(), value.into());
        self
    }

    /// Set multiple environment variables for this command
    /// # Example
    /// ```
    /// use xx::process;
    /// use std::collections::HashMap;
    /// let mut env = HashMap::new();
    /// env.insert("VAR1", "value1");
    /// env.insert("VAR2", "value2");
    /// let output = process::cmd("sh", ["-c", "echo $VAR1 $VAR2"])
    ///     .envs(env)
    ///     .read()
    ///     .unwrap();
    /// assert_eq!(output, "value1 value2");
    /// ```
    pub fn envs<I, K, V>(mut self, vars: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<OsString>,
        V: Into<OsString>,
    {
        for (k, v) in vars {
            self.env_vars.insert(k.into(), v.into());
        }
        self
    }

    /// Clear all environment variables before running (start with empty environment)
    /// # Example
    /// ```
    /// use xx::process;
    /// let output = process::cmd("sh", ["-c", "echo ${PATH:-empty}"])
    ///     .env_clear()
    ///     .env("PATH", "/bin:/usr/bin")
    ///     .read()
    ///     .unwrap();
    /// assert_eq!(output, "/bin:/usr/bin");
    /// ```
    pub fn env_clear(mut self) -> Self {
        self.env_clear = true;
        self
    }

    /// Set the working directory for this command
    /// # Example
    /// ```
    /// use xx::process;
    /// let output = process::cmd("pwd", Vec::<&str>::new())
    ///     .cwd("/tmp")
    ///     .read()
    ///     .unwrap();
    /// assert!(output.contains("tmp"));
    /// ```
    pub fn cwd<P: AsRef<Path>>(mut self, dir: P) -> Self {
        self.cwd = Some(dir.as_ref().to_path_buf());
        self
    }

    /// Provide stdin data as bytes
    /// # Example
    /// ```
    /// use xx::process;
    /// let output = process::cmd("cat", Vec::<&str>::new())
    ///     .stdin_bytes(b"hello world")
    ///     .read()
    ///     .unwrap();
    /// assert_eq!(output, "hello world");
    /// ```
    pub fn stdin_bytes<B: AsRef<[u8]>>(mut self, data: B) -> Self {
        self.stdin_data = Some(data.as_ref().to_vec());
        self
    }

    /// Provide stdin data from a file
    /// # Example
    /// ```no_run
    /// use xx::process;
    /// let output = process::cmd("cat", Vec::<&str>::new())
    ///     .stdin_file("input.txt")
    ///     .unwrap()
    ///     .read()
    ///     .unwrap();
    /// ```
    pub fn stdin_file<P: AsRef<Path>>(mut self, path: P) -> XXResult<Self> {
        let path = path.as_ref();
        let data =
            std::fs::read(path).map_err(|err| XXError::FileError(err, path.to_path_buf()))?;
        self.stdin_data = Some(data);
        Ok(self)
    }

    /// Don't check the exit status (allow non-zero exit codes)
    ///
    /// By default, `run()` and `read()` return an error if the process exits
    /// with a non-zero status. This method disables that check.
    ///
    /// # Example
    /// ```
    /// use xx::process;
    /// // This would normally error because false exits with code 1
    /// let output = process::cmd("false", Vec::<&str>::new())
    ///     .unchecked()
    ///     .run()
    ///     .unwrap();
    /// assert!(!output.status.success());
    /// ```
    pub fn unchecked(mut self) -> Self {
        self.unchecked = true;
        self
    }

    fn build_expr(&self) -> duct::Expression {
        let mut expr = duct::cmd(self.program.clone(), self.args.clone());
        if self.stdout_capture {
            expr = expr.stdout_capture();
        }
        if self.stderr_capture {
            expr = expr.stderr_capture();
        }
        if self.env_clear {
            expr = expr.full_env(self.env_vars.clone());
        } else {
            for (k, v) in &self.env_vars {
                expr = expr.env(k, v);
            }
        }
        if let Some(cwd) = &self.cwd {
            expr = expr.dir(cwd);
        }
        if let Some(stdin_data) = &self.stdin_data {
            expr = expr.stdin_bytes(stdin_data.clone());
        }
        if self.unchecked {
            expr = expr.unchecked();
        }
        expr
    }
}

impl fmt::Display for XXExpression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} {}",
            self.program.to_string_lossy(),
            self.args
                .iter()
                .map(|arg| arg.to_string_lossy())
                .collect::<Vec<_>>()
                .join(" ")
        )
    }
}

mod tests {
    #[allow(unused_imports)]
    use super::*;
    #[allow(unused_imports)]
    use std::sync::{Arc, Mutex};

    #[test]
    fn test_cmd() {
        let expr = cmd("echo", ["hello", "world"]).stdout_capture();
        let output = expr.run().unwrap();
        assert!(output.status.success());
        assert_eq!(output.stdout, b"hello world\n");
    }

    #[test]
    fn test_cmd_read() {
        let expr = cmd("echo", ["hello"]).arg("world").args(["foo", "bar"]);
        let output = expr.read().unwrap();
        assert_eq!(output, "hello world foo bar");
    }

    #[test]
    fn test_line_handlers_capture_stdout_and_stderr_lines() {
        // Use sh to emit interleaved stdout/stderr lines
        let script = r#"
            printf 'o1\n';
            printf 'e1\n' 1>&2;
            printf 'o2\n';
            printf 'e2\n' 1>&2;
        "#;
        let out_lines: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(vec![]));
        let err_lines: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(vec![]));

        let out_clone = out_lines.clone();
        let err_clone = err_lines.clone();

        let output = cmd("sh", ["-c", script])
            .on_stdout_line(move |line| out_clone.lock().unwrap().push(line.to_string()))
            .on_stderr_line(move |line| err_clone.lock().unwrap().push(line.to_string()))
            .run()
            .unwrap();
        assert!(output.status.success());

        let mut out = out_lines.lock().unwrap().clone();
        let mut err = err_lines.lock().unwrap().clone();
        out.sort();
        err.sort();
        assert_eq!(out, vec!["o1", "o2"]);
        assert_eq!(err, vec!["e1", "e2"]);
    }

    #[test]
    fn test_line_handlers_propagate_nonzero_exit() {
        // Emit some output and then exit non-zero
        let script = r#"
            printf 'ok\n';
            printf 'bad\n' 1>&2;
            exit 3;
        "#;
        let res = cmd("sh", ["-c", script])
            .on_stdout_line(|_| {})
            .on_stderr_line(|_| {})
            .run();
        assert!(res.is_err());
        let err = format!("{}", res.unwrap_err());
        assert!(err.contains("sh -c"));
    }

    #[test]
    fn test_line_handlers_handle_partial_last_line() {
        // Emit lines without trailing newline at the end
        let script = r#"
            printf 'a1\n';
            printf 'b1' 1>&2;
        "#;
        let out_lines: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(vec![]));
        let err_lines: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(vec![]));
        let out_clone = out_lines.clone();
        let err_clone = err_lines.clone();
        let output = cmd("sh", ["-c", script])
            .on_stdout_line(move |line| out_clone.lock().unwrap().push(line.to_string()))
            .on_stderr_line(move |line| err_clone.lock().unwrap().push(line.to_string()))
            .run()
            .unwrap();
        assert!(output.status.success());
        assert_eq!(out_lines.lock().unwrap().as_slice(), ["a1"]);
        assert_eq!(err_lines.lock().unwrap().as_slice(), ["b1"]);
    }

    #[test]
    fn test_line_handlers_trim_crlf() {
        // Ensure CRLF endings are normalized before handler invocation
        let script = r#"
            printf 'x1\r\n';
            printf 'y1\r\n' 1>&2;
        "#;
        let out_lines: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(vec![]));
        let err_lines: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(vec![]));
        let out_clone = out_lines.clone();
        let err_clone = err_lines.clone();
        let output = cmd("sh", ["-c", script])
            .on_stdout_line(move |line| out_clone.lock().unwrap().push(line.to_string()))
            .on_stderr_line(move |line| err_clone.lock().unwrap().push(line.to_string()))
            .run()
            .unwrap();
        assert!(output.status.success());
        assert_eq!(out_lines.lock().unwrap().as_slice(), ["x1"]);
        assert_eq!(err_lines.lock().unwrap().as_slice(), ["y1"]);
    }

    #[test]
    fn test_read_with_handlers_returns_full_stdout_and_invokes_handlers() {
        let script = r#"
            printf 'l1\n';
            printf 'l2\n';
        "#;
        let lines: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(vec![]));
        let lines_clone = lines.clone();
        let out = cmd("sh", ["-c", script])
            .on_stdout_line(move |line| lines_clone.lock().unwrap().push(line.to_string()))
            .read()
            .unwrap();
        assert_eq!(out, "l1\nl2");
        assert_eq!(lines.lock().unwrap().as_slice(), ["l1", "l2"]);
    }

    #[test]
    fn test_read_without_handlers_trims_trailing_newline() {
        let script = r#"
            printf 'a\n';
            printf 'b\n';
        "#;
        let out = cmd("sh", ["-c", script]).read().unwrap();
        assert_eq!(out, "a\nb");
    }

    #[test]
    fn test_env() {
        let out = cmd("sh", ["-c", "echo $TEST_VAR"])
            .env("TEST_VAR", "hello")
            .read()
            .unwrap();
        assert_eq!(out, "hello");
    }

    #[test]
    fn test_envs() {
        let mut vars = std::collections::HashMap::new();
        vars.insert("VAR1", "a");
        vars.insert("VAR2", "b");
        let out = cmd("sh", ["-c", "echo $VAR1 $VAR2"])
            .envs(vars)
            .read()
            .unwrap();
        assert_eq!(out, "a b");
    }

    #[test]
    fn test_cwd() {
        let out = cmd("pwd", Vec::<&str>::new()).cwd("/tmp").read().unwrap();
        // Handle macOS /private/tmp symlink
        assert!(out.contains("tmp"));
    }

    #[test]
    fn test_stdin_bytes() {
        let out = cmd("cat", Vec::<&str>::new())
            .stdin_bytes(b"hello stdin")
            .read()
            .unwrap();
        assert_eq!(out, "hello stdin");
    }

    #[test]
    fn test_unchecked() {
        // Without unchecked, this would error
        let output = cmd("false", Vec::<&str>::new()).unchecked().run().unwrap();
        assert!(!output.status.success());
    }

    #[test]
    fn test_unchecked_read() {
        // Without unchecked, this would error
        let output = cmd("sh", ["-c", "echo hello; exit 1"])
            .unchecked()
            .read()
            .unwrap();
        assert_eq!(output, "hello");
    }
}
