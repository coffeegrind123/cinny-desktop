import fetch from "node-fetch";
import { getOctokit, context } from "@actions/github";

async function getAssetSign(url) {
  const response = await fetch(url, {
    method: "GET",
    headers: {
      "Content-Type": "application/octet-stream",
    },
  });

  return response.text();
}

async function createTauriRelease() {
  if (process.env.GITHUB_TOKEN === undefined) {
    throw new Error("GITHUB_TOKEN is not found!");
  }

  const github = getOctokit(process.env.GITHUB_TOKEN);
  const { repos } = github.rest;
  const repoMetaData = {
    owner: context.repo.owner,
    repo: context.repo.repo,
  };

  const tagsResult = await repos.listTags({ ...repoMetaData, per_page: 10, page: 1 });
  const latestTag = tagsResult.data.find((tag) => tag.name.startsWith("v"));
  console.log(latestTag);

  const latestRelease = await repos.getReleaseByTag({ ...repoMetaData, tag: latestTag.name });
  const latestAssets = latestRelease.data.assets;

  const windowsX86_64 = {};
  const linuxX86_64 = {};
  const darwinX86_64 = {};
  const darwinAarch64 = {};
  const android = {};

  const promises = latestAssets.map(async (asset) => {
    const { name, browser_download_url } = asset;

    // Windows: .msi/.exe installer (non-updater)
    if ((/\.msi$/.test(name) && !/\.msi\.zip/.test(name)) ||
        (/\.exe$/.test(name) && !/\.nsis\.zip/.test(name))) {
      windowsX86_64.url = browser_download_url;
    }

    // Linux: .AppImage (non-updater)
    if (/\.AppImage$/.test(name) && !/\.AppImage\.tar\.gz/.test(name)) {
      linuxX86_64.url = browser_download_url;
    }

    // macOS: .dmg
    if (/universal\.dmg$/.test(name)) {
      darwinX86_64.url = browser_download_url;
      darwinAarch64.url = browser_download_url;
    }

    // Updater artifacts (signed — only if TAURI_SIGNING_PRIVATE_KEY is set)
    if (/\.msi\.zip$/.test(name) || /\.nsis\.zip$/.test(name)) {
      windowsX86_64.url = browser_download_url;
    }
    if (/\.msi\.zip\.sig$/.test(name) || /\.nsis\.zip\.sig$/.test(name)) {
      windowsX86_64.signature = await getAssetSign(browser_download_url);
    }

    if (/\.AppImage\.tar\.gz$/.test(name)) {
      linuxX86_64.url = browser_download_url;
    }
    if (/\.AppImage\.tar\.gz\.sig$/.test(name)) {
      linuxX86_64.signature = await getAssetSign(browser_download_url);
    }

    if (/universal\.app\.tar\.gz$/.test(name)) {
      darwinX86_64.url = browser_download_url;
      darwinAarch64.url = browser_download_url;
    }
    if (/universal\.app\.tar\.gz\.sig$/.test(name)) {
      darwinX86_64.signature = await getAssetSign(browser_download_url);
      darwinAarch64.signature = await getAssetSign(browser_download_url);
    }

    // Android: universal APK
    if (/prinny-android-universal\.apk$/.test(name)) {
      android.url = browser_download_url;
      android.version = latestTag.name;
    }
    if (/prinny-android-universal\.apk\.sha256$/.test(name)) {
      const sha256Text = await getAssetSign(browser_download_url);
      android.sha256 = sha256Text.split(/\s+/)[0];
    }
  });

  await Promise.allSettled(promises);

  const releaseData = {
    name: latestTag.name,
    notes: `https://github.com/${repoMetaData.owner}/${repoMetaData.repo}/releases/tag/${latestTag.name}`,
    pub_date: new Date().toISOString(),
    platforms: {},
  };

  const setPlatform = (key, obj) => {
    if (obj.url) {
      releaseData.platforms[key] = { signature: '', ...obj };
    } else {
      console.log(`No ${key} updater artifact (signing key not configured)`);
    }
  };
  setPlatform('windows-x86_64', windowsX86_64);
  setPlatform('linux-x86_64', linuxX86_64);
  setPlatform('darwin-x86_64', darwinX86_64);
  setPlatform('darwin-aarch64', darwinAarch64);
  setPlatform('android', android);

  // Get or create the "tauri" release used as updater metadata storage
  let tauriRelease;
  try {
    const result = await repos.getReleaseByTag({ ...repoMetaData, tag: 'tauri' });
    tauriRelease = result.data;
  } catch (err) {
    if (err.status === 404) {
      console.log('Creating tauri release for updater metadata...');
      tauriRelease = await repos.createRelease({
        ...repoMetaData,
        tag_name: 'tauri',
        name: 'Updater Metadata',
        body: 'Auto-generated release for Tauri updater metadata. Do not delete.',
        draft: false,
        prerelease: false,
      });
      tauriRelease = tauriRelease.data;
    } else {
      throw err;
    }
  }

  const prevReleaseAsset = tauriRelease.assets.find((asset) => asset.name === 'release.json');
  if (prevReleaseAsset) {
    await repos.deleteReleaseAsset({ ...repoMetaData, asset_id: prevReleaseAsset.id });
  }

  console.log(releaseData);
  await repos.uploadReleaseAsset({
    ...repoMetaData,
    release_id: tauriRelease.id,
    name: 'release.json',
    data: JSON.stringify(releaseData, null, 2),
  });
}
createTauriRelease();
