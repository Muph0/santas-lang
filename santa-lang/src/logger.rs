#![allow(dead_code)]

use log::{Level, Metadata, Record};
use std::{fmt::{Display, Write as _}, sync::Once};

struct SimpleLogger;

const RST: &str = "\x1b[0m";
const ERR: &str = "\x1b[1;31m";
const WAR: &str = "\x1b[1;33m";
const INF: &str = "\x1b[1;32m";
const DBG: &str = "\x1b[35m";
const TRA: &str = "\x1b[36m";

pub fn unwrap<T, E: Display>(r: Result<T, E>) -> T {
    match r {
        Err(e) => {
            log::error!("{e}");
            std::process::exit(1);
        }
        Ok(t) => t,
    }
}
pub fn unwrap_many<T, E: IntoIterator<Item = impl Display>>(r: Result<T, E>) -> T {
    match r {
        Err(es) => {
            for e in es {
                log::error!("{e}");
            }
            std::process::exit(1);
        }
        Ok(t) => t,
    }
}

impl log::Log for SimpleLogger {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true // accept everything
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let mut out = String::new();

            match record.level() {
                Level::Error => write!(&mut out, "{ERR}error{RST}"),
                Level::Warn => write!(&mut out, "{WAR}warn{RST}"),
                Level::Info => write!(&mut out, "{INF}info{RST}"),
                Level::Debug => write!(&mut out, "{DBG}debug{RST}"),
                Level::Trace => write!(&mut out, "{TRA}trace{RST}"),
            }
            .unwrap();

            write!(&mut out, ": {}", record.args()).unwrap();
            println!("{out}");
        }
    }

    fn flush(&self) {}
}

// A static instance required by `log::set_logger`
static LOGGER: SimpleLogger = SimpleLogger;
static INIT: Once = Once::new();

pub fn init(level: log::LevelFilter) {
    INIT.call_once(|| {
        log::set_logger(&LOGGER)
            .map(|()| log::set_max_level(level))
            .expect("Failed to set logger");
    });
}
