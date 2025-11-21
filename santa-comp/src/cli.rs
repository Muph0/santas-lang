use std::path::PathBuf;

use clap::Parser;

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[arg(short, long, help = "Run the the files directly, without compiling.")]
    pub interpret: bool,

    pub files: Vec<PathBuf>,
}

impl Args {
    pub fn validate(&self) -> Result<(), String> {
        match () {
            _ if self.files.is_empty() => Err("No files.".into()),
            _ if self.interpret == false => {
                Err("For now, only interpreter mode is supported. (see --help)".into())
            }
            _ => Ok(()),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn interpret_required() {
        let mut args = vec!["santac", "file1.sasm"];

        let args1 = Args::parse_from(&args);
        args.push("--interpret");
        let args2 = Args::parse_from(&args);

        args1.validate().unwrap_err();
        args2.validate().unwrap();
    }
}
