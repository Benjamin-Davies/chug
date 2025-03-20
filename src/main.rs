use clap::{Parser, Subcommand};

use chug::formulae::Formula;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

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

            let new_roots = bottles.iter().map(String::as_str).collect();
            let formulae = Formula::resolve_dependencies(new_roots)?;
            println!(
                "Installing: {}",
                formulae.keys().cloned().collect::<Vec<_>>().join(" ")
            );

            formulae
                .par_iter()
                .map(|(name, formula)| {
                    anyhow::ensure!(
                        formula.versions.bottle,
                        "Formula {name:?} does not have a corresponding bottle",
                    );

                    println!("Dowloading {} {}...", name, formula.versions.stable);
                    formula.download_bottle()?;

                    Ok(())
                })
                .collect::<anyhow::Result<Vec<_>>>()?;
        }
    }

    Ok(())
}
