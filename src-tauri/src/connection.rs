//! Connection parsing and SFTP client preference.

use percent_encoding::percent_decode_str;
use serde::Serialize;
use url::Url;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SftpClient {
    WinScp,
    Cyberduck,
}

impl SftpClient {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::WinScp => "winscp",
            Self::Cyberduck => "cyberduck",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "winscp" | "win-scp" | "win_scp" => Some(Self::WinScp),
            "cyberduck" | "cyber-duck" | "duck" => Some(Self::Cyberduck),
            _ => None,
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::WinScp => "WinSCP",
            Self::Cyberduck => "Cyberduck",
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionInfo {
    pub valid: bool,
    pub ssh_url: String,
    pub host: String,
    pub user: String,
    pub port: u16,
    pub display_target: String,
    pub error: Option<String>,
}

pub fn parse_connection_from_url(ssh_url: Option<String>) -> ConnectionInfo {
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

pub fn invalid_connection(ssh_url: String, message: String) -> ConnectionInfo {
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
