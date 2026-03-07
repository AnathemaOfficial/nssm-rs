use anyhow::{Context, Result};
use windows_service::service::ServiceAccess;
use windows_service::service_manager::{ServiceManager, ServiceManagerAccess};

pub fn run(service_name: &str) -> Result<()> {
    let manager = ServiceManager::local_computer(
        None::<&str>,
        ServiceManagerAccess::CONNECT,
    )
    .context("Failed to open Service Control Manager (run as Administrator)")?;

    let service = manager
        .open_service(service_name, ServiceAccess::START | ServiceAccess::QUERY_STATUS)
        .with_context(|| format!("Service '{}' not found", service_name))?;

    service
        .start(&[std::ffi::OsStr::new("")])
        .with_context(|| format!("Failed to start service '{}'", service_name))?;

    println!("Service '{}' started.", service_name);

    Ok(())
}
