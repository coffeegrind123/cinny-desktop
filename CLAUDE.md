# cinny-desktop

Cinny Matrix client packaged as a desktop app via Tauri v2. Cross-compiles to Windows from Linux using the GNU toolchain.

**Repos:**
- Desktop shell: `coffeegrind123/cinny-desktop` (this repo)
- Frontend (submodule): `coffeegrind123/cinny` branch `desktop-notifications`

## Fresh clone & build

```bash
git clone --recursive https://github.com/coffeegrind123/cinny-desktop.git
cd cinny-desktop

# Ensure submodule is on our branch
cd cinny
git fetch origin
git checkout desktop-notifications
npm ci
cd ..

# Install root deps and build for Windows
npm ci
source ~/.cargo/env
npm run tauri build -- --target x86_64-pc-windows-gnu
```

Output lands in `src-tauri/target/x86_64-pc-windows-gnu/release/`:
- `cinny.exe` — the application binary (34MB)
- `bundle/nsis/Cinny_x.y.z_x64-setup.exe` — NSIS installer (18MB)
- `bundle/nsis/Cinny_x.y.z_x64-setup.nsis.zip` — updater archive

## Submodule setup

The `cinny/` submodule points to `coffeegrind123/cinny` (not upstream `cinnyapp/cinny`). Our `desktop-notifications` branch contains the Tauri notification plugin integration, e2ee decryption handling, and message content formatting.

```bash
# First time after clone:
cd cinny
git fetch origin
git checkout desktop-notifications
npm ci
cd ..

# To update the submodule pointer in the main repo after pushing cinny changes:
git add cinny && git commit -m "Update cinny submodule"
```

**If git submodule update pulls the wrong commit:** It tracks `origin/dev` by default. Always explicitly checkout `desktop-notifications` in the submodule after `git submodule update --init`.

## Critical: working directory

**The Bash tool's CWD persists between commands.** After `cd cinny`, all subsequent Bash calls run from there. `npm run tauri` fails with "Missing script: tauri" because `cinny/package.json` doesn't have that script. Always verify `pwd` before build commands.

```bash
# Safe one-liner from anywhere:
source ~/.cargo/env && cd /opt/openclaude-src/cinny-desktop && npm run tauri build -- --target x86_64-pc-windows-gnu
```

**Also:** `source ~/.cargo/env` must run in every Bash call — shell state is not preserved.

## Cross-compiling to Windows from Linux

### Prerequisites

| Tool | Package / Install | Purpose |
|------|------------------|---------|
| Rust | `rustup` (rust-lang.org) | Tauri backend compiler |
| Windows Rust target | `rustup target add x86_64-pc-windows-gnu` | Cross-compile to Windows |
| mingw-w64 | `apt install mingw-w64` | GNU linker for Windows target |
| NSIS | `apt install nsis` | Build Windows installer |
| Tauri Linux deps | `apt install libwebkit2gtk-4.1-dev libappindicator3-dev patchelf libsoup-3.0-dev libjavascriptcoregtk-4.1-dev` | Tauri build dependencies |
| Node.js | >= 16.0.0 | Frontend build |

### Cargo linker config

Required at `src-tauri/.cargo/config.toml`:

```toml
[target.x86_64-pc-windows-gnu]
linker = "/usr/bin/x86_64-w64-mingw32-gcc"

[target.x86_64-pc-windows-gnu.tauri]
runner = ""
```

Without this, Cargo uses the system `cc` which produces Linux ELF binaries.

### Build flow

1. `npm run tauri build -- --target x86_64-pc-windows-gnu`
2. `beforeBuildCommand` fires: `cd cinny && npm run build` (Vite → `cinny/dist/`)
3. `tauri::generate_context!()` embeds `frontendDist` (`../cinny/dist`) — panics if dist missing
4. Cargo compiles Rust for `x86_64-pc-windows-gnu`
5. `makensis` creates the Windows installer `.exe`

### Cross-compile caveats

- **`tauri dev` won't work from Linux for Windows.** Needs native Windows display + WebView2.
- **MSI is ignored.** Only NSIS `.exe` installer is produced on Linux cross-compile.
- **Code signing is skipped.** Unsigned binaries are functional.
- **`__TAURI_BUNDLE_TYPE` patch fails on cross-compile.** Non-blocking warning.
- **Updater needs `TAURI_SIGNING_PRIVATE_KEY`.** Unset for local dev; warning is harmless.
- **`nsis_tauri_utils.dll` auto-downloaded** from GitHub on first build. Cached afterwards.
- **`makensis.exe` symlink needed:** `ln -sf /usr/bin/makensis /usr/bin/makensis.exe`

### Iteration (Linux → Windows)

1. Edit source in `cinny/src/` or `src-tauri/`
2. `source ~/.cargo/env && cd /opt/openclaude-src/cinny-desktop && npm run tauri build -- --target x86_64-pc-windows-gnu`
3. Copy `cinny.exe` or the NSIS installer to Windows
4. Install and run from Start Menu (required for toast notifications — see below)

For faster Rust checking: `cargo check --target x86_64-pc-windows-gnu` in `src-tauri/`.

## Windows Desktop Notifications

### Architecture

```
JS: sendNotification({title, body})
  → @tauri-apps/plugin-notification (npm package, installed in cinny/)
  → Tauri IPC → plugin:notification|notify
  → Rust: tauri_plugin_notification::init()
  → Windows Toast Notification API
  → Action Center toast popup
```

### Required setup (already done in this repo)

1. **`src-tauri/src/lib.rs`:** `.plugin(tauri_plugin_notification::init())`
2. **`src-tauri/tauri.conf.json`:** `"withGlobalTauri": true` — **critical**, Tauri v2 defaults this to `false`
3. **`src-tauri/capabilities/migrated.json`:** `"notification:default"` permission
4. **`cinny/package.json`:** `@tauri-apps/plugin-notification` dependency
5. **`cinny/src/app/utils/desktop-notifications.ts`:** Tauri/browser wrapper with `isTauri()`, `requestPermission()`, `sendNotification()`
6. **`cinny/src/app/pages/client/ClientNonUIFeatures.tsx`:** Uses `sendDesktopNotification()` with Matrix msgtype-aware body formatting; waits for `MatrixEventEvent.Decrypted` for e2ee rooms
7. **`cinny/src/app/hooks/usePermission.ts`:** Maps `denied→prompt` in Tauri (WebView2 default); 500ms polling fallback

### Pitfalls (in the order we hit them)

1. **`window.Notification` polyfill doesn't cover `requestPermission()` on Windows.** WebView2 doesn't support the Notification API natively. **Fix:** Use `@tauri-apps/plugin-notification` npm package directly.

2. **`window.__TAURI__` not injected.** Tauri v2 defaults `withGlobalTauri` to `false`. `isTauri()` always returned `false`, Enable button never appeared. **Fix:** Set `"withGlobalTauri": true` in tauri.conf.json.

3. **"Notification permission is blocked" on fresh install.** WebView2 defaults `Notification.permission` to `'denied'`. The Enable button only shows for `'prompt'`. **Fix:** `getNotificationState()` maps anything not `'granted'` to `'prompt'` when running in Tauri.

4. **"Nothing happens" when clicking Enable.** Windows desktop apps don't have a browser-style permission popup — notifications are managed in Windows Settings. The plugin's `requestPermission()` returns the OS state. On a properly installed app (NSIS → Start Menu shortcut → AppUserModelID), it returns `'granted'` immediately.

5. **Toast shows "New message from $name" instead of content.** The original code used a hardcoded body. **Fix:** Extract `msgtype` and `body` from Matrix event content, format per type (`m.text`/`m.image`/`m.video`/`m.audio`/`m.file`).

6. **Notifications show encrypted payload in e2ee rooms.** `Timeline` event fires before decryption completes. `getContent()` returns encrypted blob. **Fix:** Check `mEvent.isEncrypted()`, wait for `MatrixEventEvent.Decrypted`, then send notification.

7. **Submodule pulled wrong branch after `git submodule update --remote`.** Tracks `origin/dev` by default. **Fix:** Always explicitly `git checkout desktop-notifications` in the submodule.

### Windows AppUserModelID

Toast notifications require the app to have an AppUserModelID, which Windows assigns to *installed* applications (ones with Start Menu shortcuts). A loose `.exe` silently fails.

| Scenario | Works? |
|----------|--------|
| Installed via NSIS | Yes |
| Loose `.exe` | No — pin to Start or create shortcut manually |
| `tauri dev` on Windows | Shows PowerShell icon |

### Notification content format

| Message type | Toast body |
|-------------|-----------|
| `m.text` | `Alice: hello world` |
| `m.image` | `Alice sent an image: photo.jpg` |
| `m.video` | `Alice sent a video: clip.mp4` |
| `m.audio` | `Alice sent an audio clip: voice.ogg` |
| `m.file` | `Alice sent a file: document.pdf` |
| Encrypted | Same as above (waits for decryption first) |
| Unknown/empty | `New message from Alice` |

### Key files

| File | Role |
|------|------|
| `src-tauri/src/lib.rs` | Plugin init (notification, opener, localhost, window-state) |
| `src-tauri/tauri.conf.json` | `withGlobalTauri: true`, build config |
| `src-tauri/capabilities/migrated.json` | `notification:default` permission |
| `src-tauri/.cargo/config.toml` | Windows GNU linker config |
| `src-tauri/Cargo.toml` | Rust deps (tauri-plugin-notification, etc.) |
| `.gitmodules` | Submodule → `coffeegrind123/cinny` |
| `cinny/src/app/utils/desktop-notifications.ts` | Tauri/browser notification wrapper |
| `cinny/src/app/pages/client/ClientNonUIFeatures.tsx` | Runtime notification dispatch |
| `cinny/src/app/features/settings/notifications/SystemNotification.tsx` | Permission UI |
| `cinny/src/app/hooks/usePermission.ts` | Permission state with Tauri remapping |
