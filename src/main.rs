use std::fs;

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

            let new_roots = bottles.iter().map(String::as_str).collect();
            let formulae = Formula::resolve_dependencies(new_roots)?;
            println!(
                "Installing: {}",
                formulae.keys().cloned().collect::<Vec<_>>().join(" ")
            );

            for formula in formulae.values() {
                anyhow::ensure!(
                    formula.versions.bottle,
                    "Formula {:?} does not have a corresponding bottle",
                    formula.name,
                );

                println!("Dowloading {} {}...", formula.name, formula.versions.stable);
                let bottle = formula.bottle.stable.current_target()?.fetch()?;
                fs::write("target/bottle", bottle)?;
            }
        }
    }

    Ok(())
}
