//! Executable discovery and path helpers shared by launch + diagnostics.

use std::{
    env, fs,
    path::PathBuf,
    process::Command,
    time::SystemTime,
};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

pub const CREATE_NO_WINDOW: u32 = 0x0800_0000;

pub fn env_path(variable: &str, suffix: &str) -> PathBuf {
    env::var_os(variable)
        .map(PathBuf::from)
        .unwrap_or_default()
        .join(suffix)
}

pub fn expand_home(value: &str) -> PathBuf {
    if value == "~" {
        return env::var_os("USERPROFILE")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(value));
    }
    if let Some(remainder) = value
        .strip_prefix("~/")
        .or_else(|| value.strip_prefix("~\\"))
    {
        if let Some(home) = env::var_os("USERPROFILE") {
            return PathBuf::from(home).join(remainder);
        }
    }
    PathBuf::from(value)
}

pub fn find_executable(name: &str, extra_candidates: &[PathBuf]) -> Option<PathBuf> {
    if let Some(path) = env::var_os("PATH") {
        for directory in env::split_paths(&path) {
            let candidate = directory.join(name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    extra_candidates
        .iter()
        .find(|candidate| candidate.is_file())
        .cloned()
}

pub fn find_ssh() -> Option<PathBuf> {
    find_executable(
        "ssh.exe",
        &[env_path("WINDIR", "System32\\OpenSSH\\ssh.exe")],
    )
}

pub fn find_ssh_add() -> Option<PathBuf> {
    find_executable(
        "ssh-add.exe",
        &[env_path("WINDIR", "System32\\OpenSSH\\ssh-add.exe")],
    )
}

pub fn find_winscp() -> Option<PathBuf> {
    find_executable(
        "WinSCP.exe",
        &[
            env_path("LOCALAPPDATA", "Programs\\WinSCP\\WinSCP.exe"),
            env_path("ProgramFiles(x86)", "WinSCP\\WinSCP.exe"),
            env_path("ProgramFiles", "WinSCP\\WinSCP.exe"),
        ],
    )
}

pub fn find_cyberduck() -> Option<PathBuf> {
    find_executable(
        "Cyberduck.exe",
        &[
            env_path("ProgramFiles", "Cyberduck\\Cyberduck.exe"),
            env_path("ProgramFiles(x86)", "Cyberduck\\Cyberduck.exe"),
            env_path("LOCALAPPDATA", "Programs\\Cyberduck\\Cyberduck.exe"),
        ],
    )
}

pub fn find_terminal() -> Option<PathBuf> {
    find_executable(
        "wt.exe",
        &[env_path(
            "LOCALAPPDATA",
            "Microsoft\\WindowsApps\\wt.exe",
        )],
    )
}

pub fn find_terminal_icon_source() -> Option<PathBuf> {
    find_executable("WindowsTerminal.exe", &[])
        .or_else(find_packaged_terminal)
        .or_else(query_packaged_terminal)
}

fn find_packaged_terminal() -> Option<PathBuf> {
    let windows_apps = env_path("ProgramFiles", "WindowsApps");
    let mut candidates = fs::read_dir(windows_apps)
        .ok()?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if !name.starts_with("Microsoft.WindowsTerminal_") {
                return None;
            }
            let executable = entry.path().join("WindowsTerminal.exe");
            executable.is_file().then_some(executable)
        })
        .collect::<Vec<_>>();

    candidates.sort_by_key(|path| {
        fs::metadata(path)
            .and_then(|metadata| metadata.modified())
            .unwrap_or(SystemTime::UNIX_EPOCH)
    });
    candidates.pop()
}

fn query_packaged_terminal() -> Option<PathBuf> {
    let powershell = env_path(
        "WINDIR",
        "System32\\WindowsPowerShell\\v1.0\\powershell.exe",
    );
    let mut command = Command::new(powershell);
    command.args([
        "-NoLogo",
        "-NoProfile",
        "-NonInteractive",
        "-Command",
        "Get-AppxPackage -Name Microsoft.WindowsTerminal | Sort-Object Version -Descending | Select-Object -First 1 -ExpandProperty InstallLocation",
    ]);
    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW);

    let output = command.output().ok()?;
    if !output.status.success() {
        return None;
    }

    let install_location = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if install_location.is_empty() {
        return None;
    }

    let executable = PathBuf::from(install_location).join("WindowsTerminal.exe");
    executable.is_file().then_some(executable)
}

pub fn run_command_capture(program: &PathBuf, args: &[&str]) -> Option<String> {
    let mut command = Command::new(program);
    command.args(args);
    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW);
    let output = command.output().ok()?;
    let mut text = String::from_utf8_lossy(&output.stderr).to_string();
    if text.trim().is_empty() {
        text = String::from_utf8_lossy(&output.stdout).to_string();
    }
    Some(text)
}
