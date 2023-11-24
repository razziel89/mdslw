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

const SELF_MODULE_NAME: &str = env!("PACKAGE_NAME");

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
        if self.enabled(record.metadata()) {
            let elapsed = self.starttime.elapsed();
            let elapsed_secs = elapsed.as_secs();
            let elapsed_millis = elapsed.subsec_millis();

            eprintln!(
                "{}: {}s{}ms {}: {}",
                record.level(),
                elapsed_secs,
                elapsed_millis,
                self.format_log_location(record),
                record.args()
            );
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
}
