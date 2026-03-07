use anyhow::{Context, Result};
use winreg::enums::*;
use winreg::RegKey;

const NSSM_RS_MARKER: &str = "nssm-rs";

/// Configuration for a managed service, stored in the Windows registry.
#[derive(Debug, Clone)]
pub struct ServiceConfig {
    pub exe_path: String,
    pub arguments: String,
    pub working_dir: String,
    pub stdout_path: Option<String>,
    pub stderr_path: Option<String>,
    pub restart_delay_ms: u32,
    pub throttle_ms: u32,
    pub stop_timeout_ms: u32,
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self {
            exe_path: String::new(),
            arguments: String::new(),
            working_dir: String::new(),
            stdout_path: None,
            stderr_path: None,
            restart_delay_ms: 0,
            throttle_ms: 1500,
            stop_timeout_ms: 5000,
        }
    }
}

/// Open the registry key for a service's parameters.
fn open_parameters_key(service_name: &str, writable: bool) -> Result<RegKey> {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let path = format!(
        r"SYSTEM\CurrentControlSet\Services\{}\Parameters",
        service_name
    );

    if writable {
        let (key, _) = hklm
            .create_subkey(&path)
            .with_context(|| format!("Failed to create registry key: {}", path))?;
        Ok(key)
    } else {
        hklm.open_subkey(&path)
            .with_context(|| format!("Failed to open registry key: {}", path))
    }
}

/// Write service configuration to the registry.
pub fn write_config(service_name: &str, config: &ServiceConfig) -> Result<()> {
    let key = open_parameters_key(service_name, true)?;

    key.set_value("Application", &config.exe_path)?;
    key.set_value("AppParameters", &config.arguments)?;
    key.set_value("AppDirectory", &config.working_dir)?;
    key.set_value("AppRestartDelay", &config.restart_delay_ms)?;
    key.set_value("AppThrottle", &config.throttle_ms)?;
    key.set_value("AppStopTimeout", &config.stop_timeout_ms)?;
    key.set_value("ManagedBy", &NSSM_RS_MARKER)?;

    if let Some(ref path) = config.stdout_path {
        key.set_value("AppStdout", path)?;
    }
    if let Some(ref path) = config.stderr_path {
        key.set_value("AppStderr", path)?;
    }

    Ok(())
}

/// Read service configuration from the registry.
pub fn read_config(service_name: &str) -> Result<ServiceConfig> {
    let key = open_parameters_key(service_name, false)?;

    Ok(ServiceConfig {
        exe_path: key.get_value("Application").unwrap_or_default(),
        arguments: key.get_value("AppParameters").unwrap_or_default(),
        working_dir: key.get_value("AppDirectory").unwrap_or_default(),
        stdout_path: key.get_value("AppStdout").ok(),
        stderr_path: key.get_value("AppStderr").ok(),
        restart_delay_ms: key.get_value("AppRestartDelay").unwrap_or(0),
        throttle_ms: key.get_value("AppThrottle").unwrap_or(1500),
        stop_timeout_ms: key.get_value("AppStopTimeout").unwrap_or(5000),
    })
}

/// Delete service configuration from the registry.
pub fn delete_config(service_name: &str) -> Result<()> {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let svc_path = format!(r"SYSTEM\CurrentControlSet\Services\{}", service_name);

    // Try to delete Parameters subkey
    if let Ok(svc_key) = hklm.open_subkey_with_flags(&svc_path, KEY_WRITE) {
        let _ = svc_key.delete_subkey("Parameters");
    }

    Ok(())
}

/// Check if a service is managed by nssm-rs.
pub fn is_managed_by_us(service_name: &str) -> bool {
    if let Ok(key) = open_parameters_key(service_name, false) {
        if let Ok(managed_by) = key.get_value::<String, _>("ManagedBy") {
            return managed_by == NSSM_RS_MARKER;
        }
        // Also check if it has our Application key (compat with manual setup)
        return key.get_value::<String, _>("Application").is_ok();
    }
    false
}

/// List all services managed by nssm-rs.
pub fn list_managed_services() -> Result<Vec<String>> {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let services_key = hklm.open_subkey(r"SYSTEM\CurrentControlSet\Services")?;

    let mut managed = Vec::new();

    for name in services_key.enum_keys().filter_map(|k| k.ok()) {
        if is_managed_by_us(&name) {
            managed.push(name);
        }
    }

    Ok(managed)
}
