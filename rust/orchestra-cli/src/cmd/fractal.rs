use anyhow::{Context, Result};
use clap::Parser;

use orchestra_core::fractal;

/// Fractal algorithmic spine — deterministic operations for fractal decomposition
#[derive(Parser)]
pub struct FractalCmd {
    #[command(subcommand)]
    subcmd: FractalSubCmd,
}

#[derive(Parser)]
enum FractalSubCmd {
    /// Validate a decomposition structurally (TF-IDF orthogonality, cycle detection, complexity scoring)
    Gate(GateArgs),
    /// Schedule leaf solves into parallel waves (DAG topological sort)
    Schedule(ScheduleArgs),
    /// Analyze reunification conflicts (structural conflict detection + git merge-tree)
    Reunify(ReunifyArgs),
    /// Detect redundant nodes for pruning (set cover analysis)
    Prune(PruneArgs),
    /// Compute complexity score for a spec
    Complexity(ComplexityArgs),
}

#[derive(Parser)]
struct GateArgs {
    /// Read DecomposeInput JSON from this file (default: stdin)
    #[arg(short, long)]
    input: Option<String>,
}

#[derive(Parser)]
struct ScheduleArgs {
    /// Read manifest JSON from this file (default: stdin)
    #[arg(short, long)]
    input: Option<String>,
}

#[derive(Parser)]
struct ReunifyArgs {
    /// Read ReunifyInput JSON from this file (default: stdin)
    #[arg(short, long)]
    input: Option<String>,
}

#[derive(Parser)]
struct PruneArgs {
    /// Read tree JSON from this file (default: stdin)
    #[arg(short, long)]
    input: Option<String>,
}

#[derive(Parser)]
struct ComplexityArgs {
    /// Read spec text from this file (default: stdin)
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

impl FractalCmd {
    pub fn run(self) -> Result<()> {
        match self.subcmd {
            FractalSubCmd::Gate(args) => {
                let json = read_input(&args.input)?;
                let input: fractal::DecomposeInput =
                    serde_json::from_str(&json).context("parsing DecomposeInput")?;
                let output = fractal::decompose::run(&input);
                println!("{}", serde_json::to_string_pretty(&output)?);
                Ok(())
            }
            FractalSubCmd::Schedule(args) => {
                let json = read_input(&args.input)?;
                let manifest: fractal::Manifest =
                    serde_json::from_str(&json).context("parsing Manifest")?;
                let output = fractal::schedule::run(&manifest);
                println!("{}", serde_json::to_string_pretty(&output)?);
                Ok(())
            }
            FractalSubCmd::Reunify(args) => {
                let json = read_input(&args.input)?;
                let input: fractal::reunify::ReunifyInput =
                    serde_json::from_str(&json).context("parsing ReunifyInput")?;
                let output = fractal::reunify::run(&input);
                println!("{}", serde_json::to_string_pretty(&output)?);
                Ok(())
            }
            FractalSubCmd::Prune(args) => {
                let json = read_input(&args.input)?;
                let wrapper: PruneWrapper =
                    serde_json::from_str(&json).context("parsing tree input")?;
                let output = fractal::prune::run(&wrapper.tree);
                println!("{}", serde_json::to_string_pretty(&output)?);
                Ok(())
            }
            FractalSubCmd::Complexity(args) => {
                let text = read_input(&args.input)?;
                let score = fractal::decompose::complexity_score(&text);
                let output = serde_json::json!({ "complexity_score": score });
                println!("{}", serde_json::to_string_pretty(&output)?);
                Ok(())
            }
        }
    }
}

/// Wrapper to accept `{"tree": {...}}` input for the prune command.
#[derive(serde::Deserialize)]
struct PruneWrapper {
    tree: std::collections::HashMap<String, fractal::TreeNode>,
}
