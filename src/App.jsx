import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  CheckmarkCircleFilled,
  PersonRegular,
  PlugConnectedRegular,
  ServerRegular,
} from "@fluentui/react-icons";
import {
  localizeBackendError,
  messages,
  resolveInitialLocale,
  supportedLocales,
} from "./i18n";
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

// Action id "winscp" is the SFTP slot (shortcut W); the concrete GUI is CLI-selected.
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
        return {
          ...item,
          ...t.actions[item.id],
        };
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
    // When launched via 1Password's ssh:// handler, the OS window can appear
    // without keyboard focus. Re-request focus so shortcuts work without a click.
    const claimFocus = () => {
      window.focus();
      getCurrentWindow()
        .setFocus()
        .catch(() => {
          // Browser preview has no Tauri window bridge.
        });
    };

    claimFocus();
    const timers = [80, 250, 600].map((ms) => window.setTimeout(claimFocus, ms));
    return () => timers.forEach((id) => window.clearTimeout(id));
  }, []);

  useEffect(() => {
    invoke("get_theme_preference")
      .then((preference) => {
        setThemePreference(normalizeThemePreference(preference));
      })
      .catch(() => {
        // Browser preview keeps query-string / system theme preference.
      });

    invoke("get_sftp_preference")
      .then((preference) => {
        setSftpClient(normalizeSftpClient(preference));
      })
      .catch(() => {
        // Browser preview keeps query-string SFTP preference.
      });

    invoke("get_connection_info")
      .then((info) => {
        if (info) {
          setConnection(info);
          if (!info.valid) setRawError(info.error || "没有收到有效的 SSH Bookmark。");
        }
      })
      .catch(() => {
        // Vite's browser preview uses realistic demo data for visual QA.
      });

    invoke("get_app_icons")
      .then((result) => setIcons(result))
      .catch(() => {
        // Fluent UI fallbacks remain visible when an application is not installed.
      });
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
      if (String(error).includes("__TAURI__")) {
        setMessage(t.previewSuccess);
      } else {
        setRawError(String(error));
      }
    } finally {
      setBusy("");
    }
  };

  useEffect(() => {
    const handleKey = (event) => {
      if (event.key === "Escape") {
        getCurrentWindow().close().catch(() => {});
        return;
      }

      if (event.repeat || busy) return;
      const hit = actionDefinitions.find(
        (item) => item.key.toLowerCase() === event.key.toLowerCase(),
      );
      if (hit) launch(hit.id);
    };

    window.addEventListener("keydown", handleKey);
    return () => window.removeEventListener("keydown", handleKey);
  }, [busy, connection.valid, locale, sftpClient]);

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
        <div className="agent-status">
          <CheckmarkCircleFilled />
          <span>{t.agentConnected}</span>
        </div>
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
          <h2>{t.chooserTitle}</h2>
          <p className="intro">{t.chooserIntro}</p>
        </header>

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
          <span className={displayedMessage ? "message is-visible" : "message"}>
            {displayedMessage}
          </span>
          <span className="hint">{t.escapeHint}</span>
        </footer>
      </section>
    </main>
  );
}

export default App;
