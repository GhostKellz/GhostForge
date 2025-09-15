mod cli;
mod config;
mod game;
mod game_launcher;
mod wine;
mod launcher;
mod utils;
mod error;
mod winetricks;
mod graphics;
mod prefix;
mod protondb;
mod container;
mod performance;
#[cfg(feature = "gui")]
mod gui;

use anyhow::Result;
use clap::Parser;
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    // Check if we should launch GUI mode
    let args: Vec<String> = std::env::args().collect();

    // Launch GUI if no arguments or explicit --gui flag
    if args.len() == 1 || args.contains(&"--gui".to_string()) || args.contains(&"gui".to_string()) {
        #[cfg(feature = "gui")]
        {
            return gui::run_gui();
        }
        #[cfg(not(feature = "gui"))]
        {
            eprintln!("GUI feature not enabled. Use CLI commands or compile with --features gui");
            std::process::exit(1);
        }
    }

    // Otherwise run CLI
    let cli = cli::Cli::parse();
    cli.execute().await
}