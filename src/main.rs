use clap::{Parser, Subcommand};

use chug::action_builder::{ActionBuilder, BottleForestSnapshot};

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
            let snapshot = BottleForestSnapshot::new()?;
            ActionBuilder::new(&snapshot).add(&bottles)?.run()?;
        }
        Commands::Remove { all: true, bottles } => {
            anyhow::ensure!(
                bottles.is_empty(),
                "Cannot specify bottles when --all is used",
            );

            let snapshot = BottleForestSnapshot::new()?;
            ActionBuilder::new(&snapshot).remove_all().run()?;
        }
        Commands::Remove {
            bottles,
            all: false,
        } => {
            let snapshot = BottleForestSnapshot::new()?;
            ActionBuilder::new(&snapshot).remove(&bottles)?.run()?;
        }
    }

    Ok(())
}
