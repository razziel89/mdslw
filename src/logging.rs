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

use std::time;

use log::{Level, Log, Metadata, Record};

/// Execute a trace log while lazily evaluating closures that yield values to be logged. This macro
/// takes a string literal, followed by closures that will be evaluated lazily, followed by any
/// other possible arguments. The string literal will have to take the argument order into account.
#[macro_export]
macro_rules! trace_log {
    ($fmt_str:literal;; $($args:expr),*) => {
        if log::log_enabled!(log::Level::Trace) {
            log::trace!($fmt_str, $($args),*);
        }
    };
    ($fmt_str:literal; $($closures:expr),*; $($args:expr),*) => {
        if log::log_enabled!(log::Level::Trace) {
            log::trace!($fmt_str, $($closures()),*, $($args),*);
        }
    };
}

const SELF_MODULE_NAME: &str = env!("CARGO_PKG_NAME");

pub struct Logger {
    starttime: time::Instant,
    level: Level,
    module_name: String,
    module_prefix: String,
}

impl Logger {
    pub fn new(log_level: u8) -> Self {
        let level = match log_level {
            0 => Level::Warn,
            1 => Level::Info,
            2 => Level::Debug,
            _ => Level::Trace,
        };
        Self {
            level,
            starttime: time::Instant::now(),
            module_name: SELF_MODULE_NAME.to_string(),
            module_prefix: format!("{}::", SELF_MODULE_NAME),
        }
    }
}

impl Log for Logger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= self.level
    }

    fn log(&self, record: &Record) {
        if let Some(msg) = self.format_message(record) {
            eprintln!("{}", msg);
        }
    }

    fn flush(&self) {}
}

impl Logger {
    fn format_log_location(&self, record: &Record) -> String {
        let module = record.module_path_static().unwrap_or("");

        if module == self.module_name || module.starts_with(&self.module_prefix) {
            let file = record.file_static().unwrap_or("");
            let line = record.line().unwrap_or(0);
            format!("{}:{}:{}", module, file, line)
        } else {
            module.to_owned()
        }
    }

    fn format_message(&self, record: &Record) -> Option<String> {
        if self.enabled(record.metadata()) {
            let elapsed = self.starttime.elapsed();
            let elapsed_secs = elapsed.as_secs();
            let elapsed_millis = elapsed.subsec_millis();

            Some(format!(
                "{}: {}s{}ms {}: {}",
                record.level(),
                elapsed_secs,
                elapsed_millis,
                self.format_log_location(record),
                record.args()
            ))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use anyhow::{Error, Result};

    #[test]
    fn new_logger() {
        let logger0 = Logger::new(0);
        assert_eq!(logger0.level, Level::Warn);

        let logger1 = Logger::new(1);
        assert_eq!(logger1.level, Level::Info);

        let logger2 = Logger::new(2);
        assert_eq!(logger2.level, Level::Debug);

        let logger3 = Logger::new(3);
        assert_eq!(logger3.level, Level::Trace);
    }

    #[test]
    fn logger_enabled() {
        let logger = Logger::new(0);
        assert_eq!(logger.level, Level::Warn);

        let metadata_err = Metadata::builder().level(Level::Error).build();
        let metadata_debug = Metadata::builder().level(Level::Debug).build();

        assert!(logger.enabled(&metadata_err));
        assert!(!logger.enabled(&metadata_debug));
    }

    #[test]
    fn logging_a_message_from_own_module() -> Result<()> {
        let args = format_args!("some thing");
        let metadata = Metadata::builder().level(Level::Error).build();
        let record = Record::builder()
            .metadata(metadata)
            .module_path_static(Some("mdslw::test"))
            .file_static(Some("test_file"))
            .args(args)
            .build();

        let logger = Logger::new(0);
        let msg = logger
            .format_message(&record)
            .ok_or(Error::msg("cannot build message"))?;

        // Check beginning and end because the test might take longer than 1ms, which would fail
        // it.
        assert!(msg.starts_with("ERROR: 0s"), "incorrect start: {}", msg);
        assert!(
            msg.ends_with("ms mdslw::test:test_file:0: some thing"),
            "incorrect end: {}",
            msg
        );

        Ok(())
    }

    #[test]
    fn logging_a_message_from_another_module() -> Result<()> {
        let args = format_args!("some thing");
        let metadata = Metadata::builder().level(Level::Error).build();
        let record = Record::builder()
            .metadata(metadata)
            .module_path_static(Some("some::other::module"))
            .file_static(Some("test_file"))
            .args(args)
            .build();

        let logger = Logger::new(0);
        let msg = logger
            .format_message(&record)
            .ok_or(Error::msg("cannot build message"))?;

        // Check beginning and end because the test might take longer than 1ms, which would fail
        // it.
        assert!(msg.starts_with("ERROR: 0s"), "incorrect start: {}", msg);
        assert!(
            msg.ends_with("ms some::other::module: some thing"),
            "incorrect end: {}",
            msg
        );

        Ok(())
    }
}
