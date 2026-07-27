export const supportedLocales = ["zh-CN", "en-US"];

const diagnosticsChecksZh = {
  openssh: "Windows OpenSSH 客户端",
  agent_pipe: "OpenSSH Agent 管道",
  ssh_add: "ssh-add 密钥列表",
  terminal: "Windows Terminal",
  winscp_present: "WinSCP 安装",
  winscp_version: "WinSCP 版本（OpenSSH Agent）",
  winscp_agent: "WinSCP Agent 设置",
  cyberduck: "Cyberduck 安装",
  op_config: "1Password SSH 配置文件",
  op_include: "ssh config Include",
  identity: "Bookmark IdentityFile",
};

const diagnosticsChecksEn = {
  openssh: "Windows OpenSSH client",
  agent_pipe: "OpenSSH agent pipe",
  ssh_add: "ssh-add key list",
  terminal: "Windows Terminal",
  winscp_present: "WinSCP installation",
  winscp_version: "WinSCP version (OpenSSH agent)",
  winscp_agent: "WinSCP agent preference",
  cyberduck: "Cyberduck installation",
  op_config: "1Password SSH config file",
  op_include: "ssh config Include",
  identity: "Bookmark IdentityFile",
};

export const messages = {
  "zh-CN": {
    connectionInfoLabel: "SSH 连接信息",
    connectionTitle: "SSH 连接",
    port: "端口",
    agentConnected: "SSH Agent 管道可用",
    agentStatusMissing: "SSH Agent 管道不可用",
    agentStatusUnknown: "SSH Agent 状态未知",
    eyebrow: "1PASSWORD SSH",
    chooserTitle: "选择打开方式",
    chooserIntro: "选择一个应用继续当前连接",
    escapeHint: "按 Esc 取消",
    escapeBackHint: "按 Esc 返回",
    shortcut: "快捷键",
    launching: "正在启动…",
    previewSuccess: "预览模式：启动操作正常",
    appStarted: "应用已启动",
    bothStarted: "两个应用已启动",
    languageLabel: "界面语言",
    waitingBookmark: "等待 SSH Bookmark",
    openDiagnostics: "环境诊断",
    diagnosticsTitle: "环境诊断",
    diagnosticsIntro: "检查 1Password、OpenSSH、WinSCP 与 Terminal 是否就绪",
    diagnosticsBack: "返回",
    diagnosticsRefresh: "重新检测",
    diagnosticsListKeys: "列出 Agent 密钥",
    diagnosticsCopy: "复制报告",
    diagnosticsCopied: "报告已复制",
    diagnosticsCopyFailed: "复制失败",
    diagnosticsLoading: "正在检测…",
    diagnosticsSshAddNote: "列出密钥可能弹出 1Password 授权提示",
    diagnosticsStatus: {
      ok: "通过",
      warn: "警告",
      fail: "失败",
      unknown: "未知",
    },
    diagnosticsChecks: diagnosticsChecksZh,
    actions: {
      winscp: {
        title: "WinSCP",
        subtitle: "安全文件传输",
      },
      cyberduck: {
        title: "Cyberduck",
        subtitle: "安全文件传输",
      },
      terminal: {
        title: "Windows Terminal",
        subtitle: "命令行 SSH 会话",
      },
      both: {
        title: "同时打开",
        subtitle: "同时启动两个应用",
      },
    },
    errors: {
      noBookmark: "请从 1Password 的 SSH Bookmark 打开连接。",
      invalidUrl: "SSH Bookmark URL 格式无效。",
      missingHost: "SSH Bookmark 缺少主机名。",
      winscpMissing: "未找到 WinSCP。请先安装 WinSCP。",
      cyberduckMissing: "未找到 Cyberduck。请先安装 Cyberduck。",
      terminalMissing: "未找到 Windows Terminal。",
      opensshMissing: "未找到 Windows OpenSSH 客户端。",
      generic: "无法启动所选应用。",
    },
  },
  "en-US": {
    connectionInfoLabel: "SSH connection details",
    connectionTitle: "SSH Connection",
    port: "Port",
    agentConnected: "SSH agent pipe available",
    agentStatusMissing: "SSH agent pipe unavailable",
    agentStatusUnknown: "SSH agent status unknown",
    eyebrow: "1PASSWORD SSH",
    chooserTitle: "Choose how to open",
    chooserIntro: "Select an application to continue this connection",
    escapeHint: "Press Esc to cancel",
    escapeBackHint: "Press Esc to go back",
    shortcut: "Shortcut",
    launching: "Launching…",
    previewSuccess: "Preview mode: launch action is ready",
    appStarted: "Application launched",
    bothStarted: "Both applications launched",
    languageLabel: "Interface language",
    waitingBookmark: "Waiting for SSH Bookmark",
    openDiagnostics: "Diagnostics",
    diagnosticsTitle: "Environment diagnostics",
    diagnosticsIntro: "Check 1Password, OpenSSH, WinSCP, and Terminal readiness",
    diagnosticsBack: "Back",
    diagnosticsRefresh: "Refresh",
    diagnosticsListKeys: "List agent keys",
    diagnosticsCopy: "Copy report",
    diagnosticsCopied: "Report copied",
    diagnosticsCopyFailed: "Copy failed",
    diagnosticsLoading: "Running checks…",
    diagnosticsSshAddNote: "Listing keys may show a 1Password approval prompt",
    diagnosticsStatus: {
      ok: "OK",
      warn: "Warn",
      fail: "Fail",
      unknown: "Unknown",
    },
    diagnosticsChecks: diagnosticsChecksEn,
    actions: {
      winscp: {
        title: "WinSCP",
        subtitle: "Secure file transfer",
      },
      cyberduck: {
        title: "Cyberduck",
        subtitle: "Secure file transfer",
      },
      terminal: {
        title: "Windows Terminal",
        subtitle: "Command-line SSH session",
      },
      both: {
        title: "Open both",
        subtitle: "Launch both applications",
      },
    },
    errors: {
      noBookmark: "Open a connection from a 1Password SSH Bookmark.",
      invalidUrl: "The SSH Bookmark URL is invalid.",
      missingHost: "The SSH Bookmark does not include a host.",
      winscpMissing: "WinSCP was not found. Install WinSCP first.",
      cyberduckMissing: "Cyberduck was not found. Install Cyberduck first.",
      terminalMissing: "Windows Terminal was not found.",
      opensshMissing: "The Windows OpenSSH client was not found.",
      generic: "The selected application could not be launched.",
    },
  },
};

export function resolveInitialLocale() {
  const saved = window.localStorage.getItem("ssh-launcher-locale");
  if (supportedLocales.includes(saved)) return saved;
  return navigator.language.toLowerCase().startsWith("zh") ? "zh-CN" : "en-US";
}

export function localizeBackendError(error, locale) {
  const text = String(error || "");
  const t = messages[locale];

  if (locale === "zh-CN") return text || t.errors.generic;
  if (text.includes("SSH Bookmark URL 格式无效")) return t.errors.invalidUrl;
  if (text.includes("SSH Bookmark 缺少主机名")) return t.errors.missingHost;
  if (text.includes("SSH Bookmark") && text.includes("打开连接")) return t.errors.noBookmark;
  if (text.includes("Cyberduck") && (text.includes("未找到") || text.includes("无法启动"))) {
    return t.errors.cyberduckMissing;
  }
  if (text.includes("WinSCP.exe") || text.includes("未找到 WinSCP")) return t.errors.winscpMissing;
  if (text.includes("Windows Terminal") && text.includes("未找到")) return t.errors.terminalMissing;
  if (text.includes("OpenSSH") && text.includes("未找到")) return t.errors.opensshMissing;
  return t.errors.generic;
}

export function diagnosticCheckTitle(id, locale) {
  return messages[locale]?.diagnosticsChecks?.[id] || id;
}

export function diagnosticStatusLabel(status, locale) {
  return messages[locale]?.diagnosticsStatus?.[status] || status;
}
