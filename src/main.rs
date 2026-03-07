mod cli;
mod commands;
mod config;
mod process;
mod service;

use anyhow::Result;
use clap::Parser;
use tracing_subscriber::EnvFilter;

fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let cli = cli::Cli::parse();

    match cli.command {
        cli::Commands::Install {
            service_name,
            exe_path,
            args,
        } => commands::install::run(&service_name, &exe_path, &args),

        cli::Commands::Remove { service_name } => commands::remove::run(&service_name),

        cli::Commands::Start { service_name } => commands::start::run(&service_name),

        cli::Commands::Stop { service_name } => commands::stop::run(&service_name),

        cli::Commands::Restart { service_name } => {
            // Stop then start
            let _ = commands::stop::run(&service_name);
            std::thread::sleep(std::time::Duration::from_secs(2));
            commands::start::run(&service_name)
        }

        cli::Commands::Status { service_name } => commands::status::run(&service_name),

        cli::Commands::List => commands::list::run(),

        cli::Commands::Service { name } => service::run_as_service(name),
    }
}
