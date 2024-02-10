use std::io;
use std::process::{Command, ExitStatus};

use crate::{XXError, XXResult};

pub fn sh(script: &str) -> XXResult<String> {
    let output = Command::new("sh")
        .arg("-c")
        .arg(script)
        .output()
        .map_err(|err| XXError::ProcessError(err, format!("sh -c {script}")))?;

    check_status(output.status)
        .map_err(|err| XXError::ProcessError(err, format!("sh -c {script}")))?;
    let stdout = String::from_utf8(output.stdout).expect("stdout is not utf-8");
    Ok(stdout)
}

fn check_status(status: ExitStatus) -> io::Result<()> {
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
