# Prinny Client <img align="right" width="90" height="128" src="src-tauri/icons/prinny-logo.png" alt="Prinny" />

[![Build](https://github.com/coffeegrind123/prinny-client/actions/workflows/build.yml/badge.svg)](https://github.com/coffeegrind123/prinny-client/actions/workflows/build.yml)
[![Latest](https://img.shields.io/github/v/release/coffeegrind123/prinny-client?label=latest)](https://github.com/coffeegrind123/prinny-client/releases/latest)
[![Downloads](https://img.shields.io/github/downloads/coffeegrind123/prinny-client/total?label=downloads)](https://github.com/coffeegrind123/prinny-client/releases)

A Matrix chat client that actually feels native. Prinny Client is [Cinny](https://cinny.in) packaged with [Tauri v2](https://v2.tauri.app), shipping on Windows, macOS, Linux, and Android. Hard fork of [cinnyapp/cinny-desktop](https://github.com/cinnyapp/cinny-desktop). The frontend lives at [coffeegrind123/cinny](https://github.com/coffeegrind123/cinny) on the `desktop-notifications` branch.

## Download

Everything is on [GitHub Releases](https://github.com/coffeegrind123/prinny-client/releases).

| Platform | Download |
|----------|----------|
| Windows | [MSI](https://github.com/coffeegrind123/prinny-client/releases/latest/download/Prinny_desktop-x86_64.msi) · [NSIS setup](https://github.com/coffeegrind123/prinny-client/releases/latest/download/Prinny_desktop-x86_64-setup.exe) |
| macOS | [DMG](https://github.com/coffeegrind123/prinny-client/releases/latest/download/Prinny_desktop-universal.dmg) |
| Linux | [AppImage](https://github.com/coffeegrind123/prinny-client/releases/latest/download/Prinny_desktop-x86_64.AppImage) · [deb](https://github.com/coffeegrind123/prinny-client/releases/latest/download/Prinny_desktop-x86_64.deb) |
| Android | [APK](https://github.com/coffeegrind123/prinny-client/releases/latest/download/prinny-android-universal.apk) (sideload) |

Desktop builds update themselves via `release.json`. Android checks on launch and downloads new APKs when available.

## Features

### Desktop notifications

| Feature | Description |
|---------|-------------|
| Native OS toasts | `tauri-plugin-notification` backed. Sender avatar, message body, click to jump to room |
| E2EE-aware | Waits for `MatrixEventEvent.Decrypted` before notifying — no ciphertext in toasts |
| Content formatting | Per-msgtype body: `m.text` shows sender + body, `m.image` shows "sent an image: filename" |
| Permission handling | Tauri WebView2 maps `denied`→`prompt` so Enable button always shows. 500ms polling fallback via `isPermissionGranted()` |
| Notification actions | "Open" button on toast, `onAction` listener navigates to room |
| Windows AppUserModelID | Toast notifications require NSIS install (Start Menu shortcut). Loose `.exe` silently skips |

### Message composer

| Feature | Description |
|---------|-------------|
| Markdown editor | Slate-based rich text with inline formatting toggles |
| Editor toolbar | Bold, italic, underline, strikethrough, inline code, spoiler, block quote, code block, ordered/unordered lists, headings 1-3 |
| Keyboard shortcuts | `Ctrl+B` bold, `Ctrl+I` italic, `Ctrl+U` underline, `Ctrl+S` strikethrough, `Ctrl+[` inline code, `Ctrl+H` spoiler, `Ctrl+7` ordered list, `Ctrl+8` unordered list, `Ctrl+'` block quote, `Ctrl+;` code block, `Ctrl+1/2/3` headings |
| Autocomplete | `:emoji:` with Enter/Tab to complete first match. `/command`, `@user`, `#room` all auto-complete on Enter |
| First result highlighted | Focus ring on first autocomplete result so you can see what Enter will pick |
| `Ctrl+/` shortcut panel | Discord-style modal showing all 49 keybinds grouped by category with current keycap badges |
| Configurable keybinds | Settings → Keybinds: click any binding, press new combo to rebind. Reset per-binding or Reset All. Editor hotkeys and tooltips reflect current bindings |
| Send on Enter | `enterForNewline` setting: Enter sends (default) or adds newline |
| Slash commands | `/join`, `/leave`, `/kick`, `/ban`, `/invite`, `/me`, `/nick`, `/topic`, `/shrug`, `/tableflip` with autocomplete |

### Message timeline

| Feature | Description |
|---------|-------------|
| Reply highlights | Yellow left border + amber background when someone replies to your message |
| Timestamp position | Discord-style: timestamp next to username, `@user:server` on right only on hover (Modern layout) |
| NEW badge | Right-aligned green "NEW" badge with full-width green line divider (Discord-style) |
| Message layouts | Modern (default), Compact, Bubble |
| Message spacing | Configurable 0-500px gaps between messages |
| Read receipts | Two modes (Settings → General → Read Receipt Style): Cinny default shows "is following the conversation" text live, Element-style shows tiny avatar dots at each person's last-read position |
| Mark as Unread | Right-click any message → sets read marker to previous event. Room gets unread badge, green NEW line at marked position |
| Message context menu | Right-click: Add Reaction, View Reactions, Reply, Reply in Thread, Edit, Read Receipts, Copy Link, Pin, Mark Unread, Delete, Report |
| Hide read receipts | `hideActivity` setting disables typing indicators and read receipts |
| Hide membership events | `hideMembershipEvents` setting hides join/leave clutter |
| Hide nick/avatar events | `hideNickAvatarEvents` setting |
| Media auto-load | `mediaAutoLoad` setting |
| URL preview | `urlPreview` + `encUrlPreview` settings |

### Sidebar and navigation

| Feature | Description |
|---------|-------------|
| Presence indicators | Online/busy/away dots on DM avatars and member list (`useUserPresence` hook) |
| DM unread filter | "..." menu next to Direct Messages → "Show unread only" collapses list to rooms with unread messages |
| Mobile swipe gestures | Right-edge swipe opens room, left-edge swipe goes back (Discord-style). Works on Home, Direct, and Space screens |
| Quick switcher | `Ctrl+K` search (rooms, DMs, spaces) with fuzzy matching |
| Server/channel navigation | `Alt+Up/Down` channels, `Ctrl+Alt+Up/Down` servers, `Alt+Shift+Up/Down` unread channels |
| Space tabs | Sidebar tabs for Home, Direct Messages, and Spaces |

### Desktop shell

| Feature | Description |
|---------|-------------|
| System tray | Minimize to tray on close (`minimizeToTray` setting). Tray icon with Show/Quit menu |
| Taskbar badge | macOS Dock + Linux taskbar: native `set_badge_count()` red badge with unread count |
| Windows taskbar overlays | `ITaskbarList3::SetOverlayIcon` with numbered 1-9 and 9+ ICO overlays |
| Desktop updater | `tauri-plugin-updater` checks `release.json` on GitHub Releases |
| Window state | `tauri-plugin-window-state` remembers size, position, maximized state |
| Global shortcuts | `global-shortcut` plugin registered (available for future use) |
| Cross-platform | Single Rust codebase, conditional compilation for Windows/macOS/Linux/Android |

### Mobile (Android)

| Feature | Description |
|---------|-------------|
| Foreground service | Persistent "Connected to Matrix" notification keeps WebSocket alive on GrapheneOS and de-Googled devices |
| Importance LOW | Notification sits at low priority, no sound, non-dismissible |
| Network detection | `ConnectivityManager.NetworkCallback` detects WiFi/mobile switch, reconnects automatically |
| Watchdog | `AlarmManager` 15-min wake during doze |
| UnifiedPush | `android-connector:3.0.10` via JitPack. No Firebase, no Play Services needed |
| Push notifications | `UnifiedPushReceiver` posts system notification when app is killed, forwards to plugin when alive |
| Android updater | `UpdateChecker.kt` fetches `release.json`, downloads APK via `DownloadManager`, opens for install |
| Safe-area padding | Notch/status bar padding on mobile (`env(safe-area-inset-*)`) |
| Cleartext traffic | `usesCleartextTraffic=true` for Matrix auth HTTP redirects (homeserver discovery, OIDC) |
| Universal APK | All 4 ABIs (arm64-v8a, armeabi-v7a, x86, x86_64) in one APK |

### Theming and appearance

| Feature | Description |
|---------|-------------|
| System theme | Auto light/dark based on OS preference |
| Custom themes | Light and dark theme selectors, multiple built-in themes |
| Monochrome mode | Single-color theme variant |
| Page zoom | Configurable 50-200% zoom |
| Twitter emoji | Twemoji option for consistent cross-platform emoji |
| Markdown toggle | Disable markdown rendering for plain text |
| 24-hour clock | `hour24Clock` setting |
| Custom date format | `D MMM YYYY`, `DD/MM/YYYY`, `MM/DD/YYYY`, `YYYY/MM/DD` |

### Branding

| Feature | Description |
|---------|-------------|
| Prinny branding | All desktop icons, favicons, and app metadata replaced with Prinny panic face |
| Build-time rename | `scripts/rename-prinny.mjs` runs via `beforeBuildCommand`: Cinny→Prinny across all source (Rust, Kotlin, TSX, JSON, XML) |
| Android package | `in.prinny.app` — directory structure, namespace, applicationId, Kotlin packages all consistent |
| About page | Settings → About: 96px Prinny logo, version from `package.json`, links to our repos |
| Release assets | All CI artifacts named `Prinny_desktop-*` / `prinny-android-*` |

### Build and CI

| Feature | Description |
|---------|-------------|
| Unified CI | Single `build.yml` triggers on push (build + upload artifacts) and `release: published` (upload to release + `release.json` generation) |
| 4 platforms | Windows (x86_64 MSI + NSIS), macOS (universal DMG), Linux (x86_64 AppImage + deb), Android (universal APK) |
| Cross-compile | Windows builds from Linux via `x86_64-pc-windows-gnu` + mingw-w64 + NSIS |
| Android CI | SDK 36, NDK 27.0.12077973, `CARGO_BUILD_JOBS=1` prevents OOM |
| Serial Rust builds | `mustRunAfter` chain in `RustPlugin.kt` prevents parallel linkers from exhausting RAM |
| Release metadata | `scripts/release.mjs` generates `release.json` on `tauri` tag for desktop + Android updater |
| Source archive | Release CI creates `prinny-client-vX.Y.Z.zip` |
| APK signed | Debug keystore generated per CI run, `apksigner` signs universal APK |

## Build

You need Rust, Node.js 16+, and platform dev tools. Clone with submodules:

```bash
git clone --recursive https://github.com/coffeegrind123/prinny-client.git
cd prinny-client
cd cinny && git checkout desktop-notifications && npm ci && cd ..
npm ci
```

### Linux

```bash
sudo apt install -y libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf
npm run tauri build
```

Output lands in `src-tauri/target/release/bundle/` (AppImage, deb, rpm).

### macOS

```bash
npm run tauri build -- --target universal-apple-darwin
```

Output: `src-tauri/target/universal-apple-darwin/release/bundle/` (DMG, app bundle).

### Windows

```bash
# Cross-compile from Linux:
rustup target add x86_64-pc-windows-gnu
sudo apt install -y mingw-w64 nsis
npm run tauri build -- --target x86_64-pc-windows-gnu

# Or on Windows directly:
npm run tauri build
```

Output: `src-tauri/target/release/bundle/` (MSI, NSIS installer).

### Android

Needs SDK 36, NDK 27.0.12077973, and four Rust targets:

```bash
rustup target add aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android

export ANDROID_HOME=/path/to/sdk
export NDK_HOME=$ANDROID_HOME/ndk/27.0.12077973

npx tauri android build
```

Signing:

```bash
keytool -genkeypair -v -keystore debug.keystore -alias androiddebugkey \
  -keyalg RSA -keysize 2048 -validity 10000 \
  -dname "CN=Android Debug,O=Android,C=US" \
  -storepass android -keypass android

$ANDROID_HOME/build-tools/35.0.0/apksigner sign \
  --ks debug.keystore --ks-pass pass:android \
  --ks-key-alias androiddebugkey --key-pass pass:android \
  --out app-release-signed.apk \
  src-tauri/gen/android/app/build/outputs/apk/universal/release/app-universal-release-unsigned.apk
```

## Dev

```bash
npm run tauri dev
```

Runs the Vite dev server and launches a Tauri window. For Android use `npx tauri android dev`.

## Contributing

Personal fork. Issues and PRs welcome, no formal process. Open an issue before writing code.

## License

AGPL-3.0-only, same as upstream.
