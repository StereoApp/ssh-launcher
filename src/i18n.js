export const supportedLocales = ["zh-CN", "en-US"];

export const messages = {
  "zh-CN": {
    connectionInfoLabel: "SSH 连接信息",
    connectionTitle: "SSH 连接",
    port: "端口",
    agentConnected: "SSH Agent 已连接",
    eyebrow: "1PASSWORD SSH",
    chooserTitle: "选择打开方式",
    chooserIntro: "选择一个应用继续当前连接",
    escapeHint: "按 Esc 取消",
    shortcut: "快捷键",
    launching: "正在启动…",
    previewSuccess: "预览模式：启动操作正常",
    appStarted: "应用已启动",
    bothStarted: "两个应用已启动",
    languageLabel: "界面语言",
    waitingBookmark: "等待 SSH Bookmark",
    actions: {
      winscp: {
        title: "WinSCP",
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
      terminalMissing: "未找到 Windows Terminal。",
      opensshMissing: "未找到 Windows OpenSSH 客户端。",
      identityMissing: "无法确定唯一的 1Password 公钥，请刷新 SSH Bookmarks。",
      generic: "无法启动所选应用。",
    },
  },
  "en-US": {
    connectionInfoLabel: "SSH connection details",
    connectionTitle: "SSH Connection",
    port: "Port",
    agentConnected: "SSH Agent connected",
    eyebrow: "1PASSWORD SSH",
    chooserTitle: "Choose how to open",
    chooserIntro: "Select an application to continue this connection",
    escapeHint: "Press Esc to cancel",
    shortcut: "Shortcut",
    launching: "Launching…",
    previewSuccess: "Preview mode: launch action is ready",
    appStarted: "Application launched",
    bothStarted: "Both applications launched",
    languageLabel: "Interface language",
    waitingBookmark: "Waiting for SSH Bookmark",
    actions: {
      winscp: {
        title: "WinSCP",
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
      terminalMissing: "Windows Terminal was not found.",
      opensshMissing: "The Windows OpenSSH client was not found.",
      identityMissing:
        "A unique 1Password public key could not be resolved. Refresh your SSH Bookmarks.",
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
  if (text.includes("WinSCP.exe") || text.includes("未找到 WinSCP")) return t.errors.winscpMissing;
  if (text.includes("Windows Terminal") && text.includes("未找到")) return t.errors.terminalMissing;
  if (text.includes("OpenSSH") && text.includes("未找到")) return t.errors.opensshMissing;
  if (text.includes("1Password 公钥")) return t.errors.identityMissing;
  return t.errors.generic;
}
