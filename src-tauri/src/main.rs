#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use base64::{engine::general_purpose::STANDARD, Engine as _};
use percent_encoding::percent_decode_str;
use serde::Serialize;
use std::{
    collections::HashSet,
    env,
    fs,
    path::PathBuf,
    process::Command,
    sync::Mutex,
    time::{Duration, SystemTime},
};
use tauri::{Manager, State, Theme, WebviewWindow};
use url::Url;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ThemePreference {
    System,
    Light,
    Dark,
}

impl ThemePreference {
    fn as_str(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "system" | "auto" | "default" => Some(Self::System),
            "light" => Some(Self::Light),
            "dark" => Some(Self::Dark),
            _ => None,
        }
    }

    fn to_tauri_theme(self) -> Option<Theme> {
        match self {
            Self::System => None,
            Self::Light => Some(Theme::Light),
            Self::Dark => Some(Theme::Dark),
        }
    }
}

/// Mutually exclusive SFTP GUI client. Selected via CLI; default WinSCP.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SftpClient {
    WinScp,
    Cyberduck,
}

impl SftpClient {
    fn as_str(self) -> &'static str {
        match self {
            Self::WinScp => "winscp",
            Self::Cyberduck => "cyberduck",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "winscp" | "win-scp" | "win_scp" => Some(Self::WinScp),
            "cyberduck" | "cyber-duck" | "duck" => Some(Self::Cyberduck),
            _ => None,
        }
    }

    fn display_name(self) -> &'static str {
        match self {
            Self::WinScp => "WinSCP",
            Self::Cyberduck => "Cyberduck",
        }
    }
}

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
    theme_preference: ThemePreference,
    sftp_client: SftpClient,
}

#[derive(Clone, Debug, Serialize)]
struct AppIcons {
    winscp: Option<String>,
    cyberduck: Option<String>,
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
fn get_theme_preference(state: State<'_, AppState>) -> String {
    state.theme_preference.as_str().to_string()
}

#[tauri::command]
fn get_sftp_preference(state: State<'_, AppState>) -> String {
    state.sftp_client.as_str().to_string()
}

#[tauri::command]
fn get_app_icons() -> AppIcons {
    AppIcons {
        winscp: find_winscp().and_then(|path| extract_icon_data_url(&path)),
        cyberduck: find_cyberduck().and_then(|path| extract_icon_data_url(&path)),
        terminal: find_terminal_icon_source().and_then(|path| extract_icon_data_url(&path)),
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
    let sftp_client = state.sftp_client;
    let sftp_name = sftp_client.display_name();

    if !connection.valid {
        return Err(connection
            .error
            .unwrap_or_else(|| "没有收到有效的 SSH Bookmark。".to_string()));
    }

    let result = match choice.as_str() {
        // "winscp" remains the action id / shortcut W; dispatches to the selected SFTP GUI.
        "winscp" | "sftp" => {
            launch_sftp(&connection, sftp_client).map(|_| format!("{sftp_name} 已启动"))
        }
        "terminal" => {
            launch_terminal(&connection).map(|_| "Windows Terminal 已启动".to_string())
        }
        "both" => {
            let sftp_result = launch_sftp(&connection, sftp_client);
            let terminal_result = launch_terminal(&connection);
            match (sftp_result, terminal_result) {
                (Ok(()), Ok(())) => Ok("两个应用均已启动".to_string()),
                (Err(a), Ok(())) => {
                    Err(format!("Terminal 已启动，但 {sftp_name} 启动失败：{a}"))
                }
                (Ok(()), Err(b)) => {
                    Err(format!("{sftp_name} 已启动，但 Terminal 启动失败：{b}"))
                }
                (Err(a), Err(b)) => Err(format!("{sftp_name}：{a}\nTerminal：{b}")),
            }
        }
        _ => Err("未知的打开方式。".to_string()),
    };

    let _ = window.close();
    result
}

fn parse_cli_arguments() -> (ConnectionInfo, ThemePreference, SftpClient) {
    let args: Vec<String> = env::args().skip(1).collect();
    let mut theme_preference = ThemePreference::System;
    let mut sftp_client = SftpClient::WinScp;
    let mut ssh_url = None;
    let mut index = 0;

    while index < args.len() {
        let argument = &args[index];
        let lower = argument.to_ascii_lowercase();

        if lower == "--dark" || lower == "-dark" {
            theme_preference = ThemePreference::Dark;
        } else if lower == "--light" || lower == "-light" {
            theme_preference = ThemePreference::Light;
        } else if lower == "--theme" || lower == "-theme" {
            if let Some(value) = args.get(index + 1) {
                if let Some(parsed) = ThemePreference::parse(value) {
                    theme_preference = parsed;
                    index += 1;
                }
            }
        } else if let Some(value) = lower
            .strip_prefix("--theme=")
            .or_else(|| lower.strip_prefix("-theme="))
        {
            if let Some(parsed) = ThemePreference::parse(value) {
                theme_preference = parsed;
            }
        } else if lower == "--cyberduck" || lower == "-cyberduck" {
            sftp_client = SftpClient::Cyberduck;
        } else if lower == "--winscp" || lower == "-winscp" {
            sftp_client = SftpClient::WinScp;
        } else if lower == "--sftp" || lower == "-sftp" || lower == "--sftp-client" {
            if let Some(value) = args.get(index + 1) {
                if let Some(parsed) = SftpClient::parse(value) {
                    sftp_client = parsed;
                    index += 1;
                }
            }
        } else if let Some(value) = lower
            .strip_prefix("--sftp=")
            .or_else(|| lower.strip_prefix("-sftp="))
            .or_else(|| lower.strip_prefix("--sftp-client="))
        {
            if let Some(parsed) = SftpClient::parse(value) {
                sftp_client = parsed;
            }
        } else if lower.starts_with("ssh://") {
            ssh_url = Some(argument.clone());
        }

        index += 1;
    }

    (
        parse_connection_from_url(ssh_url),
        theme_preference,
        sftp_client,
    )
}

fn parse_connection_from_url(ssh_url: Option<String>) -> ConnectionInfo {
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

fn launch_sftp(connection: &ConnectionInfo, client: SftpClient) -> Result<(), String> {
    match client {
        SftpClient::WinScp => launch_winscp(connection),
        SftpClient::Cyberduck => launch_cyberduck(connection),
    }
}

fn build_sftp_url(connection: &ConnectionInfo) -> Result<Url, String> {
    let mut sftp_url = Url::parse(&connection.ssh_url)
        .map_err(|error| format!("无法解析 SSH URL：{error}"))?;
    sftp_url
        .set_scheme("sftp")
        .map_err(|_| "无法生成 SFTP URL。".to_string())?;
    let _ = sftp_url.set_password(None);
    Ok(sftp_url)
}

fn launch_winscp(connection: &ConnectionInfo) -> Result<(), String> {
    let winscp = find_winscp()
        .ok_or_else(|| "未找到 WinSCP.exe。请安装 WinSCP 6.6.1 或更高版本。".to_string())?;

    let sftp_url = build_sftp_url(connection)?;

    // Identity is optional: prefer ssh -G IdentityFile for multi-key agent matching.
    // Missing or non-unique keys must not block WinSCP (agent-only login still works).
    let mut args = vec!["/newinstance".to_string(), sftp_url.to_string()];
    if let Some(identity_file) = resolve_preferred_identity(connection) {
        args.push(format!("/privatekey={}", identity_file.display()));
    }

    Command::new(winscp)
        .args(args)
        .spawn()
        .map_err(|error| format!("无法启动 WinSCP：{error}"))?;

    Ok(())
}

fn launch_cyberduck(connection: &ConnectionInfo) -> Result<(), String> {
    let cyberduck = find_cyberduck()
        .ok_or_else(|| "未找到 Cyberduck.exe。请先安装 Cyberduck。".to_string())?;

    // Cyberduck registers as: Cyberduck.exe "%1" (protocol handler).
    // It uses the Windows OpenSSH agent pipe for public-key auth.
    let sftp_url = build_sftp_url(connection)?;

    Command::new(cyberduck)
        .arg(sftp_url.to_string())
        .spawn()
        .map_err(|error| format!("无法启动 Cyberduck：{error}"))?;

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

/// Best-effort identity for WinSCP `/privatekey=` from OpenSSH config (`ssh -G`).
///
/// - Uses the final `IdentityFile` list from OpenSSH (any path, not only 1Password dirs).
/// - Allows multiple files; prefers the first existing `.pub`, then the first existing file.
/// - Returns `None` on any failure so agent-only launches still work.
fn resolve_preferred_identity(connection: &ConnectionInfo) -> Option<PathBuf> {
    let identities = list_identity_files(connection);
    if identities.is_empty() {
        return None;
    }

    identities
        .iter()
        .find(|path| {
            path.extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| ext.eq_ignore_ascii_case("pub"))
        })
        .cloned()
        .or_else(|| identities.into_iter().next())
}

fn list_identity_files(connection: &ConnectionInfo) -> Vec<PathBuf> {
    let Some(ssh) = find_executable(
        "ssh.exe",
        &[env_path("WINDIR", "System32\\OpenSSH\\ssh.exe")],
    ) else {
        return Vec::new();
    };

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

    let Ok(output) = command.output() else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
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
        if value.is_empty() {
            continue;
        }
        let path = expand_home(value);
        let normalized = path
            .to_string_lossy()
            .replace('/', "\\")
            .to_ascii_lowercase();
        // Trust ssh -G order; keep existing files only (public or private path).
        if path.is_file() && seen.insert(normalized) {
            identities.push(path);
        }
    }

    identities
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

fn find_cyberduck() -> Option<PathBuf> {
    find_executable(
        "Cyberduck.exe",
        &[
            env_path("ProgramFiles", "Cyberduck\\Cyberduck.exe"),
            env_path("ProgramFiles(x86)", "Cyberduck\\Cyberduck.exe"),
            env_path("LOCALAPPDATA", "Programs\\Cyberduck\\Cyberduck.exe"),
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

fn find_terminal_icon_source() -> Option<PathBuf> {
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

/// When launched by another process (e.g. 1Password via ssh://), Windows focus
/// stealing prevention often leaves this window visible/on-top but without
/// keyboard focus. Retry foreground activation so W/T/B shortcuts work immediately.
fn claim_keyboard_focus(window: &WebviewWindow) {
    let _ = window.unminimize();
    let _ = window.show();
    let _ = window.set_focus();

    #[cfg(windows)]
    if let Ok(hwnd) = window.hwnd() {
        force_foreground_window(hwnd.0 as isize);
        let _ = window.set_focus();
    }
}

fn schedule_focus_retries(app: tauri::AppHandle) {
    std::thread::spawn(move || {
        // Immediate attempts plus delayed retries after WebView finishes loading.
        for delay_ms in [0_u64, 50, 120, 250, 500, 1000, 2000] {
            if delay_ms > 0 {
                std::thread::sleep(Duration::from_millis(delay_ms));
            }

            let app_for_focus = app.clone();
            let _ = app.run_on_main_thread(move || {
                if let Some(window) = app_for_focus.get_webview_window("main") {
                    claim_keyboard_focus(&window);
                }
            });
        }
    });
}

#[cfg(windows)]
fn force_foreground_window(hwnd_value: isize) {
    // Raw Win32 to bypass focus-stealing limits when the parent (1Password) still
    // owns the foreground after launching us as a custom SSH URL handler.
    type Handle = *mut std::ffi::c_void;
    type Bool = i32;
    type Dword = u32;

    const SW_RESTORE: i32 = 9;
    const SW_SHOW: i32 = 5;
    const VK_MENU: u8 = 0x12;
    const KEYEVENTF_EXTENDEDKEY: Dword = 0x0001;
    const KEYEVENTF_KEYUP: Dword = 0x0002;

    #[link(name = "user32")]
    extern "system" {
        fn GetForegroundWindow() -> Handle;
        fn SetForegroundWindow(hwnd: Handle) -> Bool;
        fn BringWindowToTop(hwnd: Handle) -> Bool;
        fn ShowWindow(hwnd: Handle, cmd: i32) -> Bool;
        fn IsIconic(hwnd: Handle) -> Bool;
        fn GetWindowThreadProcessId(hwnd: Handle, pid: *mut Dword) -> Dword;
        fn AttachThreadInput(id_attach: Dword, id_attach_to: Dword, attach: Bool) -> Bool;
        fn SetActiveWindow(hwnd: Handle) -> Handle;
        fn SetFocus(hwnd: Handle) -> Handle;
        fn keybd_event(vk: u8, scan: u8, flags: Dword, extra: usize);
        fn AllowSetForegroundWindow(process_id: Dword) -> Bool;
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn GetCurrentThreadId() -> Dword;
    }

    unsafe {
        let hwnd = hwnd_value as Handle;
        if hwnd.is_null() {
            return;
        }

        // ASFW_ANY (-1): request permission when the launcher briefly allows it.
        let _ = AllowSetForegroundWindow(Dword::MAX);

        if IsIconic(hwnd) != 0 {
            ShowWindow(hwnd, SW_RESTORE);
        }
        ShowWindow(hwnd, SW_SHOW);

        let foreground = GetForegroundWindow();
        if foreground == hwnd {
            SetFocus(hwnd);
            return;
        }

        let foreground_thread = if foreground.is_null() {
            0
        } else {
            GetWindowThreadProcessId(foreground, std::ptr::null_mut())
        };
        let current_thread = GetCurrentThreadId();

        if foreground_thread != 0 && foreground_thread != current_thread {
            AttachThreadInput(foreground_thread, current_thread, 1);
        }

        // Simulate Alt press/release so SetForegroundWindow is allowed.
        keybd_event(VK_MENU, 0, KEYEVENTF_EXTENDEDKEY, 0);
        keybd_event(VK_MENU, 0, KEYEVENTF_EXTENDEDKEY | KEYEVENTF_KEYUP, 0);

        BringWindowToTop(hwnd);
        SetForegroundWindow(hwnd);
        SetActiveWindow(hwnd);
        SetFocus(hwnd);

        if foreground_thread != 0 && foreground_thread != current_thread {
            AttachThreadInput(foreground_thread, current_thread, 0);
        }
    }
}

fn apply_window_theme(window: &WebviewWindow, preference: ThemePreference) {
    let _ = window.set_theme(preference.to_tauri_theme());
}

fn main() {
    let (connection, theme_preference, sftp_client) = parse_cli_arguments();
    tauri::Builder::default()
        .manage(AppState {
            connection: Mutex::new(connection),
            theme_preference,
            sftp_client,
        })
        .on_page_load(|webview, payload| {
            if matches!(payload.event(), tauri::webview::PageLoadEvent::Finished) {
                if let Some(window) = webview
                    .app_handle()
                    .get_webview_window(webview.label())
                {
                    claim_keyboard_focus(&window);
                }
            }
        })
        .setup(|app| {
            let theme_preference = app.state::<AppState>().theme_preference;
            if let Some(window) = app.get_webview_window("main") {
                apply_window_theme(&window, theme_preference);
                claim_keyboard_focus(&window);
            }
            schedule_focus_retries(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_connection_info,
            get_theme_preference,
            get_sftp_preference,
            get_app_icons,
            launch_choice
        ])
        .run(tauri::generate_context!())
        .expect("error while running SSH Launcher");
}
