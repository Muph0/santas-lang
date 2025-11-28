use std::{hash::Hash, mem, sync::Arc};

use clap::Parser;
use santa_lang::{logger, runtime::{RunCommand, Runtime}, translate::{TranslationInput, translate}};


mod cli;



fn main() {
    let mut args = cli::Args::parse();

    let level = match args.trace {
        true => log::LevelFilter::Trace,
        false => log::LevelFilter::Info,
    };
    logger::init(level);

    logger::unwrap(args.validate());
    let inputs = mem::take(&mut args.files)
        .into_iter()
        .map(|f| TranslationInput::File(f))
        .collect::<Vec<_>>();

    let unit_res = translate(inputs);

    let unit = logger::unwrap_many(unit_res);
    log::debug!("Parsing ok");

    let mut rt = Runtime::new(&unit);
    match rt.run(RunCommand::RunToEnd) {
        Ok(_) => {},
        Err(e) => log::error!("{e}"),
    }
}
