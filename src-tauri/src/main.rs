#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod connection;
mod diagnostics;
mod identity;
mod paths;

use base64::{engine::general_purpose::STANDARD, Engine as _};
use connection::{parse_connection_from_url, ConnectionInfo, SftpClient};
use diagnostics::{
    build_diagnostics_report, build_diagnostics_report_with_ssh_add, DiagnosticsReport,
};
use identity::resolve_preferred_identity;
use paths::{find_cyberduck, find_ssh, find_terminal, find_terminal_icon_source, find_winscp};
use serde::Serialize;
use std::{
    env,
    path::PathBuf,
    process::Command,
    sync::Mutex,
    time::Duration,
};
use tauri::{Manager, State, Theme, WebviewWindow};
use url::Url;

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

struct AppState {
    connection: Mutex<ConnectionInfo>,
    theme_preference: ThemePreference,
    sftp_client: SftpClient,
    open_diagnostics: bool,
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
fn get_open_diagnostics(state: State<'_, AppState>) -> bool {
    state.open_diagnostics
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
fn run_diagnostics(state: State<'_, AppState>) -> DiagnosticsReport {
    let connection = state
        .connection
        .lock()
        .expect("connection state poisoned")
        .clone();
    build_diagnostics_report(&connection, state.sftp_client)
}

/// Runs full diagnostics including `ssh-add -l` (may show 1Password prompt).
#[tauri::command]
fn run_ssh_add_check(state: State<'_, AppState>) -> DiagnosticsReport {
    let connection = state
        .connection
        .lock()
        .expect("connection state poisoned")
        .clone();
    build_diagnostics_report_with_ssh_add(&connection, state.sftp_client)
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
        "winscp" | "sftp" => {
            launch_sftp(&connection, sftp_client).map(|_| format!("{sftp_name} 已启动"))
        }
        "terminal" => launch_terminal(&connection).map(|_| "Windows Terminal 已启动".to_string()),
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

fn parse_cli_arguments() -> (ConnectionInfo, ThemePreference, SftpClient, bool) {
    let args: Vec<String> = env::args().skip(1).collect();
    let mut theme_preference = ThemePreference::System;
    let mut sftp_client = SftpClient::WinScp;
    let mut open_diagnostics = false;
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
        } else if lower == "--diagnostics" || lower == "-diagnostics" || lower == "--diagnose" {
            open_diagnostics = true;
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
        open_diagnostics,
    )
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
    let ssh = find_ssh().ok_or_else(|| "未找到 Windows OpenSSH 客户端（ssh.exe）。".to_string())?;

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

fn extract_icon_data_url(path: &PathBuf) -> Option<String> {
    let path_text = path.to_string_lossy();
    systemicons::get_icon(path_text.as_ref(), 64)
        .ok()
        .map(|png| format!("data:image/png;base64,{}", STANDARD.encode(png)))
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
    let (connection, theme_preference, sftp_client, open_diagnostics) = parse_cli_arguments();

    tauri::Builder::default()
        .manage(AppState {
            connection: Mutex::new(connection),
            theme_preference,
            sftp_client,
            open_diagnostics,
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
            get_open_diagnostics,
            get_app_icons,
            run_diagnostics,
            run_ssh_add_check,
            launch_choice
        ])
        .run(tauri::generate_context!())
        .expect("error while running SSH Launcher");
}
