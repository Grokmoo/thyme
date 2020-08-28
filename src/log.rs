/*!
A minimal logger for use with Thyme.

Logs all messages to standard output.
!*/

use log::{Level, Log, Record, Metadata, SetLoggerError};

struct SimpleLogger {
    level: Level,
}

impl Log for SimpleLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= self.level
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) { return; }

        let target = if !record.target().is_empty() {
            record.target()
        } else {
            record.module_path().unwrap_or_default()
        };

        println!("{:<5} <{}> {}", record.level().to_string(), target, record.args());
    }

    fn flush(&self) { }
}

/// Initiales the logger at the specified log level.  This should only be called once per program.
pub fn init(level: Level) -> Result<(), SetLoggerError> {
    let logger = Box::new(SimpleLogger { level });

    log::set_logger(Box::leak(logger))?;
    log::set_max_level(level.to_level_filter());
    Ok(())
}

/// Initializes the logger at the `Trace` level.  This should only be called once per program.
pub fn init_all() -> Result<(), SetLoggerError> {
    init(Level::Trace)
}