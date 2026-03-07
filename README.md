# nssm-rs

**Modern Windows service manager written in Rust** — a drop-in replacement for [NSSM](https://nssm.cc).

Turn any executable into a Windows service with a single command. One binary, zero dependencies, 855KB.

## Features

- **Single binary** — just `nssm-rs.exe`, no installer needed
- **Auto-restart** — restarts crashed processes with exponential backoff
- **I/O redirection** — stdout/stderr logged to files automatically
- **Graceful shutdown** — sends Ctrl+C before force-killing
- **Registry-based config** — compatible with NSSM's approach
- **Tiny** — ~855KB release binary

## Installation

Download `nssm-rs.exe` from the [latest release](https://github.com/AnathemaOfficial/nssm-rs/releases/latest) and place it somewhere in your PATH (e.g. `C:\Program Files\nssm-rs\`).

Or build from source:

```bash
cargo build --release
# Binary at target/release/nssm-rs.exe
```

## Usage

All commands require **Administrator** privileges.

### Install a service

```bash
nssm-rs install <service-name> <exe-path> [-- <args>...]
```

Example:
```bash
nssm-rs install MyApp "C:\myapp\server.exe" -- --port 8080 --config prod.toml
```

### Manage services

```bash
nssm-rs start <service-name>       # Start the service
nssm-rs stop <service-name>        # Stop the service
nssm-rs restart <service-name>     # Restart the service
nssm-rs status <service-name>      # Show service status
nssm-rs list                       # List all nssm-rs managed services
nssm-rs remove <service-name>      # Remove the service
```

### Example output

```
> nssm-rs list
SERVICE                        STATE           EXECUTABLE
--------------------------------------------------------------------------------
MyApp                          RUNNING         C:\myapp\server.exe
MyWorker                       STOPPED         C:\workers\worker.exe
```

## How it works

When you run `nssm-rs install`, it:

1. Registers a Windows service pointing to `nssm-rs.exe` itself
2. Stores the wrapped executable config in the Windows registry
3. When Windows starts the service, `nssm-rs.exe` spawns your executable
4. Monitors the process and restarts it if it crashes
5. On service stop, sends Ctrl+C for graceful shutdown

### Configuration

Config is stored in the registry at:
```
HKLM\SYSTEM\CurrentControlSet\Services\<name>\Parameters\
```

| Key | Type | Description |
|-----|------|-------------|
| Application | REG_SZ | Path to the executable |
| AppParameters | REG_SZ | Command-line arguments |
| AppDirectory | REG_SZ | Working directory |
| AppStdout | REG_SZ | Stdout log file path |
| AppStderr | REG_SZ | Stderr log file path |
| AppThrottle | REG_DWORD | Restart throttle delay (ms) |
| AppStopTimeout | REG_DWORD | Graceful shutdown timeout (ms) |

### Logs

By default, logs are written to:
```
C:\ProgramData\nssm-rs\logs\<service-name>\stdout.log
C:\ProgramData\nssm-rs\logs\<service-name>\stderr.log
```

## License

MIT
