# SSH Launcher

[简体中文](README.zh-CN.md) | English

A lightweight Windows launcher for 1Password SSH Bookmarks. When an `ssh://` link is opened, choose **WinSCP**, **Windows Terminal**, or **both**.

The interface supports English and Simplified Chinese. It follows the system language by default and can be switched manually.

> [!NOTE]
> This is a **vibe coding project**. The author defined the product idea, interaction decisions, and acceptance criteria, while most of the code and visual implementation was iterated in collaboration with AI. Code review, issue reports, and contributions are welcome.

## Screenshot

![SSH Launcher showing WinSCP, Windows Terminal, and Open both options](assets/screenshots/ssh-launcher.png)

## Features

- Parses `ssh://` links and displays the host, username, and port
- Opens WinSCP, Windows Terminal, or both
- Keyboard shortcuts: `W`, `T`, and `B`
- Closes automatically after an option is selected
- Keeps the chooser on top until a selection is made
- Finds WinSCP in common installation locations; `winscp.exe` does not need to be in `PATH`
- Reads WinSCP and Windows Terminal icons from locally installed applications at runtime
- English and Simplified Chinese interface
- Portable single-file executable

## Requirements

- Windows 10 or Windows 11
- [1Password 8 for Windows](https://1password.com/downloads/windows/) with SSH Agent enabled
- [WinSCP](https://winscp.net/) for file transfer
- Windows Terminal for terminal SSH sessions
- Windows OpenSSH Client
- Microsoft Edge WebView2 Runtime, normally included with Windows 10/11

## Quick start

### 1. Place the executable

Download `SSH-Launcher.exe` and keep it at a stable path, for example:

```text
C:\Tools\SSH-Launcher\SSH-Launcher.exe
```

No installer is required. To move it to another computer, copy the `.exe` and update its path in 1Password on that computer.

### 2. Enable the 1Password SSH Agent

In 1Password for Windows:

1. Open **Settings → Developer**.
2. Enable **Use the SSH Agent**.
3. Make sure the SSH private key for the target server is stored in 1Password.
4. To associate Bookmarks with their selected keys, enable **Generate SSH config files from 1Password SSH bookmarks** in the advanced SSH Agent settings.

### 3. Configure the SSH URL handler

In **Settings → Developer → SSH Agent → Advanced**:

1. Find **Open SSH URLs with**.
2. Select **Custom terminal command**.
3. Enter the following command, replacing the path with the actual executable location:

```text
"C:\Tools\SSH-Launcher\SSH-Launcher.exe" %s
```

Keep `%s` at the end. 1Password replaces it with the `ssh://` URL.

### 4. Create and open an SSH Bookmark

Create an SSH Bookmark in 1Password. Supported URL examples:

```text
ssh://user@example.com
ssh://user@example.com:2222
ssh://example.com
```

Open the Bookmark and select:

- **WinSCP** for a secure file-transfer session
- **Windows Terminal** for a command-line SSH session
- **Open both** to start both applications

You can also press `W`, `T`, or `B`. SSH Launcher closes automatically after launching the selected application(s).

## WinSCP does not need to be in PATH

SSH Launcher searches these locations:

1. System `PATH`
2. The current user's local application directory
3. `Program Files`
4. `Program Files (x86)`

A standard WinSCP installation should work without additional configuration. For a portable/custom WinSCP installation, add its directory to `PATH`.

## Languages

- On first launch, the Windows/WebView language determines whether English or Simplified Chinese is shown.
- Use the **中文 / EN** control in the top-right corner to switch languages.
- The preference is stored locally by WebView. A copied executable follows the language of the destination computer on its first launch.

## Portable migration

The release artifact is a standalone `.exe`. It does not depend on the source folder and does not store private keys.

1. Copy `SSH-Launcher.exe` to the destination computer.
2. Install and sign in to 1Password, WinSCP, and Windows Terminal.
3. Enable the 1Password SSH Agent.
4. Update **Custom terminal command** to the executable's new path.

Private keys remain managed by the 1Password SSH Agent. SSH Launcher never reads or exports them.

## Build from source

Prerequisites:

- Node.js 20+
- Stable Rust with the MSVC toolchain
- Microsoft C++ Build Tools
- WebView2

```powershell
git clone https://github.com/StereoApp/ssh-launcher.git
Set-Location ssh-launcher
npm install
npm run tauri:build
```

The portable executable is produced at:

```text
src-tauri\target\release\ssh-launcher.exe
```

Frontend-only build:

```powershell
npm run build
```

Development mode:

```powershell
npm run tauri:dev
```

## Troubleshooting

### The chooser does not appear

Verify that 1Password uses the full executable path, that the path is inside double quotes, and that `%s` remains at the end.

### WinSCP cannot be found

Install WinSCP with its official installer in the default location. For a portable copy, add the directory containing `WinSCP.exe` to `PATH`.

### Windows Terminal does not open

Confirm that Windows Terminal is installed and launches normally from the Start menu. SSH Launcher tries `wt.exe` first, then common installation locations.

### SSH uses the wrong key

Select the correct SSH Key in the 1Password SSH Bookmark and enable SSH config generation. The generated configuration can be inspected at:

```text
%USERPROFILE%\.ssh\1Password\config
```

### Is the window permanently always-on-top?

Only the chooser stays on top while waiting for a selection. It closes immediately after an option is selected and does not remain in the background.

## Security and privacy

- No telemetry
- No third-party network requests
- No reading, caching, or exporting of SSH private keys
- Credentials are handled by the 1Password SSH Agent and the system SSH client
- Third-party application icons are read from local installations at runtime and are not distributed in this repository

## Trademarks

1Password, WinSCP, Windows, and Windows Terminal are trademarks of their respective owners. This project is not affiliated with, authorized, or endorsed by those owners.

## License

[MIT License](LICENSE)
