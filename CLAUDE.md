# prinny-client

Cinny Matrix client packaged as a desktop app via Tauri v2. Cross-compiles to Windows from Linux using the GNU toolchain.

**Repos:**
- Desktop shell: `coffeegrind123/prinny-client` (this repo)
- Frontend (submodule): `coffeegrind123/cinny` branch `desktop-notifications`

## Cutting a release

CI runs on every push to `main` but only uploads build artifacts. To publish a release:

1. Make sure the latest CI run on `main` is green (all 4 platforms passing).
2. Tag and push:
   ```bash
   git tag v4.11.X && git push origin v4.11.X
   ```
3. Create the release (triggers the full pipeline including release asset uploads and `release.json`):
   ```bash
   gh release create v4.11.X \
     --title "Prinny Client v4.11.X" \
     --notes "Release notes here"
   ```
4. The `release: published` trigger fires the full `Build` workflow:
   - All 4 platforms build and upload assets to the release
   - `archive` job uploads a source zip
   - `release-update` runs `scripts/release.mjs` to generate `release.json` on the `tauri` tag — this powers the in-app updater

After the release CI completes, the `release.json` on the `tauri` tag will include the new version's platform entries, and desktop/Android clients will auto-detect the update.

**If the release CI fails**, check the failed job logs, fix the issue, delete the tag/release, and re-tag:
```bash
gh release delete v4.11.X --yes
git push origin --delete v4.11.X
git tag -d v4.11.X
# fix, commit, push, then re-tag
```

## Fresh clone & build

```bash
git clone --recursive https://github.com/coffeegrind123/prinny-client.git
cd prinny-client

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
- `bundle/nsis/Prinny_x.y.z_x64-setup.exe` — NSIS installer (18MB)
- `bundle/nsis/Prinny_x.y.z_x64-setup.nsis.zip` — updater archive

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
| JDK 21 | `/opt/jdk-21.0.10+7/` | keytool for signing |
| apksigner | `/opt/android-sdk/build-tools/35.0.0/apksigner` | APK signing |

### Environment variables

**All required** — the build fails without each one:

```bash
export ANDROID_HOME=/opt/android-sdk
export ANDROID_SDK_ROOT=/opt/android-sdk
export NDK_HOME=/opt/android-sdk/ndk/27.0.12077973
```

Also run `source ~/.cargo/env` before building (shell state not preserved between Bash calls).

### Build memory requirements (CRITICAL)

This build runs on a **21GB RAM machine with 6GB swap**. The `src-tauri/target/` directory accumulates 15+ GB of stale artifacts. When swap is full, the combined Rust + Gradle build triggers OOM (exit 137).

**The three rules that prevent OOM:**

1. **Always `cargo clean` before every build** — frees 7-16GB of stale artifacts.
2. **Always `CARGO_BUILD_JOBS=1`** — each cargo instance is single-threaded, so compilation uses ~1 core instead of all 12.
3. **Serialized `rustBuild*` tasks via `mustRunAfter` in `RustPlugin.kt`** — this is the critical fix. By default, Gradle runs all 4 `rustBuild*` tasks (one per ABI) in parallel. Even with `CARGO_BUILD_JOBS=1`, parallel cargo instances reach the linking step at overlapping times, and each linker consumes several GB of RAM. The `mustRunAfter` chain forces them to run one at a time (aarch64 → armv7 → i686 → x86_64), so only one linker is active at any moment. Total Rust time with serial builds: ~9 minutes (vs OOM in parallel). See `src-tauri/gen/android/buildSrc/.../RustPlugin.kt:63-69`.

**Note on `abiList`/`targetList` in `gradle.properties`:** These were found to be **ignored** in practice — the build compiled all 4 ABIs regardless of the property values. The serialization fix in `RustPlugin.kt` is what actually controls build behavior. The properties are left in `gradle.properties` as documentation of intent.

**Killing stale build processes (if OOM or interruption leaves orphans):**

The Gradle RustPlugin spawns `cargo`, `rustc`, and `cc` child processes that survive the parent being killed. An OOM'd or interrupted build can leave 4+ cargo instances running in the background, each consuming CPU and RAM. Check and kill them before retrying:

```bash
# Check for stale build processes
ps aux | grep -E "cargo.*cinny|rustc.*cinny|tauri android" | grep -v grep

# Kill all stale build processes
pkill -f "cargo build.*cinny" 2>/dev/null
pkill -f "tauri android build" 2>/dev/null
# If processes persist (D state = stuck in kernel), force kill:
kill -9 $(pgrep -f "cargo.*cinny") 2>/dev/null
```

Also check `free -h` before building — if swap is full (>5GB used), something else is leaking. Common culprits: Chrome (browser MCP, ~400MB), Ghidra (JVM with `-Xmx4g`), old Gradle daemons. The Bun process for openclaude itself uses ~800MB RSS.

**Before every build:**

```bash
# Kill any stale processes from previous failed builds first
pkill -f "cargo build.*cinny" 2>/dev/null

cd /opt/openclaude-src/cinny-desktop/src-tauri
cargo clean                          # frees 7-16GB
rm -rf gen/android/app/build         # clean Gradle build output
rm -rf gen/android/buildSrc/build    # clean Gradle buildSrc (needed after RustPlugin.kt changes)
```

The Gradle JVM heap is capped at 2GB (`-Xmx2048m` in `gradle.properties`). This is intentional — keep the constraint.

### End-to-end build flow

```bash
# 1. Clean (frees 15+ GB — essential for OOM avoidance)
source ~/.cargo/env
cd /opt/openclaude-src/cinny-desktop/src-tauri
cargo clean
rm -rf gen/android/app/build

# 2. Build release APK (CARGO_BUILD_JOBS=1 is CRITICAL — prevents OOM)
cd /opt/openclaude-src/cinny-desktop
CARGO_BUILD_JOBS=1 \
ANDROID_HOME=/opt/android-sdk \
ANDROID_SDK_ROOT=/opt/android-sdk \
NDK_HOME=/opt/android-sdk/ndk/27.0.12077973 \
npx tauri android build

# 3. Sign with debug keystore
/opt/android-sdk/build-tools/35.0.0/apksigner sign \
  --ks debug.keystore --ks-pass pass:android \
  --ks-key-alias androiddebugkey --key-pass pass:android \
  --out app-release-signed.apk \
  src-tauri/gen/android/app/build/outputs/apk/universal/release/app-universal-release-unsigned.apk

# 4. Verify
/opt/android-sdk/build-tools/35.0.0/apksigner verify app-release-signed.apk
```

Output: `app-release-signed.apk` (119MB, universal — all 4 ABIs, signed, installable).

### Debug keystore (one-time setup)

```bash
/opt/jdk-21.0.10+7/bin/keytool -genkeypair -v \
  -keystore debug.keystore -alias androiddebugkey \
  -keyalg RSA -keysize 2048 -validity 10000 \
  -dname "CN=Android Debug,O=Android,C=US" \
  -storepass android -keypass android
```

The keystore is at the repo root (`debug.keystore`) and is `.gitignore`d — never commit it.

### How the build works (Tauri v2 Android internals)

1. `npx tauri android build` starts a TCP server on localhost
2. `beforeBuildCommand` fires: `cd cinny && npm run build` (Vite → `cinny/dist/`)
3. Gradle's `rustBuild*` tasks connect back to the Tauri CLI via TCP and invoke `cargo build --target <target> --release` once per ABI
4. Each target's `libapp_lib.so` is symlinked into `jniLibs/<abi>/`
5. Gradle packages the APK (or AAB — also produced)

The build compiles 4 ABIs by default (arm64-v8a, armeabi-v7a, x86, x86_64) producing a "universal" APK. The `gradle.properties` `abiList` and `targetList` were found to be **ignored** — the Tauri CLI always passes all 4 targets regardless. To control which ABIs are built, modify the hardcoded lists in `RustPlugin.kt` directly.

**Serial build ordering:** `RustPlugin.kt` uses `mustRunAfter` constraints to serialize the 4 `rustBuild*` tasks. Without this, Gradle runs them in parallel and multiple linkers exhaust 21GB RAM → OOM. Each task waits for the previous one to finish before starting.

### APK output

| Variant | Path | Signed? |
|---------|------|---------|
| Release (universal) | `src-tauri/gen/android/app/build/outputs/apk/universal/release/app-universal-release-unsigned.apk` | No |
| Release AAB | `src-tauri/gen/android/app/build/outputs/bundle/universalRelease/app-universal-release.aab` | No |
| After signing | `app-release-signed.apk` (repo root) | Yes (debug key) |

### Matrix cleartext fix

Android 9+ blocks HTTP (cleartext) traffic in WebViews by default. Matrix auth flows (homeserver discovery, OIDC redirects) hit HTTP even when the final endpoint is HTTPS. The fix is in `src-tauri/gen/android/app/build.gradle.kts`:

```kotlin
// defaultConfig — applies to both debug and release
manifestPlaceholders["usesCleartextTraffic"] = "true"
```

This was previously `"false"` for release builds, causing `ERR_CLEARTEXT_NOT_PERMITTED` in the WebView.

### Foreground service (GrapheneOS background notifications)

On de-Googled devices without FCM, Android kills background apps aggressively. A foreground service with a persistent notification keeps the process alive so the Matrix WebSocket stays connected.

**Architecture:**

```
JS: startForegroundService()
  → plugin:foreground|start_foreground
  → ForegroundServicePlugin.kt (Tauri plugin)
  → ForegroundService.kt (Android Service with startForeground())
  → Persistent notification "Cinny - Connected to Matrix"
```

**Startup:** The foreground service starts automatically in `useUnifiedPush.ts` when the Matrix client connects. On cleanup (client stop), it stops the service.

**Permissions required:**
- `FOREGROUND_SERVICE` — to start a foreground service
- `FOREGROUND_SERVICE_DATA_SYNC` — Android 14+ foreground service type
- `POST_NOTIFICATIONS` — to show the persistent notification (Android 13+)

**Files:**
| File | Role |
|------|------|
| `ForegroundService.kt` | Android Service, creates notification channel, calls `startForeground()` |
| `ForegroundServicePlugin.kt` | Tauri plugin with `startForeground`/`stopForeground`/`isForegroundRunning` commands |
| `ForegroundService.kt` notification channel | `cinny_foreground`, IMPORTANCE_LOW, no badge |

**To disable:** Remove `await startForegroundService()` from `useUnifiedPush.ts:67`. Users with FCM don't need it.

### Key files (Android)

| File | Role |
|------|------|
| `src-tauri/src/lib.rs` | `#[cfg_attr(mobile, tauri::mobile_entry_point)]`, conditional plugin loading |
| `src-tauri/Cargo.toml` | `tauri-plugin-mobile-push` + mobile-gated deps |
| `src-tauri/capabilities/mobile.json` | Mobile capability permissions |
| `src-tauri/capabilities/desktop.json` | Desktop-only perms (global-shortcut moved here) |
| `src-tauri/gen/android/app/build.gradle.kts` | `usesCleartextTraffic`, minSdk/targetSdk, build types |
| `src-tauri/gen/android/gradle.properties` | JVM heap, ABI list |
| `src-tauri/gen/android/tauri.settings.gradle` | Rust target list |
| `src-tauri/gen/android/buildSrc/.../RustPlugin.kt` | Gradle→cargo bridge, `mustRunAfter` serialization |
| `src-tauri/gen/android/app/src/main/java/.../UnifiedPushPlugin.kt` | UnifiedPush Tauri plugin (Android) |
| `src-tauri/gen/android/app/src/main/java/.../UnifiedPushReceiver.kt` | UP MessagingReceiver (Android) |
| `src-tauri/gen/android/app/src/main/java/.../ForegroundService.kt` | Foreground service for background WebSocket (GrapheneOS) |
| `src-tauri/gen/android/app/src/main/java/.../ForegroundServicePlugin.kt` | Foreground service Tauri plugin (start/stop from JS) |
| `src-tauri/gen/android/app/src/main/AndroidManifest.xml` | Permissions, service/receiver declarations |
| `src-tauri/gen/android/settings.gradle` | JitPack repo for UP connector |
| `src-tauri/tauri.conf.json` | `beforeBuildCommand`, `frontendDist`, bundle identifier |
| `cinny/src/app/hooks/useUnifiedPush.ts` | UP registration + Matrix pusher hook |
| `cinny/src/app/utils/mobile-push.ts` | UP Tauri command wrappers |
| `cinny/src/index.css` | Safe-area padding for device notches |
| `.gitignore` | Excludes `*.apk`, `*.idsig`, `debug.keystore` |

### Iteration (edit → test on device)

1. Edit source in `cinny/src/` or `src-tauri/`
2. Run the end-to-end build flow above (with `CARGO_BUILD_JOBS=1`)
3. Transfer `app-release-signed.apk` to Android device
4. Sideload and launch

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
