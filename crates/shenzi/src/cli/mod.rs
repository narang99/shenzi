
use anyhow::{bail, Result};
use clap::Parser;

mod build;


#[derive(clap::Subcommand, Debug)]
pub enum Commands {
    Build {
        /// manifest file path, use `-` to take input from stdio (or when you are piping)
        manifest: String,
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
                Commands::Build { manifest } => {
                    build::run(&manifest)?;
                }
            }
        }
    };
    Ok(())
}