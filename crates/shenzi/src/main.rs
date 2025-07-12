use std::process::exit;



mod cli;
mod digest;
mod factory;
mod gather;
mod graph;
mod manifest;
mod node;
mod parse;
mod paths;
mod pkg;
mod site_pkgs;

fn main() {
    env_logger::init();
    match cli::run() {
        Ok(_) => {}
        Err(e) => {
            eprintln!("{:#}", e);
            exit(1);
        }
    }
}
