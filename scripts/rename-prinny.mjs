/**
 * Pre-build: rename Cinny → Prinny across the entire project.
 *
 * Runs as beforeBuildCommand. Modifies source in-place so the Tauri build
 * compiles with "Prinny" branding. Also renames Android package directory.
 *
 * Revert:  git checkout -- . && cd cinny && git checkout -- .
 */
import { readFileSync, writeFileSync, readdirSync, existsSync, renameSync, unlinkSync } from 'node:fs';
import { join, extname, relative, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..');
const SCRIPT_PATH = fileURLToPath(import.meta.url);

const TEXT_EXTS = new Set([
  '.rs', '.toml', '.json', '.json5',
  '.ts', '.tsx', '.js', '.jsx', '.mjs',
  '.html', '.css', '.svg', '.xml',
  '.kt', '.kts', '.gradle', '.properties',
  '.md',
]);

const SKIP_DIRS = new Set([
  'node_modules', 'target', '.git', 'dist',
  'build',    // Android build intermediates
  'icons',    // icon files
  'scripts',  // don't self-rename
]);

const SKIP_FILES = new Set([
  'accountData.ts', // Matrix protocol constant
]);

// ── Phase 0: rename Android package directory ──────────────────────

function renameAndroidPackageDir() {
  const javaRoot = join(ROOT, 'src-tauri', 'gen', 'android', 'app', 'src', 'main', 'java', 'in');
  const oldDir = join(javaRoot, 'cinny');
  const newDir = join(javaRoot, 'prinny');
  if (existsSync(oldDir) && !existsSync(newDir)) {
    renameSync(oldDir, newDir);
    console.log(`  [dir] in/cinny → in/prinny`);
  }
  // Remove stale assets tauri.conf.json so it's regenerated fresh
  const staleAssets = join(ROOT, 'src-tauri', 'gen', 'android', 'app', 'src', 'main', 'assets', 'tauri.conf.json');
  if (existsSync(staleAssets)) {
    unlinkSync(staleAssets);
    console.log(`  [del] stale assets/tauri.conf.json`);
  }
}

// ── Phase 1: file content replacements ─────────────────────────────

function replaceInFile(filePath) {
  if (filePath === SCRIPT_PATH) return false;
  try {
    let content = readFileSync(filePath, 'utf8');
    let original = content;

    // Repo URLs
    content = content.replace(
      /coffeegrind123\/cinny-desktop/g,
      'coffeegrind123/prinny-client'
    );

    // Android package identifier stays as in.cinny.app in config files
    // because Tauri CLI reads it before beforeBuildCommand runs.
    // Only user-facing strings (app_name, notifications) are renamed.
    // NOTE: do NOT rename identifier in tauri.conf.json or
    // namespace/applicationId in build.gradle.kts.

    // Android notification channel IDs
    content = content.replace(/\bcinny_foreground\b/g, 'prinny_foreground');
    content = content.replace(/\bcinny_messages\b/g, 'prinny_messages');

    // Android theme
    content = content.replace(/Theme\.cinny\b/g, 'Theme.prinny');

    // Kotlin APK filename
    content = content.replace(/"cinny-v\$/g, '"prinny-v$');

    // Tauri config
    content = content.replace(/"productName"\s*:\s*"Cinny"/g, '"productName": "Prinny"');

    // General case-sensitive — title case and all caps only.
    // Lowercase "cinny" is NOT blanket-replaced because it appears
    // in directory paths (cd cinny, ../cinny/dist).
    content = content.replace(/\bCinny\b/g, 'Prinny');
    content = content.replace(/\bCINNY\b/g, 'PRINNY');

    if (content !== original) {
      writeFileSync(filePath, content, 'utf8');
      return true;
    }
    return false;
  } catch {
    return false;
  }
}

// ── Walk ────────────────────────────────────────────────────────────

function walk(dir) {
  let count = 0;
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    if (entry.name.startsWith('.') && entry.name !== '.gitmodules') continue;
    if (SKIP_DIRS.has(entry.name)) continue;

    const path = join(dir, entry.name);

    if (entry.isDirectory()) {
      count += walk(path);
    } else if (entry.isFile()) {
      if (SKIP_FILES.has(entry.name)) continue;
      if (TEXT_EXTS.has(extname(entry.name).toLowerCase())) {
        if (replaceInFile(path)) {
          console.log(`  ${relative(ROOT, path)}`);
          count++;
        }
      }
    }
  }
  return count;
}

// ── Main ────────────────────────────────────────────────────────────

console.log('[prinny] Renaming Cinny → Prinny...');
renameAndroidPackageDir();
const changed = walk(ROOT);
console.log(`[prinny] Done — ${changed} files changed.`);
