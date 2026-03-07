use anyhow::{Context, Result};
use windows_service::service::ServiceAccess;
use windows_service::service_manager::{ServiceManager, ServiceManagerAccess};

use crate::config;

pub fn run(service_name: &str) -> Result<()> {
    let manager = ServiceManager::local_computer(
        None::<&str>,
        ServiceManagerAccess::CONNECT,
    )
    .context("Failed to open Service Control Manager (run as Administrator)")?;

    let service = manager
        .open_service(
            service_name,
            ServiceAccess::STOP | ServiceAccess::DELETE | ServiceAccess::QUERY_STATUS,
        )
        .with_context(|| format!("Failed to open service '{}'", service_name))?;

    // Try to stop the service first
    let status = service.query_status()?;
    if status.current_state != windows_service::service::ServiceState::Stopped {
        println!("Stopping service '{}'...", service_name);
        let _ = service.stop();
        // Wait a moment for it to stop
        std::thread::sleep(std::time::Duration::from_secs(3));
    }

    // Delete the service
    service
        .delete()
        .with_context(|| format!("Failed to delete service '{}'", service_name))?;

    // Clean up registry config
    let _ = config::delete_config(service_name);

    println!("Service '{}' removed successfully.", service_name);

    Ok(())
}
