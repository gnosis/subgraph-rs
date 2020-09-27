//! Module containing logger implementation.

use crate::{ffi::string::AscString, sys};
use log::{Level, LevelFilter, Log, Metadata, Record};

/// The main logger implementation for the `log` facade crate.
pub struct Logger;

impl Log for Logger {
    fn enabled(&self, _: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        let level = match record.level() {
            Level::Error => ERROR,
            Level::Warn => WARNING,
            Level::Info => INFO,
            Level::Debug | Level::Trace => DEBUG,
        };
        let message = AscString::new(record.args().to_string());

        unsafe {
            sys::log::log(level, &message);
        }
    }

    fn flush(&self) {}
}

/// Initialize logging.
pub fn init() {
    static LOGGER: Logger = Logger;
    log::set_logger(&LOGGER).unwrap();
    log::set_max_level(LevelFilter::Debug);
}

const ERROR: u32 = 1;
const WARNING: u32 = 2;
const INFO: u32 = 3;
const DEBUG: u32 = 4;
