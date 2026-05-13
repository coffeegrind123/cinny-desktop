# Prinny Client <img align="right" width="90" height="128" src="src-tauri/icons/prinny-logo.png" alt="Prinny" />

[![Build](https://github.com/coffeegrind123/prinny-client/actions/workflows/build.yml/badge.svg)](https://github.com/coffeegrind123/prinny-client/actions/workflows/build.yml)
[![Latest](https://img.shields.io/github/v/release/coffeegrind123/prinny-client?label=latest)](https://github.com/coffeegrind123/prinny-client/releases/latest)
[![Downloads](https://img.shields.io/github/downloads/coffeegrind123/prinny-client/total?label=downloads)](https://github.com/coffeegrind123/prinny-client/releases)

A Matrix chat client that actually feels native. Prinny Client is [Cinny](https://cinny.in) packaged with [Tauri v2](https://v2.tauri.app), shipping on Windows, macOS, Linux, and Android. It does desktop notifications with proper click-to-open. It flashes the taskbar. It puts unread counts on your dock icon. On mobile there's swipe navigation like Discord. On Android it keeps the Matrix connection alive in the background so you don't miss messages on de-Googled phones.

Hard fork of [cinnyapp/cinny-desktop](https://github.com/cinnyapp/cinny-desktop). The frontend lives at [coffeegrind123/cinny](https://github.com/coffeegrind123/cinny) on the `desktop-notifications` branch.

## Download

Everything is on [GitHub Releases](https://github.com/coffeegrind123/prinny-client/releases).

| Platform | Download |
|----------|----------|
| Windows | [MSI](https://github.com/coffeegrind123/prinny-client/releases/latest/download/Cinny_desktop-x86_64.msi) · [NSIS setup](https://github.com/coffeegrind123/prinny-client/releases/latest/download/Cinny_desktop-x86_64-setup.exe) |
| macOS | [DMG](https://github.com/coffeegrind123/prinny-client/releases/latest/download/Cinny_desktop-universal.dmg) |
| Linux | [AppImage](https://github.com/coffeegrind123/prinny-client/releases/latest/download/Cinny_desktop-x86_64.AppImage) · [deb](https://github.com/coffeegrind123/prinny-client/releases/latest/download/Cinny_desktop-x86_64.deb) |
| Android | [APK](https://github.com/coffeegrind123/prinny-client/releases/latest/download/cinny-android-universal.apk) (sideload) |

Desktop builds update themselves. Android checks on launch and downloads new APKs when available.

## What it does

Notifications work the way you'd expect. Sender avatar in the toast, click to jump to the room, taskbar flashes on Windows. E2EE rooms wait for decryption before notifying so you don't see ciphertext.

The macOS Dock and Linux taskbar show an unread count badge. Red circle, number in it, clears when you're caught up.

On mobile, swipe from the right edge to open a room, swipe from the left edge to go back. Same gesture Discord uses. Works on the Home, Direct, and Space screens.

DM avatars and the member list show online/busy/away dots. When someone replies to your message, the replied-to message gets a yellow left border. The "New Messages" divider looks like Discord's (green line, "NEW" badge on the right). Timestamps sit next to usernames instead of floating on the far right.

The Android build keeps a foreground service running so the Matrix WebSocket stays connected. This matters on GrapheneOS and other devices without FCM. The persistent notification says "Connected to Matrix" and sits at low priority so it's not annoying. UnifiedPush support means you get pings even when the app is fully killed, assuming you have a distributor installed.

MatrixRocks community servers are built into the room directory. The updater checks `release.json` on desktop and uses DownloadManager on Android.

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
