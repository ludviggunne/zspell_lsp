use clap::Parser;
use log::{error, Level, LevelFilter, Log, Metadata, Record, SetLoggerError};
use server::Server;
use std::process;

mod lexer;
mod server;

#[derive(Parser, Debug)]
#[command(about, long_about = None)]
pub struct Options {
    /// Specify affix file.
    #[arg(short, long, default_value_t = String::from("./index.aff"))]
    affix: String,

    /// Specify dictionary file.
    #[arg(short, long, default_value_t = String::from("./index.dic"))]
    dictionary: String,
}

struct Logger {}
static LOGGER: Logger = Logger {};

impl Logger {
    fn init() -> Result<(), SetLoggerError> {
        log::set_logger(&LOGGER)
            .map(|()| log::set_max_level(LevelFilter::Debug))
    }
}

impl Log for Logger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Debug
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            eprintln!("[{}]: {}", record.level(), record.args());
        }
    }

    fn flush(&self) {}
}

fn main() {
    Logger::init().unwrap();

    let options = Options::parse();
    _ = options;

    let affix_str = match std::fs::read_to_string(&options.affix) {
        Ok(affix) => affix,
        Err(e) => {
            error!("Unable to open affix file {}: {}", options.affix, e);
            process::exit(-1);
        }
    };

    let dict_str = match std::fs::read_to_string(&options.dictionary) {
        Ok(dict) => dict,
        Err(e) => {
            error!(
                "Unable to open dictionary file {}: {}",
                options.dictionary, e
            );
            process::exit(-1);
        }
    };

    let dict = match zspell::builder()
        .config_str(&affix_str)
        .dict_str(&dict_str)
        .build()
    {
        Ok(dict) => dict,
        Err(err) => {
            error!("Unable to create dictionary: {}", err);
            process::exit(-1);
        }
    };

    let mut server = match Server::new(dict) {
        Ok(server) => server,
        Err(e) => {
            error!("Couldn't initialize server: {}", e);
            process::exit(-1);
        }
    };

    if let Err(err) = server.run() {
        error!("Server error: {}", err);
        process::exit(-1);
    }
}
