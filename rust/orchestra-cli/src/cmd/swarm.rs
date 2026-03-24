use anyhow::{Context, Result};
use clap::Parser;

use orchestra_core::swarm;

/// Swarm algorithmic spine — deterministic operations for speculative swarm
#[derive(Parser)]
pub struct SwarmCmd {
    #[command(subcommand)]
    subcmd: SwarmSubCmd,
}

#[derive(Parser)]
enum SwarmSubCmd {
    /// Compute pairwise Jaccard similarities between swarm branches
    Checkpoint(InputArgs),
    /// Prune convergent branches (remove high-similarity duplicates, min 2 survivors)
    Prune(InputArgs),
}

#[derive(Parser)]
struct InputArgs {
    /// Read JSON input from this file (default: stdin)
    #[arg(short, long)]
    input: Option<String>,
}

/// Read JSON from file or stdin.
fn read_input(path: &Option<String>) -> Result<String> {
    match path {
        Some(p) => std::fs::read_to_string(p).context(format!("reading {}", p)),
        None => {
            use std::io::Read;
            let mut buf = String::new();
            std::io::stdin()
                .read_to_string(&mut buf)
                .context("reading stdin")?;
            Ok(buf)
        }
    }
}

impl SwarmCmd {
    pub fn run(self) -> Result<()> {
        match self.subcmd {
            SwarmSubCmd::Checkpoint(args) => {
                let json = read_input(&args.input)?;
                let manifest: swarm::SwarmManifest =
                    serde_json::from_str(&json).context("parsing SwarmManifest")?;
                let output = swarm::checkpoint::run(&manifest);
                println!("{}", serde_json::to_string_pretty(&output)?);
                Ok(())
            }
            SwarmSubCmd::Prune(args) => {
                let json = read_input(&args.input)?;
                let input: swarm::prune::PruneInput =
                    serde_json::from_str(&json).context("parsing PruneInput")?;
                let output = swarm::prune::run(&input);
                println!("{}", serde_json::to_string_pretty(&output)?);
                Ok(())
            }
        }
    }
}
