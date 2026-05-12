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

## Android build

### Prerequisites

| Tool | Path / Install | Purpose |
|------|---------------|---------|
| Android SDK | `/opt/android-sdk/` | Platform tools, build tools, platform APIs |
| Android NDK | `/opt/android-sdk/ndk/27.0.12077973/` | Native (Rust) code compiler for Android |
| Rust Android targets | `rustup target add aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android` | Cross-compile Rust to Android ABIs |
| Tauri CLI | `@tauri-apps/cli@2.7.1` (npm root dep) | `npx tauri android build` entry point |
| JDK | System (Gradle wrapper) | Gradle build |

### Environment variables

**All required** — the build fails without each one:

```bash
export ANDROID_HOME=/opt/android-sdk
export ANDROID_SDK_ROOT=/opt/android-sdk
export NDK_HOME=/opt/android-sdk/ndk/27.0.12077973
```

Also run `source ~/.cargo/env` before building (shell state not preserved between Bash calls).

### Resource constraints (CRITICAL)

This build runs on a **21GB RAM machine with only 6GB swap**, and the swap is often nearly full. Running all Rust targets + Gradle simultaneously triggers OOM (exit 137).

**Constraints in place:**

| What | Setting | Why |
|------|---------|-----|
| Gradle JVM heap | `-Xmx2048m` (in `gradle.properties`) | Keep Gradle daemon memory bounded |
| Cargo build jobs | `CARGO_BUILD_JOBS=1` (or `2`) | One Rust compilation at a time — cross-compile to Android is heavy |
| Android ABIs | `abiList=arm64-v8a,armeabi-v7a` (in `gradle.properties`) | Only ARM targets, skip x86/x86_64 |
| Rust targets | `targetList=aarch64,armv7` (in `tauri.settings.gradle`) | Matches the ABI list above |

**The OOM problem:** `npx tauri android build` fires everything at once — `beforeBuildCommand` (Vite) + `cargo build` for all Android targets (via Gradle RustPlugin) + Gradle APK packaging. With ~8GB free RAM and the Rust cross-compiler consuming multiple GB per target, the combined peak exceeds available memory and the OOM killer terminates the process.

**Staged build workaround (attempted but still OOM'd):** Build one Rust target at a time via `cargo build --target <target>` directly, then run Gradle separately for packaging. Still tight on memory.

### Build commands

```bash
# Release APK (unsigned — won't install without signing)
source ~/.cargo/env && cd /opt/openclaude-src/cinny-desktop && \
  ANDROID_HOME=/opt/android-sdk \
  ANDROID_SDK_ROOT=/opt/android-sdk \
  NDK_HOME=/opt/android-sdk/ndk/27.0.12077973 \
  npx tauri android build

# Debug APK (auto-signed — installable for testing)
source ~/.cargo/env && cd /opt/openclaude-src/cinny-desktop && \
  ANDROID_HOME=/opt/android-sdk \
  ANDROID_SDK_ROOT=/opt/android-sdk \
  NDK_HOME=/opt/android-sdk/ndk/27.0.12077973 \
  npx tauri android build --debug
```

### APK output

| Variant | Path | Signed? |
|---------|------|---------|
| Release (universal) | `src-tauri/gen/android/app/build/outputs/apk/universal/release/app-universal-release-unsigned.apk` | No — must sign before install |
| Debug | `src-tauri/gen/android/app/build/outputs/apk/debug/app-debug.apk` | Yes (debug keystore) |

### Signing a release APK for testing

When the debug build won't complete (OOM during Gradle + Rust simultaneously) but the release APK already exists, sign the release APK with a debug keystore:

```bash
# 1. Generate a one-time debug keystore (skip if ~/.android/debug.keystore already exists)
keytool -genkeypair -v -keystore debug.keystore \
  -alias androiddebugkey -keyalg RSA -keysize 2048 -validity 10000 \
  -dname "CN=Android Debug,O=Android,C=US" \
  -storepass android -keypass android

# 2. Sign the unsigned release APK
apksigner sign --ks debug.keystore --ks-pass pass:android \
  --ks-key-alias androiddebugkey --key-pass pass:android \
  --out app-debug-signed.apk \
  src-tauri/gen/android/app/build/outputs/apk/universal/release/app-universal-release-unsigned.apk

# 3. Verify signature
apksigner verify app-debug-signed.apk
```

A debug-signed APK installs and runs fine for testing. The only difference from a true debug build is `android:debuggable` won't be set in the manifest (so USB debugging / logcat won't attach), but the app runs normally.

Tool paths on this machine:
- `keytool`: `/opt/jdk-21.0.10+7/bin/keytool`
- `apksigner`: `/opt/android-sdk/build-tools/35.0.0/apksigner`

### Current state (2026-05-12)

Android support is **uncommitted** — all changes are staged in the working tree:

| File | Status | What changed |
|------|--------|-------------|
| `src-tauri/Cargo.toml` | Modified | Added `tauri-plugin-mobile-push = "0.1"` |
| `src-tauri/Cargo.lock` | Modified | Updated for new deps |
| `src-tauri/src/lib.rs` | Modified | Added `#[cfg_attr(mobile, tauri::mobile_entry_point)]`, made plugins conditional on `#[cfg(not(mobile))]`, added mobile-only plugins |
| `src-tauri/capabilities/desktop.json` | Modified | Moved `global-shortcut` perms here from migrated |
| `src-tauri/capabilities/migrated.json` | Modified | Removed `global-shortcut` perms (desktop-only) |
| `src-tauri/capabilities/mobile.json` | **New** | Mobile capability permissions (iOS + Android) |
| `src-tauri/gen/android/` | **New** | 205 generated files (Kotlin, Gradle, XML, resources) |
| `src-tauri/gen/schemas/android-schema.json` | **New** | Android-specific schema |
| `src-tauri/gen/schemas/mobile-schema.json` | **New** | Mobile-specific schema |
| `cinny` (submodule) | Modified | Untracked changes inside submodule |

The release APK was built successfully at 119MB (universal, all 4 ABIs), but **won't install** because it's unsigned. The debug build (auto-signed) is the correct target for testing.

### Known issues

1. **Unsigned release APK won't install.** Android requires all APKs to be signed. Debug builds are auto-signed with a debug keystore. For release: sign with `apksigner` or `jarsigner` using a proper keystore.

2. **OOM during build.** The combined Rust + Gradle memory peak exceeds available RAM. Build with `CARGO_BUILD_JOBS=1` (or staged: one target at a time).

3. **`tauri-plugin-mobile-push = "0.1"` in Cargo.toml** — this crate may not exist on crates.io. Verify it's a valid dependency.

4. **The `--debug` flag for `npx tauri android build`** produces an `app-debug.apk` with debug signing. This is the go-to for device testing.

### Iteration (edit → test on device)

1. Edit source in `cinny/src/` or `src-tauri/`
2. Run the debug build command (see above)
3. Copy `app-debug.apk` to Android device
4. Install and launch

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
