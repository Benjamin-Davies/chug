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
    Add { bottles: Vec<String> },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Add { bottles } => {
            anyhow::ensure!(
                bottles.len() >= 1,
                "Must specify one or more bottles to add",
            );

            let new_roots = bottles.iter().map(String::as_str).collect();
            let formulae = Formula::resolve_dependencies(new_roots)?;
            println!(
                "Adding: {}",
                formulae.keys().cloned().collect::<Vec<_>>().join(" ")
            );

            let bottles = formulae
                .par_iter()
                .map(|(name, formula)| {
                    anyhow::ensure!(
                        formula.versions.bottle,
                        "Formula {name:?} does not have a corresponding bottle",
                    );

                    let bottle = formula.download_bottle()?;

                    Ok(bottle)
                })
                .collect::<anyhow::Result<Vec<_>>>()?;

            bottles
                .par_iter()
                .map(|bottle| {
                    bottle.link()?;

                    Ok(())
                })
                .collect::<anyhow::Result<Vec<_>>>()?;
        }
    }

    Ok(())
}
