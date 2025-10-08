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

use std::collections::VecDeque;
use std::fmt;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::Mutex;

use anyhow::{Context, Error, Result};

use crate::trace_log;

pub struct Upstream {
    cmd: String,
    args: Vec<String>,
}

impl Upstream {
    pub fn from_cfg(command: &str, args: &str, sep: &str) -> Result<Self> {
        let mut split_args = if sep.is_empty() {
            args.split_whitespace()
                .map(String::from)
                .collect::<VecDeque<_>>()
        } else {
            args.split(sep).map(String::from).collect::<VecDeque<_>>()
        };
        let cmd = if !command.is_empty() {
            command.to_string()
        } else {
            split_args
                .pop_front()
                .ok_or_else(|| {
                    Error::msg(format!(
                        "Failed to extract upstream command from arguments '{}'.",
                        args
                    ))
                })?
                .to_string()
        };
        let result = Self {
            cmd,
            args: split_args.into_iter().collect::<Vec<_>>(),
        };
        log::debug!("using upstream formatter {}", result);
        Ok(result)
    }
}

impl fmt::Display for Upstream {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "'{}' '{}'", self.cmd, self.args.join("' '"))
    }
}

pub fn upstream_formatter(
    upstream: &Upstream,
    file_content: String,
    workdir: &Path,
) -> Result<String> {
    let fallback_workdir = PathBuf::from(".");
    let workdir = if workdir.components().count() == 0 {
        &fallback_workdir
    } else {
        workdir
    };
    log::debug!(
        "running upstream executable in directory: {}",
        workdir.to_string_lossy()
    );

    let mut process = Command::new(&upstream.cmd)
        .args(&upstream.args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .current_dir(workdir)
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
    stdin: Option<std::process::ChildStdin>,
    process: std::process::Child,
}

impl Pager {
    pub fn send(&mut self, s: &str) -> Result<()> {
        log::debug!("sending {} bytes to downstream pager's stdin", s.len());
        trace_log!("message sent to downstream pager: {}", s);
        if let Some(ref mut stdin) = self.stdin {
            stdin
                .write_all(s.as_bytes())
                .context("failed to send text to pager's stdin")
        } else {
            unreachable!("cannot send to closed stdin of downstream pager");
        }
    }
}

impl Drop for Pager {
    fn drop(&mut self) {
        {
            log::debug!("closing stdin of downstream pager");
            // Have pager's stdin go out of scope before waiting for the pager
            // process. This should not be needed according to the docs of
            // "wait", because supposedly that stdin is closed before waiting to
            // prevent deadlocks, but it seems to be needed, because there is a
            // deadlock without this.
            let _ = self.stdin.take();
        }
        self.process
            .wait()
            .expect("failed to wait for pager to finish");
    }
}

/// If to_null is set, the output of this pager will be directed to /dev/null.
/// That is used solely for testing.
fn downstream_pager(pager: &str, workdir: std::path::PathBuf, to_null: bool) -> Result<Pager> {
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

    let mut process_cfg = Command::new(cmd);
    process_cfg
        .args(&args)
        .stdin(Stdio::piped())
        .current_dir(pager_workdir);
    if to_null {
        process_cfg.stdout(Stdio::null());
    }
    let mut process = process_cfg
        .spawn()
        .context("failed to spawn downstream pager")?;

    let stdin = process
        .stdin
        .take()
        .context("failed to acquire stdin of the downstream pager")?;

    Ok(Pager {
        stdin: Some(stdin),
        process,
    })
}

/// A helper to ensure that text written to stdout is not mangled due to parallelisation.
pub enum ParallelPrinter {
    // First bool indicates whether there had been a failure writing to the pager.
    Paged(Mutex<(bool, Pager)>),
    Direct(Mutex<()>),
}

impl ParallelPrinter {
    pub fn new(pager: &Option<String>) -> Result<Self> {
        if let Some(pager) = pager {
            let downstream = downstream_pager(pager, PathBuf::from("."), false)?;
            Ok(Self::Paged(Mutex::new((false, downstream))))
        } else {
            Ok(Self::Direct(Mutex::new(())))
        }
    }

    pub fn println(&self, text: &str) {
        match self {
            Self::Paged(mutex) => {
                let mut result = mutex
                    .lock()
                    .expect("failed to lock mutex due to previous panic");
                // We do not retry sending to the pager in case sending failed once.
                if !result.0 {
                    if let Err(err) = result.1.send(text) {
                        log::error!("{:?}", err);
                        result.0 = true;
                    }
                }
            }
            Self::Direct(mutex) => {
                // Assigning to keep the lock. The lock is lifted once the binding is dropped.
                let _lock = mutex
                    .lock()
                    .expect("failed to lock mutex due to previous panic");
                println!("{}", text);
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn can_call_simple_executable_with_stdio_handling() -> Result<()> {
        let input = String::from("some text");
        let piped = upstream_formatter(
            &Upstream::from_cfg("", "cat", " ")?,
            input.clone(),
            &PathBuf::from("."),
        )
        .unwrap();
        assert_eq!(input, piped);
        Ok(())
    }

    #[test]
    fn can_call_with_args() -> Result<()> {
        let piped = upstream_formatter(
            &Upstream::from_cfg("echo", "some text", "")?,
            String::new(),
            &PathBuf::from("."),
        )
        .unwrap();
        assert_eq!("some text\n", piped);
        Ok(())
    }

    #[test]
    fn need_to_provide_command() -> Result<()> {
        let result = upstream_formatter(
            &Upstream::from_cfg("", "", " ")?,
            String::new(),
            &PathBuf::from("."),
        );
        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn unknown_executable_fails() -> Result<()> {
        let result = upstream_formatter(
            &Upstream::from_cfg("", "executable-unknown-asdf", " ")?,
            String::new(),
            &PathBuf::from("."),
        );
        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn can_call_pager_with_args() -> Result<()> {
        let mut pager = downstream_pager(&String::from("cat -"), ".".into(), true)?;
        pager.send("some text")?;
        Ok(())
    }

    #[test]
    fn need_to_provide_pager_command() {
        let result = downstream_pager("", ".".into(), true);
        assert!(result.is_err());
    }

    #[test]
    fn unknown_pager_executable_fails() {
        let result = downstream_pager(&String::from("executable-unknown-asdf"), ".".into(), true);
        assert!(result.is_err());
    }
}
