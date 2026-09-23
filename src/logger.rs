use std::{io::Write, path::PathBuf};

use chrono::offset::Local;
use log::{debug, Level, LevelFilter, Metadata, Record};

struct AppLogger {
    max_level: LevelFilter,
    log_file_path: PathBuf,
}

impl log::Log for AppLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= self.max_level
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            // Write to log File
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .append(true)
                .open(&self.log_file_path)
                .expect("Can't open logfile");

            let local = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
            let s = format!("{} {} - {}\n", &local, record.level(), record.args());
            if let Err(e) = file.write_all(s.as_bytes()) {
                eprintln!("Couldn't write to logfile: {}", e);
                std::process::exit(1);
            }

            // Write to stdio or stderror
            match record.level() {
                Level::Error => {
                    eprintln!("{} - {}", record.level(), record.args());
                    eprintln!("aborting...");
                    std::process::exit(1);
                }
                Level::Info => println!("{}", record.args()),
                _ => println!("{} - {}", record.level(), record.args()),
            }
        }
    }

    fn flush(&self) {}
}

pub fn init(app_name: &str, log_dir: &PathBuf, log_level: u64) {
    let level = match log_level {
        0 => LevelFilter::Info,
        1 => LevelFilter::Debug,
        2 | _ => LevelFilter::Trace,
    };

    let logger = AppLogger {
        log_file_path: log_dir.join(&(app_name.to_string() + ".log")),
        max_level: level,
    };
    match log::set_boxed_logger(std::boxed::Box::new(logger)) {
        Ok(_) => log::set_max_level(level),
        Err(e) => {
            eprintln!("Couldn't initialize logger: {}", e);
            std::process::exit(1);
        }
    }

    match level {
        LevelFilter::Info => debug!("Log mode is set to INFO"),
        LevelFilter::Debug => debug!("Log mode is set to DEBUG"),
        LevelFilter::Trace => debug!("Log mode is set to TRACE"),
        _ => log::error!("Log mode not available"), // make the compiler happy
    }
}
