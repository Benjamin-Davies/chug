use clap::{Parser, Subcommand};

use chug::{db::models::DownloadedBottle, formulae::Formula};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Add {
        bottles: Vec<String>,
    },
    Remove {
        bottles: Vec<String>,
        /// Remove all downloaded bottles.
        #[arg(long)]
        all: bool,
    },
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
                    bottle.patch()?;

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
        Commands::Remove { all: true, .. } => {
            let downloaded_bottles = DownloadedBottle::get_all()?;
            anyhow::ensure!(downloaded_bottles.len() >= 1, "No bottles to remove");

            println!(
                "Removing: {}",
                downloaded_bottles
                    .iter()
                    .map(DownloadedBottle::name)
                    .collect::<Vec<_>>()
                    .join(" "),
            );

            for bottle in downloaded_bottles {
                bottle.unlink()?;
                bottle.remove()?;
            }
        }
        Commands::Remove {
            bottles: _,
            all: false,
        } => todo!(),
    }

    Ok(())
}
