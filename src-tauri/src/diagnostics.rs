//! Environment diagnostics for the 1Password + OpenSSH + WinSCP workflow.

use std::{fs, path::PathBuf, process::Command};

use serde::Serialize;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

use crate::connection::{ConnectionInfo, SftpClient};
use crate::identity::{list_identity_files, resolve_preferred_identity};
use crate::paths::{
    env_path, find_cyberduck, find_ssh, find_ssh_add, find_terminal, find_winscp,
    run_command_capture, CREATE_NO_WINDOW,
};

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DiagnosticStatus {
    Ok,
    Warn,
    Fail,
    Unknown,
}

impl DiagnosticStatus {
    fn as_report_label(self) -> &'static str {
        match self {
            Self::Ok => "OK",
            Self::Warn => "WARN",
            Self::Fail => "FAIL",
            Self::Unknown => "UNKNOWN",
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticCheck {
    pub id: String,
    pub status: DiagnosticStatus,
    pub detail: String,
    pub hint: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticsReport {
    pub sftp_client: String,
    pub connection_valid: bool,
    pub display_target: String,
    pub checks: Vec<DiagnosticCheck>,
    pub report_text: String,
}

fn check(
    id: &str,
    status: DiagnosticStatus,
    detail: impl Into<String>,
    hint: Option<String>,
) -> DiagnosticCheck {
    DiagnosticCheck {
        id: id.to_string(),
        status,
        detail: detail.into(),
        hint,
    }
}

fn check_present(
    id: &str,
    path: Option<PathBuf>,
    missing: &str,
    missing_hint: &str,
) -> DiagnosticCheck {
    match path {
        Some(path) => check(id, DiagnosticStatus::Ok, path.display().to_string(), None),
        None => check(
            id,
            DiagnosticStatus::Fail,
            missing,
            Some(missing_hint.to_string()),
        ),
    }
}

pub fn build_diagnostics_report(
    connection: &ConnectionInfo,
    sftp_client: SftpClient,
) -> DiagnosticsReport {
    let checks = vec![
        check_openssh(),
        check_agent_pipe(),
        check_present(
            "terminal",
            find_terminal(),
            "Windows Terminal (wt.exe) not found",
            "Install Windows Terminal from the Microsoft Store.",
        ),
        check_present(
            "winscp_present",
            find_winscp(),
            "WinSCP.exe not found",
            "Install WinSCP 6.6.1 or newer.",
        ),
        check_winscp_version(),
        check_winscp_agent(),
        check_cyberduck(sftp_client),
        check_op_config(),
        check_op_include(),
        check_identity(connection),
    ];
    finish_report(connection, sftp_client, checks)
}

/// Full report including `ssh-add -l` (may prompt 1Password).
pub fn build_diagnostics_report_with_ssh_add(
    connection: &ConnectionInfo,
    sftp_client: SftpClient,
) -> DiagnosticsReport {
    let mut checks = build_diagnostics_report(connection, sftp_client).checks;
    checks.insert(2, check_ssh_add());
    finish_report(connection, sftp_client, checks)
}

fn finish_report(
    connection: &ConnectionInfo,
    sftp_client: SftpClient,
    checks: Vec<DiagnosticCheck>,
) -> DiagnosticsReport {
    let report_text = format_report_text(connection, sftp_client, &checks);
    DiagnosticsReport {
        sftp_client: sftp_client.as_str().to_string(),
        connection_valid: connection.valid,
        display_target: connection.display_target.clone(),
        checks,
        report_text,
    }
}

fn format_report_text(
    connection: &ConnectionInfo,
    sftp_client: SftpClient,
    checks: &[DiagnosticCheck],
) -> String {
    let mut lines = vec![
        "SSH Launcher diagnostics".to_string(),
        format!("sftpClient: {}", sftp_client.as_str()),
        format!(
            "connection: valid={} target={}",
            connection.valid, connection.display_target
        ),
        String::new(),
    ];
    for item in checks {
        lines.push(format!(
            "[{}] {} — {}",
            item.status.as_report_label(),
            item.id,
            item.detail
        ));
        if let Some(hint) = &item.hint {
            lines.push(format!("  hint: {hint}"));
        }
    }
    lines.join("\n")
}

fn check_openssh() -> DiagnosticCheck {
    match find_ssh() {
        Some(path) => {
            let version = run_command_capture(&path, &["-V"])
                .map(|text| text.trim().to_string())
                .filter(|text| !text.is_empty())
                .unwrap_or_else(|| "version unknown".to_string());
            check(
                "openssh",
                DiagnosticStatus::Ok,
                format!("{} · {}", path.display(), version),
                None,
            )
        }
        None => check(
            "openssh",
            DiagnosticStatus::Fail,
            "ssh.exe not found",
            Some("Install Windows OpenSSH Client (Optional Features).".to_string()),
        ),
    }
}

fn check_agent_pipe() -> DiagnosticCheck {
    match openssh_agent_pipe_available() {
        Some(true) => check(
            "agent_pipe",
            DiagnosticStatus::Ok,
            r"Named pipe \\.\pipe\openssh-ssh-agent is available",
            Some(
                "An OpenSSH-compatible agent is listening (1Password or another agent)."
                    .to_string(),
            ),
        ),
        Some(false) => check(
            "agent_pipe",
            DiagnosticStatus::Fail,
            r"Named pipe \\.\pipe\openssh-ssh-agent is not available",
            Some(
                "Enable the 1Password SSH Agent (and disable the system OpenSSH Authentication Agent service if 1Password asks)."
                    .to_string(),
            ),
        ),
        None => check(
            "agent_pipe",
            DiagnosticStatus::Unknown,
            "Could not probe the OpenSSH agent pipe on this platform",
            None,
        ),
    }
}

fn check_ssh_add() -> DiagnosticCheck {
    let Some(ssh_add) = find_ssh_add() else {
        return check(
            "ssh_add",
            DiagnosticStatus::Fail,
            "ssh-add.exe not found",
            Some("Install Windows OpenSSH Client.".to_string()),
        );
    };

    let mut command = Command::new(ssh_add);
    command.arg("-l");
    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW);

    match command.output() {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            let combined = match (stdout.is_empty(), stderr.is_empty()) {
                (false, false) => format!("{stdout}\n{stderr}"),
                (false, true) => stdout,
                (true, false) => stderr,
                (true, true) => String::new(),
            };

            if output.status.success() {
                if combined.is_empty()
                    || combined.contains("no identities")
                    || combined.contains("The agent has no identities")
                {
                    check(
                        "ssh_add",
                        DiagnosticStatus::Warn,
                        "ssh-add -l: agent has no identities",
                        Some(
                            "Unlock 1Password and ensure SSH keys are available to the agent."
                                .to_string(),
                        ),
                    )
                } else {
                    let lines = combined.lines().count();
                    let preview = combined.lines().take(6).collect::<Vec<_>>().join(" · ");
                    check(
                        "ssh_add",
                        DiagnosticStatus::Ok,
                        format!("ssh-add -l: {lines} identit(y/ies) · {preview}"),
                        None,
                    )
                }
            } else {
                check(
                    "ssh_add",
                    DiagnosticStatus::Fail,
                    if combined.is_empty() {
                        format!("ssh-add -l failed (exit {:?})", output.status.code())
                    } else {
                        combined
                    },
                    Some(
                        "Approve the 1Password prompt if shown; confirm the agent is enabled."
                            .to_string(),
                    ),
                )
            }
        }
        Err(error) => check(
            "ssh_add",
            DiagnosticStatus::Fail,
            format!("Could not run ssh-add: {error}"),
            None,
        ),
    }
}

fn check_winscp_version() -> DiagnosticCheck {
    let Some(path) = find_winscp() else {
        return check(
            "winscp_version",
            DiagnosticStatus::Unknown,
            "WinSCP not installed; version not checked",
            None,
        );
    };

    match read_file_version(&path) {
        Some((major, minor, patch, build)) => {
            let label = format!("{major}.{minor}.{patch}.{build}");
            let ok = major > 6 || (major == 6 && (minor > 6 || (minor == 6 && patch >= 1)));
            if ok {
                check(
                    "winscp_version",
                    DiagnosticStatus::Ok,
                    format!("WinSCP {label} supports OpenSSH agent (requires 6.6.1+)"),
                    None,
                )
            } else {
                check(
                    "winscp_version",
                    DiagnosticStatus::Warn,
                    format!("WinSCP {label} is older than 6.6.1"),
                    Some(
                        "Upgrade WinSCP to 6.6.1+ for native OpenSSH ssh-agent support."
                            .to_string(),
                    ),
                )
            }
        }
        None => check(
            "winscp_version",
            DiagnosticStatus::Unknown,
            format!("Could not read version of {}", path.display()),
            None,
        ),
    }
}

/// WinSCP registry/INI `AuthAgent`: 0 = Pageant, 1 = OpenSSH ssh-agent.
#[derive(Clone, Copy)]
enum WinscpAuthAgent {
    Pageant = 0,
    OpenSsh = 1,
}

fn check_winscp_agent() -> DiagnosticCheck {
    match read_winscp_auth_agent() {
        Some(v) if v == WinscpAuthAgent::Pageant as u32 => check(
            "winscp_agent",
            DiagnosticStatus::Warn,
            "WinSCP AuthAgent=0 (Pageant)",
            Some(
                "Set Preferences → Security → Authentication agent → OpenSSH ssh-agent."
                    .to_string(),
            ),
        ),
        Some(v) if v == WinscpAuthAgent::OpenSsh as u32 => check(
            "winscp_agent",
            DiagnosticStatus::Ok,
            "WinSCP AuthAgent=1 (OpenSSH ssh-agent)",
            None,
        ),
        Some(value) => check(
            "winscp_agent",
            DiagnosticStatus::Unknown,
            format!("WinSCP AuthAgent={value} (unrecognized)"),
            Some(
                "Confirm Preferences → Security → Authentication agent is OpenSSH ssh-agent."
                    .to_string(),
            ),
        ),
        None => check(
            "winscp_agent",
            DiagnosticStatus::Unknown,
            "Could not read WinSCP AuthAgent from registry/INI",
            Some(
                "Open WinSCP once, then set Preferences → Security → Authentication agent → OpenSSH ssh-agent."
                    .to_string(),
            ),
        ),
    }
}

fn check_cyberduck(sftp_client: SftpClient) -> DiagnosticCheck {
    match find_cyberduck() {
        Some(path) => check(
            "cyberduck",
            DiagnosticStatus::Ok,
            path.display().to_string(),
            None,
        ),
        None if sftp_client == SftpClient::Cyberduck => check(
            "cyberduck",
            DiagnosticStatus::Fail,
            "Cyberduck.exe not found (selected SFTP client)",
            Some("Install Cyberduck or launch with --sftp=winscp.".to_string()),
        ),
        None => check(
            "cyberduck",
            DiagnosticStatus::Unknown,
            "Cyberduck not installed (optional unless --sftp=cyberduck)",
            None,
        ),
    }
}

fn check_op_config() -> DiagnosticCheck {
    let path = env_path("USERPROFILE", ".ssh\\1Password\\config");
    if path.is_file() {
        let size = fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        check(
            "op_config",
            DiagnosticStatus::Ok,
            format!("{} ({size} bytes)", path.display()),
            None,
        )
    } else {
        check(
            "op_config",
            DiagnosticStatus::Warn,
            format!("{} not found", path.display()),
            Some(
                "In 1Password: enable Generate SSH config files from SSH bookmarks.".to_string(),
            ),
        )
    }
}

fn check_op_include() -> DiagnosticCheck {
    let config = env_path("USERPROFILE", ".ssh\\config");
    if !config.is_file() {
        return check(
            "op_include",
            DiagnosticStatus::Warn,
            format!("{} not found", config.display()),
            Some(r#"Add: Include ~/.ssh/1Password/config"#.to_string()),
        );
    }

    let Ok(text) = fs::read_to_string(&config) else {
        return check(
            "op_include",
            DiagnosticStatus::Unknown,
            format!("Could not read {}", config.display()),
            None,
        );
    };

    let included = text.lines().any(|line| {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            return false;
        }
        let lower = trimmed.to_ascii_lowercase();
        lower.starts_with("include")
            && (lower.contains("1password/config")
                || lower.contains("1password\\config")
                || lower.contains(".ssh/1password")
                || lower.contains(".ssh\\1password"))
    });

    if included {
        check(
            "op_include",
            DiagnosticStatus::Ok,
            format!("{} includes 1Password config", config.display()),
            None,
        )
    } else {
        check(
            "op_include",
            DiagnosticStatus::Warn,
            format!("{} has no Include for 1Password/config", config.display()),
            Some(r#"Add: Include ~/.ssh/1Password/config"#.to_string()),
        )
    }
}

fn check_identity(connection: &ConnectionInfo) -> DiagnosticCheck {
    if !connection.valid {
        return check(
            "identity",
            DiagnosticStatus::Unknown,
            "No valid SSH Bookmark in this session",
            Some("Open a 1Password SSH Bookmark (ssh://...) to resolve IdentityFile.".to_string()),
        );
    }

    let identities = list_identity_files(connection);
    let preferred = resolve_preferred_identity(connection);

    if identities.is_empty() {
        return check(
            "identity",
            DiagnosticStatus::Warn,
            format!(
                "{} — no existing IdentityFile from ssh -G (agent-only OK)",
                connection.display_target
            ),
            Some(
                "Refresh 1Password SSH Bookmarks / SSH config if you expected a matched key."
                    .to_string(),
            ),
        );
    }

    let list = identities
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>()
        .join(" · ");
    let chosen = preferred
        .as_ref()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "(none)".to_string());

    check(
        "identity",
        DiagnosticStatus::Ok,
        format!(
            "{} — {} candidate(s); preferred for WinSCP: {} | all: {}",
            connection.display_target,
            identities.len(),
            chosen,
            list
        ),
        None,
    )
}

fn read_file_version(path: &PathBuf) -> Option<(u16, u16, u16, u16)> {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        type Dword = u32;
        type Bool = i32;
        #[link(name = "version")]
        extern "system" {
            fn GetFileVersionInfoSizeW(filename: *const u16, handle: *mut Dword) -> Dword;
            fn GetFileVersionInfoW(
                filename: *const u16,
                handle: Dword,
                len: Dword,
                data: *mut u8,
            ) -> Bool;
            fn VerQueryValueW(
                block: *const u8,
                sub_block: *const u16,
                buffer: *mut *mut u8,
                len: *mut u32,
            ) -> Bool;
        }

        #[repr(C)]
        struct VsFixedFileInfo {
            signature: u32,
            struct_version: u32,
            file_version_ms: u32,
            file_version_ls: u32,
            product_version_ms: u32,
            product_version_ls: u32,
            file_flags_mask: u32,
            file_flags: u32,
            file_os: u32,
            file_type: u32,
            file_subtype: u32,
            file_date_ms: u32,
            file_date_ls: u32,
        }

        let wide: Vec<u16> = path
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        unsafe {
            let mut handle = 0_u32;
            let size = GetFileVersionInfoSizeW(wide.as_ptr(), &mut handle);
            if size == 0 {
                return None;
            }
            let mut buffer = vec![0_u8; size as usize];
            if GetFileVersionInfoW(wide.as_ptr(), 0, size, buffer.as_mut_ptr()) == 0 {
                return None;
            }
            let sub: Vec<u16> = "\\".encode_utf16().chain(std::iter::once(0)).collect();
            let mut value: *mut u8 = std::ptr::null_mut();
            let mut len = 0_u32;
            if VerQueryValueW(buffer.as_ptr(), sub.as_ptr(), &mut value, &mut len) == 0
                || value.is_null()
                || (len as usize) < std::mem::size_of::<VsFixedFileInfo>()
            {
                return None;
            }
            let info = &*(value as *const VsFixedFileInfo);
            if info.signature != 0xFEEF_04BD {
                return None;
            }
            let major = (info.file_version_ms >> 16) as u16;
            let minor = (info.file_version_ms & 0xFFFF) as u16;
            let patch = (info.file_version_ls >> 16) as u16;
            let build = (info.file_version_ls & 0xFFFF) as u16;
            return Some((major, minor, patch, build));
        }
    }
    #[cfg(not(windows))]
    {
        let _ = path;
        None
    }
}

fn read_winscp_auth_agent() -> Option<u32> {
    read_winscp_auth_agent_registry().or_else(read_winscp_auth_agent_ini)
}

fn read_winscp_auth_agent_registry() -> Option<u32> {
    #[cfg(windows)]
    {
        type Hkey = *mut std::ffi::c_void;
        type Dword = u32;
        type Long = i32;
        const HKEY_CURRENT_USER: Hkey = 0x8000_0001u32 as Hkey;
        const KEY_READ: Dword = 0x20019;
        const ERROR_SUCCESS: Long = 0;
        const REG_DWORD: Dword = 4;

        #[link(name = "advapi32")]
        extern "system" {
            fn RegOpenKeyExW(
                key: Hkey,
                sub_key: *const u16,
                options: Dword,
                sam: Dword,
                result: *mut Hkey,
            ) -> Long;
            fn RegQueryValueExW(
                key: Hkey,
                value_name: *const u16,
                reserved: *mut Dword,
                value_type: *mut Dword,
                data: *mut u8,
                data_len: *mut Dword,
            ) -> Long;
            fn RegCloseKey(key: Hkey) -> Long;
        }

        fn wide(s: &str) -> Vec<u16> {
            s.encode_utf16().chain(std::iter::once(0)).collect()
        }

        unsafe {
            let sub = wide(r"Software\Martin Prikryl\WinSCP 2\Configuration\Interface");
            let mut key = std::ptr::null_mut();
            if RegOpenKeyExW(HKEY_CURRENT_USER, sub.as_ptr(), 0, KEY_READ, &mut key) != ERROR_SUCCESS
            {
                return None;
            }
            let name = wide("AuthAgent");
            let mut value_type = 0_u32;
            let mut data = 0_u32;
            let mut data_len = 4_u32;
            let status = RegQueryValueExW(
                key,
                name.as_ptr(),
                std::ptr::null_mut(),
                &mut value_type,
                (&mut data as *mut u32) as *mut u8,
                &mut data_len,
            );
            let _ = RegCloseKey(key);
            if status != ERROR_SUCCESS || value_type != REG_DWORD {
                return None;
            }
            return Some(data);
        }
    }
    #[cfg(not(windows))]
    {
        None
    }
}

fn read_winscp_auth_agent_ini() -> Option<u32> {
    let mut paths = vec![
        env_path("APPDATA", "WinSCP.ini"),
        env_path("APPDATA", "Martin Prikryl\\WinSCP 2\\WinSCP.ini"),
    ];
    if let Some(exe) = find_winscp() {
        if let Some(parent) = exe.parent() {
            paths.push(parent.join("WinSCP.ini"));
        }
    }

    for path in paths {
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        for line in text.lines() {
            let trimmed = line.trim();
            if let Some(value) = trimmed
                .strip_prefix("AuthAgent=")
                .or_else(|| trimmed.strip_prefix("AuthAgent ="))
            {
                if let Ok(parsed) = value.trim().parse::<u32>() {
                    return Some(parsed);
                }
            }
        }
    }
    None
}

pub fn openssh_agent_pipe_available() -> Option<bool> {
    #[cfg(windows)]
    {
        type Handle = *mut std::ffi::c_void;
        type Dword = u32;
        const INVALID_HANDLE_VALUE: isize = -1;
        const GENERIC_READ: Dword = 0x8000_0000;
        const GENERIC_WRITE: Dword = 0x4000_0000;
        const OPEN_EXISTING: Dword = 3;
        const FILE_ATTRIBUTE_NORMAL: Dword = 0x80;

        #[link(name = "kernel32")]
        extern "system" {
            fn CreateFileW(
                name: *const u16,
                access: Dword,
                share: Dword,
                security: *mut std::ffi::c_void,
                disposition: Dword,
                flags: Dword,
                template: Handle,
            ) -> Handle;
            fn CloseHandle(handle: Handle) -> i32;
        }

        let pipe: Vec<u16> = r"\\.\pipe\openssh-ssh-agent"
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        unsafe {
            let handle = CreateFileW(
                pipe.as_ptr(),
                GENERIC_READ | GENERIC_WRITE,
                0,
                std::ptr::null_mut(),
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL,
                std::ptr::null_mut(),
            );
            if handle as isize == INVALID_HANDLE_VALUE || handle.is_null() {
                return Some(false);
            }
            let _ = CloseHandle(handle);
            return Some(true);
        }
    }
    #[cfg(not(windows))]
    {
        None
    }
}
