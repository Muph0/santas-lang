use clap::Parser;

mod cli;
mod log;


fn main() {
    log::init();
    let args = cli::Args::parse();


}
