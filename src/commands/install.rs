use anyhow::{bail, Context, Result};
use std::ffi::OsString;
use std::path::Path;
use windows_service::service::{
    ServiceAccess, ServiceErrorControl, ServiceInfo, ServiceStartType, ServiceType,
};
use windows_service::service_manager::{ServiceManager, ServiceManagerAccess};

use crate::config::{self, ServiceConfig};

pub fn run(service_name: &str, exe_path: &str, args: &[String]) -> Result<()> {
    // Validate exe path exists
    if !Path::new(exe_path).exists() {
        bail!("Executable not found: {}", exe_path);
    }

    let exe_path_abs = std::fs::canonicalize(exe_path)
        .with_context(|| format!("Failed to resolve path: {}", exe_path))?
        .to_string_lossy()
        .to_string();

    // Remove UNC prefix (\\?\) that canonicalize adds on Windows
    let exe_path_abs = exe_path_abs
        .strip_prefix(r"\\?\")
        .unwrap_or(&exe_path_abs)
        .to_string();

    // Get the path to our own executable (nssm-rs.exe)
    let self_exe = std::env::current_exe()
        .context("Failed to get current executable path")?
        .to_string_lossy()
        .to_string();

    let self_exe = self_exe
        .strip_prefix(r"\\?\")
        .unwrap_or(&self_exe)
        .to_string();

    // Open Service Control Manager
    let manager = ServiceManager::local_computer(
        None::<&str>,
        ServiceManagerAccess::CREATE_SERVICE,
    )
    .context("Failed to open Service Control Manager (run as Administrator)")?;

    // Service binary runs: nssm-rs.exe service --name <service_name>
    let service_info = ServiceInfo {
        name: OsString::from(service_name),
        display_name: OsString::from(format!("{} (nssm-rs)", service_name)),
        service_type: ServiceType::OWN_PROCESS,
        start_type: ServiceStartType::AutoStart,
        error_control: ServiceErrorControl::Normal,
        executable_path: std::path::PathBuf::from(&self_exe),
        launch_arguments: vec![
            OsString::from("service"),
            OsString::from("--name"),
            OsString::from(service_name),
        ],
        dependencies: vec![],
        account_name: None, // LocalSystem
        account_password: None,
    };

    // Create the service
    let _service = manager
        .create_service(&service_info, ServiceAccess::CHANGE_CONFIG)
        .with_context(|| format!("Failed to create service '{}' (already exists?)", service_name))?;

    // Determine working directory
    let working_dir = Path::new(&exe_path_abs)
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();

    // Default log paths
    let log_dir = format!(r"C:\ProgramData\nssm-rs\logs\{}", service_name);
    std::fs::create_dir_all(&log_dir)
        .with_context(|| format!("Failed to create log directory: {}", log_dir))?;

    // Filter out "--" separator from args
    let filtered_args: Vec<&str> = args.iter().map(|s| s.as_str()).filter(|s| *s != "--").collect();

    // Write configuration to registry
    let config = ServiceConfig {
        exe_path: exe_path_abs,
        arguments: filtered_args.join(" "),
        working_dir,
        stdout_path: Some(format!(r"{}\stdout.log", log_dir)),
        stderr_path: Some(format!(r"{}\stderr.log", log_dir)),
        ..Default::default()
    };

    config::write_config(service_name, &config)?;

    println!("Service '{}' installed successfully.", service_name);
    println!("  Executable: {}", config.exe_path);
    if !config.arguments.is_empty() {
        println!("  Arguments:  {}", config.arguments);
    }
    println!("  Logs:       {}", log_dir);
    println!();
    println!("Start it with: nssm-rs start {}", service_name);

    Ok(())
}
