use std::{borrow::Cow, path::PathBuf};

use clap::Parser;

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[arg(
        short,
        long,
        help = "Run the the files in the interpreter, without compiling."
    )]
    interpret: bool,

    files: Vec<PathBuf>,
}

pub enum IsValid {
    Yes,
    No { reason: String },
}

impl Args {
    pub fn validate(&self) -> IsValid {
        if self.interpret == false {
            return IsValid::No {
                reason: "For now, only interpret mode is supported.".into(),
            };
        }
        IsValid::Yes
    }
}

impl IsValid {
    /// Do nothing if valid, otherwise log error and close.
    pub fn unwrap(&self) {
        if let IsValid::No { reason } = self {
            log::error!("{}", reason);
            std::process::exit(2);
        }
    }
    pub fn as_bool(&self) -> bool {
        match self {
            IsValid::Yes => true,
            _ => false,
        }
    }
}

mod test {
    use super::*;

    #[test]
    fn interpret_required() {
        let mut args = vec![];

        let args1 = Args::parse_from(&args);
        args.push("--interpret");
        let args2 = Args::parse_from(&args);

        assert!(args1.validate().is_err());
        assert!(args2.validate().is_ok());
    }
}
