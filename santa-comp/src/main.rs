use std::{hash::Hash, mem, sync::Arc};

use clap::Parser;
use santa_lang::{logger, translate::{TranslationInput, translate}};


mod cli;



fn main() {
    logger::init(log::LevelFilter::Trace);
    let mut args = cli::Args::parse();
    logger::unwrap(args.validate());

    let inputs = mem::take(&mut args.files)
        .into_iter()
        .map(|f| TranslationInput::File(f))
        .collect::<Vec<_>>();

    let unit_res = translate(inputs);

    let unit = logger::unwrap_many(unit_res);
}
