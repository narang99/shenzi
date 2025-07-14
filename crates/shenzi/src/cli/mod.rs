
use anyhow::{bail, Result};
use clap::Parser;

mod build;


#[derive(clap::Subcommand, Debug)]
pub enum Commands {
    Build {
        /// manifest file path, use `-` to take input from stdio (or when you are piping)
        manifest: String,

        /// Skip validation of warnings, you should generally not skip this, although warnings validation can take a long time.
        /// You can skip this if you are running shenzi multiple times and are confident that there were no warnings in the first invocation.
        #[arg(long, default_value_t = false)]
        skip_warning_checks: bool,
    },
}

#[derive(Debug, clap::Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}


pub fn run() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        None => {
            bail!("empty command not allowed");
        },
        Some(cmd) => {
            match cmd {
                Commands::Build { manifest, skip_warning_checks} => {
                    build::run(&manifest, skip_warning_checks)?;
                }
            }
        }
    };
    Ok(())
}