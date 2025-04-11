use clap::{Parser, Subcommand};

use chug_cli::action_builder::{ActionBuilder, BottleForestSnapshot};

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Download and link bottles.
    Add {
        /// Bottles to add.
        bottles: Vec<String>,
    },
    /// Unlink and remove bottles.
    Remove {
        /// Bottles to remove.
        bottles: Vec<String>,
        /// Remove all downloaded bottles.
        #[arg(long)]
        all: bool,
    },
    /// Update already-downloaded bottles.
    Update,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Add { bottles } => {
            let snapshot = BottleForestSnapshot::new()?;
            ActionBuilder::new(&snapshot).add_bottles(&bottles)?.run()?;
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
            ActionBuilder::new(&snapshot)
                .remove_bottles(&bottles)?
                .run()?;
        }
        Commands::Update => {
            let snapshot = BottleForestSnapshot::new()?;
            ActionBuilder::new(&snapshot).update()?.run()?;
        }
    }

    Ok(())
}
