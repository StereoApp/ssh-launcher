export function resolvePreviewOpenDiagnostics() {
  try {
    const params = new URLSearchParams(window.location.search);
    if (params.has("diagnostics") || params.get("view") === "diagnostics") {
      return true;
    }
  } catch {
    // Ignore malformed query strings.
  }
  return false;
}

/** Only treat missing Tauri bridge as preview; real failures stay visible. */
export function isTauriBridgeMissing(error) {
  const text = String(error || "");
  return (
    text.includes("__TAURI__") ||
    text.includes("Tauri") ||
    text.includes("invoke") ||
    text.includes("not a function") ||
    text.includes("Cannot read")
  );
}

export function createMockDiagnosticsReport(connection, sftpClient) {
  const target = connection?.displayTarget || "demo@server.example.com:2222";
  const checks = [
    {
      id: "openssh",
      status: "ok",
      detail: "C:\\Windows\\System32\\OpenSSH\\ssh.exe · OpenSSH_for_Windows (preview)",
      hint: null,
    },
    {
      id: "agent_pipe",
      status: "ok",
      detail: String.raw`Named pipe \\.\pipe\openssh-ssh-agent is available`,
      hint: "Preview mock: agent pipe assumed available.",
    },
    {
      id: "terminal",
      status: "ok",
      detail: "wt.exe (preview)",
      hint: null,
    },
    {
      id: "winscp_present",
      status: "ok",
      detail: "WinSCP.exe (preview)",
      hint: null,
    },
    {
      id: "winscp_version",
      status: "ok",
      detail: "WinSCP 6.6.2 (preview) supports OpenSSH agent (requires 6.6.1+)",
      hint: null,
    },
    {
      id: "winscp_agent",
      status: "ok",
      detail: "WinSCP AuthAgent=1 (OpenSSH ssh-agent)",
      hint: null,
    },
    {
      id: "cyberduck",
      status: sftpClient === "cyberduck" ? "ok" : "unknown",
      detail:
        sftpClient === "cyberduck"
          ? "Cyberduck.exe (preview)"
          : "Cyberduck not installed (optional unless --sftp=cyberduck)",
      hint: null,
    },
    {
      id: "op_config",
      status: "ok",
      detail: "%USERPROFILE%\\.ssh\\1Password\\config (preview)",
      hint: null,
    },
    {
      id: "op_include",
      status: "ok",
      detail: "%USERPROFILE%\\.ssh\\config includes 1Password config",
      hint: null,
    },
    {
      id: "identity",
      status: connection?.valid === false ? "unknown" : "ok",
      detail:
        connection?.valid === false
          ? "No valid SSH Bookmark in this session"
          : `${target} — 1 candidate(s); preferred for WinSCP: id_ed25519.pub (preview)`,
      hint: null,
    },
  ];

  return {
    sftpClient: sftpClient || "winscp",
    connectionValid: Boolean(connection?.valid),
    displayTarget: target,
    checks,
    reportText: [
      "SSH Launcher diagnostics (browser preview mock)",
      `sftpClient: ${sftpClient || "winscp"}`,
      `connection: valid=${Boolean(connection?.valid)} target=${target}`,
      "",
      ...checks.map((c) => `[${String(c.status).toUpperCase()}] ${c.id} — ${c.detail}`),
    ].join("\n"),
  };
}

export function agentPipeStatus(diagnostics) {
  const check = diagnostics?.checks?.find((item) => item.id === "agent_pipe");
  return check?.status || null;
}
