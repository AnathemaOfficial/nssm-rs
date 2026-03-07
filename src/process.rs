use anyhow::{Context, Result};
use std::fs::{self, File, OpenOptions};
use std::process::{Child, Command, Stdio};

use crate::config::ServiceConfig;

/// Spawn the wrapped process with I/O redirection.
pub fn spawn(config: &ServiceConfig) -> Result<Child> {
    let mut cmd = Command::new(&config.exe_path);

    // Parse arguments
    if !config.arguments.is_empty() {
        for arg in config.arguments.split_whitespace() {
            cmd.arg(arg);
        }
    }

    // Set working directory
    if !config.working_dir.is_empty() {
        cmd.current_dir(&config.working_dir);
    }

    // Configure stdout redirection
    if let Some(ref stdout_path) = config.stdout_path {
        if let Some(parent) = std::path::Path::new(stdout_path).parent() {
            fs::create_dir_all(parent).ok();
        }
        let file = open_log_file(stdout_path)
            .with_context(|| format!("Failed to open stdout log: {}", stdout_path))?;
        cmd.stdout(Stdio::from(file));
    } else {
        cmd.stdout(Stdio::null());
    }

    // Configure stderr redirection
    if let Some(ref stderr_path) = config.stderr_path {
        if let Some(parent) = std::path::Path::new(stderr_path).parent() {
            fs::create_dir_all(parent).ok();
        }
        let file = open_log_file(stderr_path)
            .with_context(|| format!("Failed to open stderr log: {}", stderr_path))?;
        cmd.stderr(Stdio::from(file));
    } else {
        cmd.stderr(Stdio::null());
    }

    // No stdin for service processes
    cmd.stdin(Stdio::null());

    let child = cmd
        .spawn()
        .with_context(|| format!("Failed to spawn process: {}", config.exe_path))?;

    Ok(child)
}

/// Open a log file for appending, creating it if necessary.
fn open_log_file(path: &str) -> Result<File> {
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .with_context(|| format!("Cannot open log file: {}", path))
}

/// Try to gracefully terminate a process, then force kill if needed.
pub fn terminate(child: &mut Child, timeout_ms: u32) -> Result<()> {
    // First, try sending Ctrl+C via GenerateConsoleCtrlEvent
    let pid = child.id();
    let sent_ctrl_c = send_ctrl_c(pid);

    if sent_ctrl_c {
        // Wait for the process to exit gracefully
        match wait_with_timeout(child, timeout_ms) {
            Ok(true) => return Ok(()),
            _ => {}
        }
    }

    // Force kill if graceful shutdown failed
    tracing::warn!("Force killing process {}", pid);
    child.kill().context("Failed to kill process")?;
    child.wait().ok();

    Ok(())
}

/// Send Ctrl+C to a process using Windows API.
fn send_ctrl_c(pid: u32) -> bool {
    use windows::Win32::System::Console::{
        AttachConsole, FreeConsole, GenerateConsoleCtrlEvent, SetConsoleCtrlHandler,
        CTRL_C_EVENT,
    };

    unsafe {
        // Detach from our own console
        let _ = FreeConsole();

        // Attach to target process console
        if AttachConsole(pid).is_err() {
            return false;
        }

        // Disable Ctrl+C handling in our own process
        let _ = SetConsoleCtrlHandler(None, true);

        // Send Ctrl+C to the target
        let result = GenerateConsoleCtrlEvent(CTRL_C_EVENT, 0).is_ok();

        // Re-enable Ctrl+C handling
        let _ = SetConsoleCtrlHandler(None, false);

        // Detach from target console
        let _ = FreeConsole();

        result
    }
}

/// Wait for a process to exit with a timeout. Returns Ok(true) if exited, Ok(false) if timeout.
fn wait_with_timeout(child: &mut Child, timeout_ms: u32) -> Result<bool> {
    let start = std::time::Instant::now();
    let timeout = std::time::Duration::from_millis(timeout_ms as u64);

    loop {
        match child.try_wait()? {
            Some(_) => return Ok(true),
            None => {
                if start.elapsed() >= timeout {
                    return Ok(false);
                }
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        }
    }
}
