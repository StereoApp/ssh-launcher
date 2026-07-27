# AGENTS.md — SSH Launcher

Agent guidance for this repository. Prefer this file over ad-hoc assumptions when changing product behavior, UI, or release packaging.

## Product

**SSH Launcher** is a lightweight Windows desktop chooser for [1Password](https://1password.com/) SSH Bookmarks. When the user opens an `ssh://` link (typically from 1Password’s custom terminal command), this app shows connection details and lets them open:

| Action id | Shortcut | Opens |
|-----------|----------|--------|
| `winscp` | `W` | Selected SFTP GUI (default **WinSCP**; optional **Cyberduck**) |
| `terminal` | `T` | Windows Terminal + OpenSSH |
| `both` | `B` | SFTP GUI and Windows Terminal |

After a successful launch choice, the window closes. Esc cancels / closes. The window stays always-on-top until the user acts.

- Repo: https://github.com/StereoApp/ssh-launcher
- License: MIT
- Version: keep `package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json` in sync

## Durable product decisions

Do not reverse these without an explicit user decision:

1. **Final product is a Tauri Windows desktop app**, not WinForms or a pure web page.
2. **Release deliverable** is a **portable single-file `.exe`** that can be copied between Windows 10/11 PCs (`npm run tauri:build` → `src-tauri/target/release/ssh-launcher.exe`). Bundle installers are disabled (`bundle.active: false`, `--no-bundle`).
3. **Visual target** is the light, 1Password-inspired split-pane UI in `design-reference.png` (fixed **760 × 500** window).
4. **Three actions only**: SFTP GUI, Windows Terminal, Open both. The combined action must show the **real SFTP app and Windows Terminal icons** together.
5. **SFTP GUI is mutually exclusive**: **WinSCP** (default) or **Cyberduck**, selected only via CLI (`--sftp=winscp|cyberduck`, or `--winscp` / `--cyberduck`). Not an in-app toggle; UI always shows one SFTP card.
6. **CLI contract**: accept a 1Password SSH Bookmark URL as a meaningful argument (`ssh://...`). 1Password is configured as: `"…\SSH-Launcher.exe" %s` (optional flags before `%s`).
7. **Never handle private keys**. Auth stays with the 1Password SSH Agent; this app only launches tools with the parsed host/user/port (WinSCP may pass a bookmark public key path for identity matching only).
8. **i18n**: English (`en-US`) and Simplified Chinese (`zh-CN`). Default from system/WebView language; user can switch; preference in `localStorage` key `ssh-launcher-locale`.
9. **Theme**: light / dark. Default **follow system**. Override via CLI (`--theme=system|light|dark`, `--dark`, `--light`). Not a durable in-app toggle unless product decision changes.

## Architecture

```
┌──────────────────────────── Tauri 2 (Windows) ────────────────────────────┐
│  CLI args → parse ssh:// + theme + sftp client → AppState                  │
│  Commands: get_connection_info | get_theme_preference | get_sftp_preference │
│            get_app_icons | launch_choice                                   │
│  Process launch: WinSCP / Cyberduck / wt.exe + ssh.exe                     │
└───────────────────────────────────┬────────────────────────────────────────┘
                                    │ invoke()
┌───────────────────────────────────▼────────────────────────────────────────┐
│  React 19 + Vite frontend (`src/`)                                         │
│  Left: connection panel · Right: action list · Locale switcher · theme CSS │
│  Browser preview: demo connection if Tauri invoke is unavailable           │
└────────────────────────────────────────────────────────────────────────────┘
```

| Layer | Path | Role |
|-------|------|------|
| Frontend UI | `src/App.jsx`, `src/styles.css`, `src/i18n.js`, `src/theme.js`, `src/sftp.js`, `src/main.jsx` | Chooser UI, shortcuts, locale, theme, SFTP label |
| Desktop backend | `src-tauri/src/main.rs` | URL/theme/SFTP parse, icon extract, process launch |
| Tauri config | `src-tauri/tauri.conf.json`, `capabilities/default.json` | Window chrome, capabilities |
| Web / Sites preview | `worker/index.js`, `.openai/hosting.json`, `scripts/prepare-sites-build.mjs` | Optional Sites handoff for UI preview |

### Backend Tauri commands

- `get_connection_info` → `ConnectionInfo` (camelCase JSON: `valid`, `sshUrl`, `host`, `user`, `port`, `displayTarget`, `error`)
- `get_theme_preference` → `"system"` \| `"light"` \| `"dark"` (from CLI; default `system`)
- `get_sftp_preference` → `"winscp"` \| `"cyberduck"` (from CLI; default `winscp`)
- `get_app_icons` → `{ winscp?, cyberduck?, terminal? }` as data-URL images from installed apps
- `launch_choice({ choice })` → launches then closes the window; `choice` is `winscp` \| `terminal` \| `both` (`winscp` = configured SFTP GUI)

### Frontend preview behavior

In plain Vite/browser (no Tauri), `invoke` fails and the UI keeps **demo** connection data (`demo@server.example.com:2222`) so visual work still works. Launch in that mode surfaces a preview success message rather than starting real apps.

## Prototype / agent workflow

1. **Run the preview yourself** when validating UI: start the local server and open it in the environment browser. Do not only tell the user how to start it if you can run it.
2. **Substantial visual changes**: treat `design-reference.png` (and any selected mock) as source of truth for layout, density, spacing, color, typography, and hierarchy. Record durable design decisions in this file.
3. **App UI lives in `src/`**. Do not break Sites handoff files:
   - `.openai/hosting.json`
   - `worker/index.js`
   - `scripts/prepare-sites-build.mjs`
   - `tests/sites-worker.test.mjs`
4. **Before a Sites handoff**: `npm run build` and `npm run test:sites` (or the project’s Sites prepare + test flow). Artifacts must include:
   - `dist/client/index.html`
   - `dist/server/index.js`
   - `dist/.openai/hosting.json`

## Commands

Prerequisites for desktop builds: Node.js 20+, Rust stable (MSVC), Microsoft C++ Build Tools, WebView2.

```powershell
npm install

# Frontend only (browser preview / Sites client)
npm run dev          # Vite on http://127.0.0.1:1420
npm run build        # → dist/client

# Desktop
npm run tauri:dev    # full app with Rust backend
npm run tauri:build  # portable exe (no installer bundle)
```

Portable binary path:

```text
src-tauri\target\release\ssh-launcher.exe
```

Manual desktop smoke (real launch path):

```powershell
.\src-tauri\target\release\ssh-launcher.exe "ssh://demo@server.example.com:2222"
.\src-tauri\target\release\ssh-launcher.exe --theme=dark "ssh://demo@server.example.com:2222"
.\src-tauri\target\release\ssh-launcher.exe --sftp=cyberduck "ssh://demo@server.example.com:2222"
.\src-tauri\target\release\ssh-launcher.exe --cyberduck "ssh://demo@server.example.com:2222"
```

## Release (GitHub Actions)

Workflow: `.github/workflows/release.yml`

- **Trigger**: push tag `v*` (e.g. `v1.1.1`), or manual **workflow_dispatch**
- **Runner**: `windows-latest` only (Windows product)
- **Build**: `npm ci` → `npm run tauri:build` → portable `SSH-Launcher.exe` + `SHA256SUMS.txt`
- **Publish**: `gh release create` / `gh release upload` with `GITHUB_TOKEN`
- **Version gate**: tag `vX.Y.Z` must match `package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json`

```powershell
# after bumping the three version fields and committing
git tag v1.1.1
git push origin v1.1.1
```

## Layout of important files

```text
src/
  App.jsx          # UI shell, actions, shortcuts, invoke wiring
  i18n.js          # zh-CN / en-US strings + backend error localization
  sftp.js          # SFTP client preference normalize / browser preview
  styles.css       # light split-pane chrome matching design-reference.png
  theme.js
  main.jsx
src-tauri/
  src/main.rs      # all native logic (prefer keep single-file unless it grows a lot)
  tauri.conf.json  # fixed 760×500, alwaysOnTop, no resize
  icons/icon.ico
assets/
  screenshots/     # README screenshots
design-reference.png
design-qa.md       # last design verification notes
worker/            # SPA fallback worker for Sites
scripts/prepare-sites-build.mjs
tests/sites-worker.test.mjs
```

## UI / design constraints

- **Window**: 760×500 logical px, non-resizable, centered, always on top, standard decorations.
- **Structure**: left connection panel (host / user / port / agent status), right action panel (eyebrow, locale switcher, three action cards, footer message + Esc hint).
- **Icons**: prefer live icons from installed SFTP GUI / Windows Terminal via backend; Fluent UI icons are fallbacks only. Combined action uses stacked real icons for the **selected** SFTP client + Terminal.
- **Keyboard**: `W` / `T` / `B` / `Esc`. Cards show the shortcut as `<kbd>`. `W` always means the configured SFTP GUI.
- **Colors / type**: light 1Password-inspired palette is the default visual target; dark theme uses the same layout with CSS variables (`data-theme="light"|"dark"`). Default appearance follows the OS; CLI can force light/dark.
- Visual QA notes: `design-qa.md`.

## Implementation conventions

- **Frontend**: React function components; strings only via `i18n.js` (no hard-coded user-facing English/Chinese in JSX except language switcher labels `中文` / `EN`).
- **Backend errors**: may be Chinese from Rust; map known phrases through `localizeBackendError` in `i18n.js` so EN UI stays consistent.
- **WinSCP discovery**: PATH, then common install dirs (`LocalAppData`, `Program Files`, `Program Files (x86)`). Do not require WinSCP on PATH for default installs.
- **Cyberduck discovery**: PATH, then `Program Files\Cyberduck`, `Program Files (x86)\Cyberduck`, `LocalAppData\Programs\Cyberduck`. Launch with `sftp://…` URL (protocol-handler style).
- **Terminal**: prefer `wt.exe`; fall back to packaged Terminal locations.
- **Security / privacy**: no storage of SSH private keys; only parse URL and launch external tools.
- **Release profile** (Cargo): size-oriented (`opt-level = "s"`, LTO, strip, `panic = "abort"`). Keep portable exe lean.
- **Scope discipline**: change only what the task needs. Do not drive-by refactor README, Sites plumbing, or unrelated assets.

## Acceptance checklist (feature work)

- [ ] Still accepts `ssh://user@host[:port]` as CLI argument and shows correct `displayTarget`
- [ ] Three actions work; shortcuts `W`/`T`/`B`; Esc closes
- [ ] `--sftp=winscp` (default) and `--sftp=cyberduck` / `--cyberduck` switch the SFTP card label, icon, and launch target
- [ ] Combined action shows real icons for the selected SFTP app + Terminal when installed
- [ ] zh-CN and en-US remain complete for new strings
- [ ] Browser preview still shows demo data without Tauri (`?sftp=cyberduck` optional)
- [ ] `npm run tauri:build` still produces a single portable exe when desktop is in scope
- [ ] Sites-critical files untouched unless the task is Sites-related

## Out of scope (unless explicitly requested)

- macOS / Linux builds
- Installer / MSI / Store packaging
- Embedding or managing SSH private keys
- Extra launch targets beyond the selected SFTP GUI and Windows Terminal
- Showing both WinSCP and Cyberduck as simultaneous UI actions
- Resizable or multi-window UI
- In-window theme or SFTP-client picker (CLI-only unless product decision changes)
