export const themePreferences = ["system", "light", "dark"];

export function normalizeThemePreference(value) {
  const theme = String(value || "")
    .trim()
    .toLowerCase();
  if (themePreferences.includes(theme)) return theme;
  return "system";
}

export function resolveSystemAppearance() {
  if (typeof window === "undefined" || !window.matchMedia) return "light";
  return window.matchMedia("(prefers-color-scheme: dark)").matches
    ? "dark"
    : "light";
}

export function resolveEffectiveTheme(preference) {
  const normalized = normalizeThemePreference(preference);
  return normalized === "system" ? resolveSystemAppearance() : normalized;
}

/** Browser preview: ?theme=dark|light|system or --theme via hash not needed. */
export function resolvePreviewThemePreference() {
  try {
    const params = new URLSearchParams(window.location.search);
    if (params.has("theme")) {
      return normalizeThemePreference(params.get("theme"));
    }
  } catch {
    // Ignore malformed query strings in preview mode.
  }
  return "system";
}

export function applyDocumentTheme(preference) {
  const effective = resolveEffectiveTheme(preference);
  document.documentElement.dataset.theme = effective;
  document.documentElement.style.colorScheme = effective;
  return effective;
}
