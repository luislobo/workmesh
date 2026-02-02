use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "workmesh", version, about = "WorkMesh CLI (WIP)")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Print version information
    Version,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Some(Command::Version) => {
            println!("workmesh {}", workmesh_core::version());
        }
        None => {
            Cli::command().print_help()?;
            println!();
        }
    }
    Ok(())
}
