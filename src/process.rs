use std::process::{Command, ExitStatus};
use std::{ffi::OsString, fmt, io, process::Output};

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
    Err(io::Error::new(io::ErrorKind::Other, msg))
}

pub struct XXExpression {
    expr: duct::Expression,
    program: OsString,
    args: Vec<OsString>,
}

pub fn cmd<T, U>(program: T, args: U) -> XXExpression
where
    T: IntoExecutablePath,
    U: IntoIterator,
    U::Item: Into<OsString>,
{
    let program = program.to_executable();
    let args = args.into_iter().map(|arg| arg.into()).collect::<Vec<_>>();
    let expr = duct::cmd(program.clone(), args.clone());
    XXExpression {
        expr,
        program,
        args,
    }
}

impl XXExpression {
    pub fn stdout_capture(self) -> Self {
        let expr = self.expr.stdout_capture();
        Self {
            expr,
            program: self.program,
            args: self.args,
        }
    }

    pub fn stderr_capture(self) -> Self {
        let expr = self.expr.stderr_capture();
        Self {
            expr,
            program: self.program,
            args: self.args,
        }
    }

    pub fn run(&self) -> XXResult<Output> {
        debug!("$ {}", self);
        self.expr
            .run()
            .map_err(|err| XXError::ProcessError(err, self.to_string()))
    }

    pub fn read(&self) -> XXResult<String> {
        debug!("$ {}", self);
        self.expr
            .read()
            .map_err(|err| XXError::ProcessError(err, self.to_string()))
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

    #[test]
    fn test_cmd() {
        let expr = cmd("echo", ["hello", "world"]).stdout_capture();
        let output = expr.run().unwrap();
        assert!(output.status.success());
        assert_eq!(output.stdout, b"hello world\n");
    }

    #[test]
    fn test_cmd_read() {
        let expr = cmd("echo", ["hello", "world"]);
        let output = expr.read().unwrap();
        assert_eq!(output, "hello world");
    }
}
