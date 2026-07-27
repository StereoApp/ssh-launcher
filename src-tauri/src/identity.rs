//! OpenSSH config identity resolution (`ssh -G`).

use std::{collections::HashSet, path::PathBuf, process::Command};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

use crate::connection::ConnectionInfo;
use crate::paths::{expand_home, find_ssh, CREATE_NO_WINDOW};

/// Best-effort identity for WinSCP `/privatekey=` from OpenSSH config (`ssh -G`).
///
/// Prefers the first existing `.pub`, else the first existing IdentityFile.
/// Returns `None` on any failure so agent-only launches still work.
pub fn resolve_preferred_identity(connection: &ConnectionInfo) -> Option<PathBuf> {
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

pub fn list_identity_files(connection: &ConnectionInfo) -> Vec<PathBuf> {
    let Some(ssh) = find_ssh() else {
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
        if path.is_file() && seen.insert(normalized) {
            identities.push(path);
        }
    }

    identities
}
