/* An opinionated line wrapper for markdown files.
Copyright (C) 2023  Torsten Long

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU General Public License for more details.

You should have received a copy of the GNU General Public License
along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/

use anyhow::{Context, Error, Result};
use std::io::Write;
use std::process::{Command, Stdio};

pub fn upstream_formatter(
    upstream: &str,
    file_content: String,
    workdir: std::path::PathBuf,
) -> Result<String> {
    let split_upstream = upstream.split_whitespace().collect::<Vec<_>>();

    // Interpret an empty directory as the current directory.
    let upstream_workdir = if workdir.components().count() == 0 {
        ".".into()
    } else {
        workdir
    };
    log::debug!(
        "running upstream executable in directory: {}",
        upstream_workdir.to_string_lossy()
    );

    let cmd = split_upstream
        .first()
        .ok_or(Error::msg("must specify an upstream command"))
        .context("failed to determine upstream auto-formatter command")?;
    log::debug!("using upstream executable {}", cmd);

    let args = split_upstream[1..].to_owned();
    log::debug!("using upstream arguments {:?}", args);

    let mut process = Command::new(cmd)
        .args(&args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .current_dir(upstream_workdir)
        .spawn()
        .context("failed to spawn upstream auto-formatter")?;

    let mut stdin = process
        .stdin
        .take()
        .context("failed to acquire stdin of upstream auto-formatter")?;

    // Write to stdin in a separate thread. Is there really is no other way to do that? Calling
    // "expect" here is not a problem because, if the process panics, we receive an error.
    std::thread::spawn(move || {
        stdin
            .write_all(file_content.as_bytes())
            .expect("failed to write stdin to upstream auto-formatter")
    });

    let output = process
        .wait_with_output()
        .context("failed to wait for output of upstream auto-formatter")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if output.status.success() {
        Ok(stdout.to_string())
    } else {
        Err(Error::msg(format!(
            "failed to read stdout of upstream auto-formatter \"{}\". Stderr follows: \n\n{}",
            upstream, stderr,
        )))
    }
}

pub struct Pager {
    stdin: std::process::ChildStdin,
}

impl Pager {
    pub fn send(&mut self, s: &str) -> Result<()> {
        self.stdin
            .write_all(s.as_bytes())
            .context("sending text to pager's stdin")
    }
}

pub fn downstream_pager(pager: &str, workdir: std::path::PathBuf) -> Result<Pager> {
    let split_pager = pager.split_whitespace().collect::<Vec<_>>();

    // Interpret an empty directory as the current directory.
    let pager_workdir = if workdir.components().count() == 0 {
        ".".into()
    } else {
        workdir
    };
    log::debug!(
        "running downstream pager in directory: {}",
        pager_workdir.to_string_lossy()
    );

    let cmd = split_pager
        .first()
        .ok_or(Error::msg("must specify a pager command"))
        .context("failed to determine downstream pager command")?;
    log::debug!("using pager executable {}", cmd);

    let args = split_pager[1..].to_owned();
    log::debug!("using pager arguments {:?}", args);

    let mut process = Command::new(cmd)
        .args(&args)
        .stdin(Stdio::piped())
        .current_dir(pager_workdir)
        .spawn()
        .context("failed to spawn downstream pager")?;

    let stdin = process
        .stdin
        .take()
        .context("failed to acquire stdin of the downstream pager")?;

    Ok(Pager { stdin })
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn can_call_simple_executable_with_stdio_handling() {
        let input = String::from("some text");
        let piped = upstream_formatter(&String::from("cat"), input.clone(), ".".into()).unwrap();
        assert_eq!(input, piped);
    }

    #[test]
    fn can_call_with_args() {
        let piped =
            upstream_formatter(&String::from("echo some text"), String::new(), ".".into()).unwrap();
        assert_eq!("some text\n", piped);
    }

    #[test]
    fn need_to_provide_command() {
        let result = upstream_formatter("", String::new(), ".".into());
        assert!(result.is_err());
    }

    #[test]
    fn unknown_executable_fails() {
        let result = upstream_formatter(
            &String::from("executable-unknown-asdf"),
            String::new(),
            ".".into(),
        );
        assert!(result.is_err());
    }
}
