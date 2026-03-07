use anyhow::Result;
use windows_service::service::{ServiceAccess, ServiceState};
use windows_service::service_manager::{ServiceManager, ServiceManagerAccess};

use crate::config;

pub fn run() -> Result<()> {
    let services = config::list_managed_services()?;

    if services.is_empty() {
        println!("No services managed by nssm-rs.");
        return Ok(());
    }

    let manager = ServiceManager::local_computer(
        None::<&str>,
        ServiceManagerAccess::CONNECT,
    )?;

    println!("{:<30} {:<15} {}", "SERVICE", "STATE", "EXECUTABLE");
    println!("{}", "-".repeat(80));

    for name in &services {
        let state = if let Ok(svc) = manager.open_service(name, ServiceAccess::QUERY_STATUS) {
            match svc.query_status() {
                Ok(s) => match s.current_state {
                    ServiceState::Running => "RUNNING",
                    ServiceState::Stopped => "STOPPED",
                    ServiceState::StartPending => "STARTING",
                    ServiceState::StopPending => "STOPPING",
                    ServiceState::Paused => "PAUSED",
                    _ => "UNKNOWN",
                },
                Err(_) => "ERROR",
            }
        } else {
            "N/A"
        };

        let exe = config::read_config(name)
            .map(|c| c.exe_path)
            .unwrap_or_else(|_| "?".to_string());

        println!("{:<30} {:<15} {}", name, state, exe);
    }

    Ok(())
}
