import { useCallback, useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  CheckmarkCircleFilled,
  DismissCircleFilled,
  PersonRegular,
  PlugConnectedRegular,
  ServerRegular,
  WarningFilled,
} from "@fluentui/react-icons";
import {
  localizeBackendError,
  messages,
  resolveInitialLocale,
  supportedLocales,
} from "./i18n";
import {
  agentPipeStatus,
  createMockDiagnosticsReport,
  isTauriBridgeMissing,
  resolvePreviewOpenDiagnostics,
} from "./diagnostics";
import { DiagnosticsView } from "./DiagnosticsView";
import {
  normalizeSftpClient,
  resolvePreviewSftpClient,
} from "./sftp";
import {
  applyDocumentTheme,
  normalizeThemePreference,
  resolvePreviewThemePreference,
} from "./theme";
import "./styles.css";

const demoConnection = {
  valid: true,
  host: "server.example.com",
  user: "demo",
  port: 2222,
  displayTarget: "demo@server.example.com:2222",
};

const actionDefinitions = [
  { id: "winscp", key: "W" },
  { id: "terminal", key: "T" },
  { id: "both", key: "B" },
];

function IconImage({ src, fallback: Fallback = PlugConnectedRegular }) {
  return src ? <img src={src} alt="" /> : <Fallback aria-hidden="true" />;
}

function AppIcon({ type, icons, sftpClient }) {
  const sftpIcon = icons[sftpClient] || icons.winscp;

  if (type === "both") {
    return (
      <span className="app-icon app-icon--combined" aria-hidden="true">
        <span className="combined-tile combined-tile--sftp">
          <IconImage src={sftpIcon} fallback={ServerRegular} />
        </span>
        <span className="combined-tile combined-tile--terminal">
          <IconImage src={icons.terminal} />
        </span>
      </span>
    );
  }

  if (type === "winscp") {
    return (
      <span className="app-icon" aria-hidden="true">
        <IconImage src={sftpIcon} fallback={ServerRegular} />
      </span>
    );
  }

  return (
    <span className="app-icon" aria-hidden="true">
      <IconImage src={icons.terminal} fallback={PlugConnectedRegular} />
    </span>
  );
}

function AgentStatus({ status, labels }) {
  if (status === "fail") {
    return (
      <div className="agent-status agent-status--fail">
        <DismissCircleFilled />
        <span>{labels.fail}</span>
      </div>
    );
  }
  if (status === "warn" || status === "unknown") {
    return (
      <div className="agent-status agent-status--warn">
        <WarningFilled />
        <span>{labels.warn}</span>
      </div>
    );
  }
  return (
    <div className="agent-status">
      <CheckmarkCircleFilled />
      <span>{labels.ok}</span>
    </div>
  );
}

export function App() {
  const [locale, setLocale] = useState(resolveInitialLocale);
  const [themePreference, setThemePreference] = useState(
    resolvePreviewThemePreference,
  );
  const [sftpClient, setSftpClient] = useState(resolvePreviewSftpClient);
  const [connection, setConnection] = useState(demoConnection);
  const [icons, setIcons] = useState({
    winscp: null,
    cyberduck: null,
    terminal: null,
  });
  const [view, setView] = useState(
    resolvePreviewOpenDiagnostics() ? "diagnostics" : "chooser",
  );
  const [diagnostics, setDiagnostics] = useState(null);
  const [diagnosticsLoading, setDiagnosticsLoading] = useState(false);
  const [sshAddBusy, setSshAddBusy] = useState(false);
  const [active, setActive] = useState("both");
  const [busy, setBusy] = useState("");
  const [message, setMessage] = useState("");
  const [rawError, setRawError] = useState("");
  const t = messages[locale];

  const actions = useMemo(
    () =>
      actionDefinitions.map((item) => {
        if (item.id === "winscp") {
          const sftpCopy = t.actions[sftpClient] || t.actions.winscp;
          return { ...item, ...sftpCopy };
        }
        return { ...item, ...t.actions[item.id] };
      }),
    [t, sftpClient],
  );

  const target = useMemo(() => {
    if (!connection.valid) return t.waitingBookmark;
    if (connection.displayTarget) return connection.displayTarget;
    return connection.port === 22
      ? `${connection.user}@${connection.host}`
      : `${connection.user}@${connection.host}:${connection.port}`;
  }, [connection, t.waitingBookmark]);

  const displayedMessage = rawError
    ? localizeBackendError(rawError, locale)
    : message;

  const pipeStatus = agentPipeStatus(diagnostics);

  const loadDiagnostics = useCallback(async () => {
    setDiagnosticsLoading(true);
    try {
      const report = await invoke("run_diagnostics");
      setDiagnostics(report);
    } catch (error) {
      if (isTauriBridgeMissing(error)) {
        setDiagnostics(createMockDiagnosticsReport(connection, sftpClient));
      } else {
        setDiagnostics({
          sftpClient,
          connectionValid: connection.valid,
          displayTarget: connection.displayTarget || "",
          checks: [
            {
              id: "openssh",
              status: "fail",
              detail: String(error),
              hint: "Diagnostics invoke failed",
            },
          ],
          reportText: `diagnostics failed: ${error}`,
        });
      }
    } finally {
      setDiagnosticsLoading(false);
    }
  }, [connection, sftpClient]);

  useEffect(() => {
    document.documentElement.lang = locale;
    window.localStorage.setItem("ssh-launcher-locale", locale);
  }, [locale]);

  useEffect(() => {
    applyDocumentTheme(themePreference);
    if (themePreference !== "system" || !window.matchMedia) return undefined;
    const media = window.matchMedia("(prefers-color-scheme: dark)");
    const onChange = () => applyDocumentTheme("system");
    if (media.addEventListener) {
      media.addEventListener("change", onChange);
      return () => media.removeEventListener("change", onChange);
    }
    media.addListener(onChange);
    return () => media.removeListener(onChange);
  }, [themePreference]);

  useEffect(() => {
    const claimFocus = () => {
      window.focus();
      getCurrentWindow()
        .setFocus()
        .catch(() => {});
    };
    claimFocus();
    const timers = [80, 250, 600].map((ms) => window.setTimeout(claimFocus, ms));
    return () => timers.forEach((id) => window.clearTimeout(id));
  }, []);

  useEffect(() => {
    invoke("get_theme_preference")
      .then((preference) => setThemePreference(normalizeThemePreference(preference)))
      .catch(() => {});
    invoke("get_sftp_preference")
      .then((preference) => setSftpClient(normalizeSftpClient(preference)))
      .catch(() => {});
    invoke("get_open_diagnostics")
      .then((open) => {
        if (open) setView("diagnostics");
      })
      .catch(() => {});
    invoke("get_connection_info")
      .then((info) => {
        if (info) {
          setConnection(info);
          if (!info.valid) setRawError(info.error || "没有收到有效的 SSH Bookmark。");
        }
      })
      .catch(() => {});
    invoke("get_app_icons")
      .then((result) => setIcons(result))
      .catch(() => {});
  }, []);

  useEffect(() => {
    if (view === "diagnostics") loadDiagnostics();
  }, [view, loadDiagnostics]);

  // Warm agent status for the chooser left rail once per session.
  useEffect(() => {
    let cancelled = false;
    invoke("run_diagnostics")
      .then((report) => {
        if (!cancelled) setDiagnostics((current) => current || report);
      })
      .catch((error) => {
        if (cancelled) return;
        if (isTauriBridgeMissing(error)) {
          setDiagnostics((current) =>
            current || createMockDiagnosticsReport(connection, sftpClient),
          );
        }
      });
    return () => {
      cancelled = true;
    };
    // Only warm once on mount; full refresh happens in diagnostics view.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const launch = async (choice) => {
    if (busy || !connection.valid) return;
    setActive(choice);
    setBusy(choice);
    setMessage("");
    setRawError("");
    try {
      await invoke("launch_choice", { choice });
      setMessage(choice === "both" ? t.bothStarted : t.appStarted);
    } catch (error) {
      if (String(error).includes("__TAURI__")) setMessage(t.previewSuccess);
      else setRawError(String(error));
    } finally {
      setBusy("");
    }
  };

  const listAgentKeys = async () => {
    setSshAddBusy(true);
    setMessage("");
    try {
      // Backend returns a full report including ssh-add (single report owner).
      const report = await invoke("run_ssh_add_check");
      setDiagnostics(report);
    } catch (error) {
      if (isTauriBridgeMissing(error)) {
        setMessage(t.diagnosticsSshAddNote);
      } else {
        setMessage(String(error));
      }
    } finally {
      setSshAddBusy(false);
    }
  };

  const copyReport = async () => {
    const text = diagnostics?.reportText || "";
    if (!text) return;
    try {
      await navigator.clipboard.writeText(text);
      setMessage(t.diagnosticsCopied);
    } catch {
      setMessage(t.diagnosticsCopyFailed);
    }
  };

  useEffect(() => {
    const handleKey = (event) => {
      if (event.key === "Escape") {
        if (view === "diagnostics") {
          setView("chooser");
          setMessage("");
          return;
        }
        getCurrentWindow().close().catch(() => {});
        return;
      }
      if (view !== "chooser" || event.repeat || busy) return;
      const hit = actionDefinitions.find(
        (item) => item.key.toLowerCase() === event.key.toLowerCase(),
      );
      if (hit) launch(hit.id);
    };
    window.addEventListener("keydown", handleKey);
    return () => window.removeEventListener("keydown", handleKey);
  }, [busy, connection.valid, locale, sftpClient, view]);

  return (
    <main className="launcher-shell">
      <aside className="connection-panel" aria-label={t.connectionInfoLabel}>
        <div className="connection-mark" aria-hidden="true">
          <IconImage src={icons.terminal} />
        </div>

        <h1>{t.connectionTitle}</h1>
        <div className="divider" />

        <dl className="connection-details">
          <div>
            <dt><ServerRegular /></dt>
            <dd title={target}>{target}</dd>
          </div>
          <div>
            <dt><PersonRegular /></dt>
            <dd>{connection.user || "—"}</dd>
          </div>
          <div>
            <dt><PlugConnectedRegular /></dt>
            <dd>{t.port} {connection.port}</dd>
          </div>
        </dl>

        <div className="divider divider--status" />
        <AgentStatus
          status={pipeStatus}
          labels={{
            ok: t.agentConnected,
            warn: t.agentStatusUnknown,
            fail: t.agentStatusMissing,
          }}
        />
      </aside>

      <section className="action-panel">
        <header>
          <div className="header-topline">
            <p className="eyebrow">{t.eyebrow}</p>
            <div className="language-switcher" aria-label={t.languageLabel}>
              {supportedLocales.map((item) => (
                <button
                  key={item}
                  type="button"
                  className={locale === item ? "is-selected" : ""}
                  aria-pressed={locale === item}
                  onClick={() => setLocale(item)}
                >
                  {item === "zh-CN" ? "中文" : "EN"}
                </button>
              ))}
            </div>
          </div>
          <h2>{view === "diagnostics" ? t.diagnosticsTitle : t.chooserTitle}</h2>
          <p className="intro">
            {view === "diagnostics" ? t.diagnosticsIntro : t.chooserIntro}
          </p>
        </header>

        {view === "chooser" ? (
          <>
            <div className="action-list" role="list">
              {actions.map((item) => (
                <button
                  key={item.id}
                  className={`action-card ${active === item.id ? "is-active" : ""}`}
                  type="button"
                  onMouseEnter={() => setActive(item.id)}
                  onFocus={() => setActive(item.id)}
                  onClick={() => launch(item.id)}
                  disabled={Boolean(busy) || !connection.valid}
                  aria-label={`${item.title}, ${item.subtitle}, ${t.shortcut} ${item.key}`}
                >
                  <AppIcon type={item.id} icons={icons} sftpClient={sftpClient} />
                  <span className="action-copy">
                    <strong>{item.title}</strong>
                    <span>{busy === item.id ? t.launching : item.subtitle}</span>
                  </span>
                  <kbd>{item.key}</kbd>
                </button>
              ))}
            </div>

            <footer>
              <button
                type="button"
                className="text-link"
                onClick={() => {
                  setMessage("");
                  setView("diagnostics");
                }}
              >
                {t.openDiagnostics}
              </button>
              <span className={displayedMessage ? "message is-visible" : "message"}>
                {displayedMessage}
              </span>
              <span className="hint">{t.escapeHint}</span>
            </footer>
          </>
        ) : (
          <DiagnosticsView
            t={t}
            locale={locale}
            diagnostics={diagnostics}
            loading={diagnosticsLoading}
            sshAddBusy={sshAddBusy}
            message={displayedMessage}
            onBack={() => {
              setMessage("");
              setView("chooser");
            }}
            onRefresh={loadDiagnostics}
            onListKeys={listAgentKeys}
            onCopyReport={copyReport}
          />
        )}
      </section>
    </main>
  );
}

export default App;
