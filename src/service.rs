use std::sync::mpsc;
use std::sync::OnceLock;
use std::time::Duration;

use anyhow::{Context, Result};
use windows_service::service::{
    ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus,
    ServiceType,
};
use windows_service::service_control_handler::{self, ServiceControlHandlerResult};
use windows_service::service_dispatcher;

use crate::config;
use crate::process;

/// Global storage for the service name (set before dispatcher starts).
static SERVICE_NAME: OnceLock<String> = OnceLock::new();

/// The raw service entry point required by Windows SCM.
/// Must be an `extern "system"` fn — no closures allowed.
extern "system" fn service_entry(_argc: u32, _argv: *mut *mut u16) {
    let name = SERVICE_NAME.get().expect("SERVICE_NAME not set").clone();
    if let Err(e) = service_main(name) {
        tracing::error!("Service error: {:?}", e);
    }
}

/// Entry point when running as a Windows service.
/// Called from main.rs when the CLI command is `service --name <name>`.
pub fn run_as_service(service_name: String) -> Result<()> {
    SERVICE_NAME
        .set(service_name.clone())
        .expect("SERVICE_NAME already set");

    service_dispatcher::start(service_name, service_entry)
        .context("Failed to start service dispatcher")?;

    Ok(())
}

/// The actual service main function.
fn service_main(service_name: String) -> Result<()> {
    // Channel to receive stop events from the control handler
    let (shutdown_tx, shutdown_rx) = mpsc::channel::<()>();

    // Register the service control handler
    let status_handle = service_control_handler::register(
        &service_name,
        move |control_event| -> ServiceControlHandlerResult {
            match control_event {
                ServiceControl::Stop | ServiceControl::Shutdown => {
                    let _ = shutdown_tx.send(());
                    ServiceControlHandlerResult::NoError
                }
                ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
                _ => ServiceControlHandlerResult::NotImplemented,
            }
        },
    )
    .context("Failed to register service control handler")?;

    // Report: Starting
    status_handle.set_service_status(ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::StartPending,
        controls_accepted: ServiceControlAccept::empty(),
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::from_secs(10),
        process_id: None,
    })?;

    // Read configuration from registry
    let cfg = config::read_config(&service_name)
        .context("Failed to read service configuration from registry")?;

    tracing::info!(
        "Starting wrapped process: {} {}",
        cfg.exe_path,
        cfg.arguments
    );

    // Spawn the initial process
    let mut child = process::spawn(&cfg).context("Failed to spawn wrapped process")?;

    tracing::info!("Process started with PID {}", child.id());

    // Report: Running
    status_handle.set_service_status(ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::Running,
        controls_accepted: ServiceControlAccept::STOP | ServiceControlAccept::SHUTDOWN,
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::default(),
        process_id: None,
    })?;

    // Main monitoring loop
    let mut throttle_count: u32 = 0;

    loop {
        // Check for shutdown signal (non-blocking with short timeout)
        if shutdown_rx.recv_timeout(Duration::from_millis(500)).is_ok() {
            tracing::info!("Stop signal received, shutting down wrapped process...");
            break;
        }

        // Check if the child process is still running
        match child.try_wait() {
            Ok(Some(exit_status)) => {
                let code = exit_status.code().unwrap_or(-1);
                tracing::warn!("Wrapped process exited with code {}", code);

                // Calculate throttle delay (exponential backoff, capped at 5 minutes)
                let base_delay = cfg.throttle_ms.max(1000) as u64;
                let throttle_delay = std::cmp::min(
                    base_delay * 2u64.saturating_pow(throttle_count),
                    300_000, // 5 minutes max
                );

                tracing::info!(
                    "Restarting in {} ms (attempt #{})",
                    throttle_delay,
                    throttle_count + 1
                );

                // Wait for restart delay, but check for shutdown signal
                if shutdown_rx
                    .recv_timeout(Duration::from_millis(throttle_delay))
                    .is_ok()
                {
                    tracing::info!("Stop signal during restart wait, shutting down.");
                    break;
                }

                // Try to respawn
                match process::spawn(&cfg) {
                    Ok(new_child) => {
                        tracing::info!("Process restarted with PID {}", new_child.id());
                        child = new_child;
                        throttle_count += 1;
                    }
                    Err(e) => {
                        tracing::error!("Failed to restart process: {:?}", e);
                        throttle_count += 1;
                    }
                }
            }
            Ok(None) => {
                // Process still running — reset throttle counter
                throttle_count = 0;
            }
            Err(e) => {
                tracing::error!("Error checking process status: {:?}", e);
            }
        }
    }

    // Graceful shutdown of the child process
    status_handle.set_service_status(ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::StopPending,
        controls_accepted: ServiceControlAccept::empty(),
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::from_secs(10),
        process_id: None,
    })?;

    if let Err(e) = process::terminate(&mut child, cfg.stop_timeout_ms) {
        tracing::error!("Error during process shutdown: {:?}", e);
    }

    // Report: Stopped
    status_handle.set_service_status(ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::Stopped,
        controls_accepted: ServiceControlAccept::empty(),
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::default(),
        process_id: None,
    })?;

    tracing::info!("Service stopped.");
    Ok(())
}
