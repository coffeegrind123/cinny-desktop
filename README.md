# Prinny Client <img align="right" width="90" height="128" src="src-tauri/icons/prinny-logo.png" alt="Prinny" />

[![Build Status](https://github.com/coffeegrind123/cinny-desktop/actions/workflows/build.yml/badge.svg)](https://github.com/coffeegrind123/cinny-desktop/actions/workflows/build.yml)
[![Latest Release](https://img.shields.io/github/v/release/coffeegrind123/cinny-desktop?label=latest)](https://github.com/coffeegrind123/cinny-desktop/releases/latest)
[![Download](https://img.shields.io/github/downloads/coffeegrind123/cinny-desktop/total?label=downloads)](https://github.com/coffeegrind123/cinny-desktop/releases)

Prinny Client is a Matrix chat client packaged as a desktop and mobile app via [Tauri v2](https://v2.tauri.app). It wraps the [Cinny](https://cinny.in) web client with native integrations: desktop notifications, taskbar badges, mobile swipe gestures, background WebSocket on Android, and an in-app updater.

It is a hard fork of [cinnyapp/cinny-desktop](https://github.com/cinnyapp/cinny-desktop) with the frontend forked from [coffeegrind123/cinny](https://github.com/coffeegrind123/cinny) (`desktop-notifications` branch).

## Download

Installers for all platforms are published on [GitHub Releases](https://github.com/coffeegrind123/cinny-desktop/releases).

| Platform | Download |
|----------|----------|
| Windows | [MSI installer](https://github.com/coffeegrind123/cinny-desktop/releases/latest/download/Cinny_desktop-x86_64.msi) · [NSIS setup](https://github.com/coffeegrind123/cinny-desktop/releases/latest/download/Cinny_desktop-x86_64-setup.exe) |
| macOS | [DMG](https://github.com/coffeegrind123/cinny-desktop/releases/latest/download/Cinny_desktop-universal.dmg) |
| Linux | [AppImage](https://github.com/coffeegrind123/cinny-desktop/releases/latest/download/Cinny_desktop-x86_64.AppImage) · [deb](https://github.com/coffeegrind123/cinny-desktop/releases/latest/download/Cinny_desktop-x86_64.deb) |
| Android | [APK](https://github.com/coffeegrind123/cinny-desktop/releases/latest/download/cinny-android-universal.apk) (sideload) |

The desktop app auto-updates via the built-in updater. Android checks for updates on launch and downloads new APKs automatically.

## Features

* **Desktop notifications** — Native OS toasts with sender avatar, click-to-open, and taskbar flash on Windows.
* **Taskbar unread badge** — Red badge count on the macOS Dock and Linux taskbar.
* **Mobile swipe gestures** — Discord-style edge swipes: left-edge back from room, right-edge open room from list.
* **Presence indicators** — Online/busy/away dots on DM avatars and in the member list.
* **Reply highlights** — Yellow left-border on messages someone replies to.
* **Android foreground service** — Keeps the Matrix WebSocket alive in the background on de-Googled devices (GrapheneOS).
* **UnifiedPush** — Push notifications via UnifiedPush distributors when the app is killed.
* **In-app updater** — Desktop: checks for updates via `release.json`. Android: native background update checker with DownloadManager.
* **MatrixRocks** — Built-in featured community server directory with room info.
* **E2EE-aware notifications** — Waits for decryption before sending notification content.

## Build

### Prerequisites

| Tool | Purpose |
|------|---------|
| [Rust](https://rustup.rs) | Tauri backend |
| [Node.js](https://nodejs.org) ≥ 16 | Frontend build |
| OS dev tools | Platform-specific (see below) |

Clone with submodules:

```bash
git clone --recursive https://github.com/coffeegrind123/cinny-desktop.git
cd cinny-desktop
cd cinny && git checkout desktop-notifications && npm ci && cd ..
npm ci
```

### Linux

```bash
# Install system dependencies
sudo apt install -y libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf

# Build
npm run tauri build
```

Output: `src-tauri/target/release/bundle/` — AppImage, deb, rpm.

### macOS

```bash
npm run tauri build -- --target universal-apple-darwin
```

Output: `src-tauri/target/universal-apple-darwin/release/bundle/` — DMG, app bundle.

### Windows

```bash
# From Linux (cross-compile):
rustup target add x86_64-pc-windows-gnu
sudo apt install -y mingw-w64 nsis
npm run tauri build -- --target x86_64-pc-windows-gnu

# From Windows:
npm run tauri build
```

Output: `src-tauri/target/release/bundle/` — MSI, NSIS installer.

### Android

Requires Android SDK 36, NDK 27.0.12077973, and Rust Android targets:

```bash
rustup target add aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android

export ANDROID_HOME=/path/to/sdk
export NDK_HOME=$ANDROID_HOME/ndk/27.0.12077973

npx tauri android build
```

Sign the APK:

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

This starts the Vite dev server for the frontend and launches the Tauri window. On Android, use `npx tauri android dev`.

## Contributing

This is a personal fork. Issues and PRs are welcome but there's no contribution guide — just open an issue first to discuss what you want to change.

## License

AGPL-3.0-only — same as upstream Cinny.
