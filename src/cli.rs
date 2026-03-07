use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "nssm-rs",
    about = "Non-Sucking Service Manager — Rewritten in Rust",
    version
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Install a new service wrapping an executable
    Install {
        /// Name of the Windows service
        service_name: String,
        /// Path to the executable to wrap
        exe_path: String,
        /// Arguments to pass to the executable (use -- before args starting with -)
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// Remove an installed service
    Remove {
        /// Name of the service to remove
        service_name: String,
    },

    /// Start a service
    Start {
        /// Name of the service to start
        service_name: String,
    },

    /// Stop a service
    Stop {
        /// Name of the service to stop
        service_name: String,
    },

    /// Restart a service (stop + start)
    Restart {
        /// Name of the service to restart
        service_name: String,
    },

    /// Show the status of a service
    Status {
        /// Name of the service to query
        service_name: String,
    },

    /// List all services managed by nssm-rs
    List,

    /// Internal: run as a Windows service (called by SCM)
    #[command(hide = true)]
    Service {
        /// Service name to run
        #[arg(long)]
        name: String,
    },
}
