#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use base64::{engine::general_purpose::STANDARD, Engine as _};
use percent_encoding::percent_decode_str;
use serde::Serialize;
use std::{
    collections::HashSet,
    env,
    path::PathBuf,
    process::Command,
    sync::Mutex,
};
use tauri::State;
use url::Url;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ConnectionInfo {
    valid: bool,
    ssh_url: String,
    host: String,
    user: String,
    port: u16,
    display_target: String,
    error: Option<String>,
}

struct AppState {
    connection: Mutex<ConnectionInfo>,
}

#[derive(Clone, Debug, Serialize)]
struct AppIcons {
    winscp: Option<String>,
    terminal: Option<String>,
}

#[tauri::command]
fn get_connection_info(state: State<'_, AppState>) -> ConnectionInfo {
    state
        .connection
        .lock()
        .expect("connection state poisoned")
        .clone()
}

#[tauri::command]
fn get_app_icons() -> AppIcons {
    AppIcons {
        winscp: find_winscp().and_then(|path| extract_icon_data_url(&path)),
        terminal: find_terminal().and_then(|path| extract_icon_data_url(&path)),
    }
}

#[tauri::command]
fn launch_choice(
    choice: String,
    window: tauri::Window,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let connection = state
        .connection
        .lock()
        .map_err(|_| "无法读取连接信息。".to_string())?
        .clone();

    if !connection.valid {
        return Err(connection
            .error
            .unwrap_or_else(|| "没有收到有效的 SSH Bookmark。".to_string()));
    }

    let result = match choice.as_str() {
        "winscp" => {
            launch_winscp(&connection).map(|_| "WinSCP 已启动".to_string())
        }
        "terminal" => {
            launch_terminal(&connection).map(|_| "Windows Terminal 已启动".to_string())
        }
        "both" => {
            let winscp_result = launch_winscp(&connection);
            let terminal_result = launch_terminal(&connection);
            match (winscp_result, terminal_result) {
                (Ok(()), Ok(())) => Ok("两个应用均已启动".to_string()),
                (Err(a), Ok(())) => Err(format!("Terminal 已启动，但 WinSCP 启动失败：{a}")),
                (Ok(()), Err(b)) => Err(format!("WinSCP 已启动，但 Terminal 启动失败：{b}")),
                (Err(a), Err(b)) => Err(format!("WinSCP：{a}\nTerminal：{b}")),
            }
        }
        _ => Err("未知的打开方式。".to_string()),
    };

    let _ = window.close();
    result
}

fn parse_connection_argument() -> ConnectionInfo {
    let ssh_url = env::args()
        .skip(1)
        .find(|argument| argument.to_ascii_lowercase().starts_with("ssh://"));

    let Some(ssh_url) = ssh_url else {
        return invalid_connection(
            String::new(),
            "请从 1Password 的 SSH Bookmark 打开连接。".to_string(),
        );
    };

    let parsed = match Url::parse(&ssh_url) {
        Ok(value) if value.scheme() == "ssh" => value,
        _ => {
            return invalid_connection(ssh_url, "SSH Bookmark URL 格式无效。".to_string());
        }
    };

    let Some(host) = parsed.host_str().map(str::to_string) else {
        return invalid_connection(ssh_url, "SSH Bookmark 缺少主机名。".to_string());
    };

    let user = percent_decode_str(parsed.username())
        .decode_utf8_lossy()
        .into_owned();
    let port = parsed.port().unwrap_or(22);
    let display_host = if host.contains(':') {
        format!("[{host}]")
    } else {
        host.clone()
    };
    let display_target = match (user.is_empty(), port == 22) {
        (true, true) => display_host,
        (true, false) => format!("{display_host}:{port}"),
        (false, true) => format!("{user}@{display_host}"),
        (false, false) => format!("{user}@{display_host}:{port}"),
    };

    ConnectionInfo {
        valid: true,
        ssh_url,
        host,
        user,
        port,
        display_target,
        error: None,
    }
}

fn invalid_connection(ssh_url: String, message: String) -> ConnectionInfo {
    ConnectionInfo {
        valid: false,
        ssh_url,
        host: String::new(),
        user: String::new(),
        port: 22,
        display_target: "等待 SSH Bookmark".to_string(),
        error: Some(message),
    }
}

fn launch_winscp(connection: &ConnectionInfo) -> Result<(), String> {
    let winscp = find_winscp()
        .ok_or_else(|| "未找到 WinSCP.exe。请安装 WinSCP 6.6.1 或更高版本。".to_string())?;

    let identity_file = resolve_bookmark_identity(connection)?;
    let mut sftp_url = Url::parse(&connection.ssh_url)
        .map_err(|error| format!("无法解析 SSH URL：{error}"))?;
    sftp_url
        .set_scheme("sftp")
        .map_err(|_| "无法生成 SFTP URL。".to_string())?;
    let _ = sftp_url.set_password(None);

    Command::new(winscp)
        .args([
            "/newinstance".to_string(),
            sftp_url.to_string(),
            format!("/privatekey={}", identity_file.display()),
        ])
        .spawn()
        .map_err(|error| format!("无法启动 WinSCP：{error}"))?;

    Ok(())
}

fn launch_terminal(connection: &ConnectionInfo) -> Result<(), String> {
    let terminal =
        find_terminal().ok_or_else(|| "未找到 Windows Terminal（wt.exe）。".to_string())?;

    let ssh = find_executable(
        "ssh.exe",
        &[env_path("WINDIR", "System32\\OpenSSH\\ssh.exe")],
    )
    .ok_or_else(|| "未找到 Windows OpenSSH 客户端（ssh.exe）。".to_string())?;

    let mut arguments = vec!["new-tab".to_string(), ssh.display().to_string()];
    if !connection.user.is_empty() {
        arguments.push("-l".to_string());
        arguments.push(connection.user.clone());
    }
    if connection.port != 22 {
        arguments.push("-p".to_string());
        arguments.push(connection.port.to_string());
    }
    arguments.push(connection.host.clone());

    Command::new(terminal)
        .args(arguments)
        .spawn()
        .map_err(|error| format!("无法启动 Windows Terminal：{error}"))?;

    Ok(())
}

fn resolve_bookmark_identity(connection: &ConnectionInfo) -> Result<PathBuf, String> {
    let ssh = find_executable(
        "ssh.exe",
        &[env_path("WINDIR", "System32\\OpenSSH\\ssh.exe")],
    )
    .ok_or_else(|| "未找到 Windows OpenSSH 客户端（ssh.exe）。".to_string())?;

    let mut command = Command::new(ssh);
    command.arg("-G");
    if !connection.user.is_empty() {
        command.args(["-l", &connection.user]);
    }
    let port = connection.port.to_string();
    command.args(["-p", &port]);
    command.arg(&connection.host);
    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW);

    let output = command
        .output()
        .map_err(|error| format!("无法读取 SSH 配置：{error}"))?;
    if !output.status.success() {
        return Err("OpenSSH 无法解析该 Bookmark 的配置。".to_string());
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let mut seen = HashSet::new();
    let mut identities = Vec::new();

    for line in text.lines() {
        let mut parts = line.splitn(2, char::is_whitespace);
        let key = parts.next().unwrap_or_default();
        if !key.eq_ignore_ascii_case("identityfile") {
            continue;
        }
        let value = parts.next().unwrap_or_default().trim().trim_matches('"');
        let path = expand_home(value);
        let normalized = path
            .to_string_lossy()
            .replace('/', "\\")
            .to_ascii_lowercase();
        if normalized.contains("\\.ssh\\1password\\")
            && normalized.ends_with(".pub")
            && path.is_file()
            && seen.insert(normalized)
        {
            identities.push(path);
        }
    }

    match identities.len() {
        1 => Ok(identities.remove(0)),
        count => Err(format!(
            "无法确定唯一的 1Password 公钥（找到 {count} 个）。请刷新 1Password SSH Bookmarks。"
        )),
    }
}

fn expand_home(value: &str) -> PathBuf {
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

fn env_path(variable: &str, suffix: &str) -> PathBuf {
    env::var_os(variable)
        .map(PathBuf::from)
        .unwrap_or_default()
        .join(suffix)
}

fn find_winscp() -> Option<PathBuf> {
    find_executable(
        "WinSCP.exe",
        &[
            env_path("LOCALAPPDATA", "Programs\\WinSCP\\WinSCP.exe"),
            env_path("ProgramFiles(x86)", "WinSCP\\WinSCP.exe"),
            env_path("ProgramFiles", "WinSCP\\WinSCP.exe"),
        ],
    )
}

fn find_terminal() -> Option<PathBuf> {
    find_executable(
        "wt.exe",
        &[env_path(
            "LOCALAPPDATA",
            "Microsoft\\WindowsApps\\wt.exe",
        )],
    )
}

fn extract_icon_data_url(path: &PathBuf) -> Option<String> {
    let path_text = path.to_string_lossy();
    systemicons::get_icon(path_text.as_ref(), 64)
        .ok()
        .map(|png| format!("data:image/png;base64,{}", STANDARD.encode(png)))
}

fn find_executable(name: &str, extra_candidates: &[PathBuf]) -> Option<PathBuf> {
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

fn main() {
    let connection = parse_connection_argument();
    tauri::Builder::default()
        .manage(AppState {
            connection: Mutex::new(connection),
        })
        .invoke_handler(tauri::generate_handler![
            get_connection_info,
            get_app_icons,
            launch_choice
        ])
        .run(tauri::generate_context!())
        .expect("error while running SSH Launcher");
}
