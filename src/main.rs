use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Init,
    CatFile {
        #[clap(short = 'p')]
        pretty_print: bool,

        object_hash: String,
    },
    HashObject {
        #[clap(short = 'w')]
        write: bool,

        file: PathBuf,
    },
}

mod commands;

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    match args.command {
        Command::Init => commands::init::invoke(),
        Command::CatFile {
            pretty_print,
            object_hash,
        } => commands::cat_file::invoke(pretty_print, object_hash),
        Command::HashObject { write, file } => commands::hash_object::invoke(write, &file),
    }?;

    Ok(())
}
