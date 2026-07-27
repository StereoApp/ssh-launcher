import {
  diagnosticCheckTitle,
  diagnosticStatusLabel,
} from "./i18n";

export function DiagnosticsView({
  t,
  locale,
  diagnostics,
  loading,
  sshAddBusy,
  message,
  onBack,
  onRefresh,
  onListKeys,
  onCopyReport,
}) {
  return (
    <>
      <div className="diagnostics-toolbar">
        <button type="button" className="ghost-button" onClick={onBack}>
          {t.diagnosticsBack}
        </button>
        <button
          type="button"
          className="ghost-button"
          onClick={onRefresh}
          disabled={loading}
        >
          {t.diagnosticsRefresh}
        </button>
        <button
          type="button"
          className="ghost-button"
          onClick={onListKeys}
          disabled={sshAddBusy || loading}
          title={t.diagnosticsSshAddNote}
        >
          {t.diagnosticsListKeys}
        </button>
        <button
          type="button"
          className="ghost-button"
          onClick={onCopyReport}
          disabled={!diagnostics?.reportText}
        >
          {t.diagnosticsCopy}
        </button>
      </div>

      <div className="diagnostics-list" role="list">
        {loading && !diagnostics ? (
          <p className="diagnostics-empty">{t.diagnosticsLoading}</p>
        ) : (
          (diagnostics?.checks || []).map((check) => (
            <div
              key={check.id}
              className={`diagnostics-row status-${check.status}`}
              role="listitem"
            >
              <span className="diagnostics-badge">
                {diagnosticStatusLabel(check.status, locale)}
              </span>
              <div className="diagnostics-copy">
                <strong>{diagnosticCheckTitle(check.id, locale)}</strong>
                <span title={check.detail}>{check.detail}</span>
                {check.hint ? (
                  <em className="diagnostics-hint">{check.hint}</em>
                ) : null}
              </div>
            </div>
          ))
        )}
      </div>

      <footer>
        <span className={message ? "message is-visible" : "message"}>
          {message || (loading ? t.diagnosticsLoading : "")}
        </span>
        <span className="hint">{t.escapeBackHint}</span>
      </footer>
    </>
  );
}
