use clap::{Parser, Subcommand};

use chug::formulae::Formula;

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Install { bottles: Vec<String> },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Install { bottles } => {
            anyhow::ensure!(
                bottles.len() >= 1,
                "Must specify one or more bottles to install",
            );

            let dependencies =
                Formula::resolve_dependencies(bottles.iter().map(String::as_str).collect())?;
            println!(
                "Installing: {}",
                dependencies.keys().cloned().collect::<Vec<_>>().join(" ")
            );
        }
    }

    Ok(())
}
