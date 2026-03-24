mod cmd;

use clap::Parser;

/// Orchestra CLI — AI coding coordination patterns
#[derive(Parser)]
#[command(name = "orchestra", version, about)]
enum Cli {
    /// Fractal algorithmic spine — deterministic operations for decomposition
    Fractal(cmd::fractal::FractalCmd),
    /// Swarm algorithmic spine — deterministic operations for speculative swarm
    Swarm(cmd::swarm::SwarmCmd),
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli {
        Cli::Fractal(cmd) => cmd.run(),
        Cli::Swarm(cmd) => cmd.run(),
    }
}
