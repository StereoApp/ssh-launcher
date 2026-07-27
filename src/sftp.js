export const sftpClients = ["winscp", "cyberduck"];

export function normalizeSftpClient(value) {
  const client = String(value || "")
    .trim()
    .toLowerCase();
  if (sftpClients.includes(client)) return client;
  return "winscp";
}

/** Browser preview: ?sftp=winscp|cyberduck */
export function resolvePreviewSftpClient() {
  try {
    const params = new URLSearchParams(window.location.search);
    if (params.has("sftp")) {
      return normalizeSftpClient(params.get("sftp"));
    }
  } catch {
    // Ignore malformed query strings in preview mode.
  }
  return "winscp";
}
