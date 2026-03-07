use anyhow::{Context, Result};
use windows_service::service::{ServiceAccess, ServiceState};
use windows_service::service_manager::{ServiceManager, ServiceManagerAccess};

use crate::config;

pub fn run(service_name: &str) -> Result<()> {
    let manager = ServiceManager::local_computer(
        None::<&str>,
        ServiceManagerAccess::CONNECT,
    )
    .context("Failed to open Service Control Manager")?;

    let service = manager
        .open_service(service_name, ServiceAccess::QUERY_STATUS)
        .with_context(|| format!("Service '{}' not found", service_name))?;

    let status = service.query_status()?;

    let state_str = match status.current_state {
        ServiceState::Stopped => "STOPPED",
        ServiceState::StartPending => "START_PENDING",
        ServiceState::StopPending => "STOP_PENDING",
        ServiceState::Running => "RUNNING",
        ServiceState::ContinuePending => "CONTINUE_PENDING",
        ServiceState::PausePending => "PAUSE_PENDING",
        ServiceState::Paused => "PAUSED",
    };

    println!("Service: {}", service_name);
    println!("State:   {}", state_str);

    if let Ok(cfg) = config::read_config(service_name) {
        println!("Exe:     {}", cfg.exe_path);
        if !cfg.arguments.is_empty() {
            println!("Args:    {}", cfg.arguments);
        }
        if let Some(ref stdout) = cfg.stdout_path {
            println!("Stdout:  {}", stdout);
        }
    }

    Ok(())
}
