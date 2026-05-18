package `in`.prinny.app

import android.app.DownloadManager
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.net.Uri
import android.os.Environment
import android.util.Log
import `in`.prinny.app.BuildConfig
import org.json.JSONObject
import java.io.BufferedReader
import java.io.InputStreamReader
import java.net.HttpURLConnection
import java.net.URL

class UpdateChecker(private val context: Context) {

    companion object {
        private const val TAG = "UpdateChecker"
        private const val RELEASE_JSON_URL =
            "https://github.com/coffeegrind123/prinny-client/releases/download/tauri/release.json"
    }

    fun check() {
        Thread {
            try {
                val json = fetchReleaseJson() ?: return@Thread
                // Android lives at the top-level `android` key (not under `platforms`,
                // which the Tauri updater deserializes as { signature, url } per entry —
                // putting android there fails with "missing field signature").
                // Fall back to platforms.android for older release.json versions.
                val android = json.optJSONObject("android")
                    ?: json.optJSONObject("platforms")?.optJSONObject("android")
                    ?: return@Thread

                val latestVersion = android.optString("version", "").removePrefix("v")
                val apkUrl = android.optString("url", "")
                val sha256 = android.optString("sha256", "")

                if (latestVersion.isEmpty() || apkUrl.isEmpty()) {
                    Log.w(TAG, "release.json missing android version or URL")
                    return@Thread
                }

                val currentVersion = BuildConfig.VERSION_NAME.removePrefix("v")

                if (isNewer(latestVersion, currentVersion)) {
                    Log.i(TAG, "New version available: v$latestVersion (current: v$currentVersion)")
                    downloadApk(apkUrl, latestVersion, sha256)
                } else {
                    Log.d(TAG, "Already up to date (v$currentVersion)")
                }
            } catch (e: Exception) {
                Log.w(TAG, "Update check failed", e)
            }
        }.start()
    }

    private fun fetchReleaseJson(): JSONObject? {
        var connection: HttpURLConnection? = null
        try {
            val url = URL(RELEASE_JSON_URL)
            connection = url.openConnection() as HttpURLConnection
            connection.connectTimeout = 15_000
            connection.readTimeout = 15_000
            connection.instanceFollowRedirects = true

            if (connection.responseCode != 200) {
                Log.w(TAG, "HTTP ${connection.responseCode} fetching release.json")
                return null
            }

            val reader = BufferedReader(InputStreamReader(connection.inputStream))
            val body = reader.use { it.readText() }
            return JSONObject(body)
        } finally {
            connection?.disconnect()
        }
    }

    private fun isNewer(latest: String, current: String): Boolean {
        val latestParts = latest.split(".").map { it.toIntOrNull() ?: 0 }
        val currentParts = current.split(".").map { it.toIntOrNull() ?: 0 }
        val maxLen = maxOf(latestParts.size, currentParts.size)

        for (i in 0 until maxLen) {
            val l = latestParts.getOrElse(i) { 0 }
            val c = currentParts.getOrElse(i) { 0 }
            if (l > c) return true
            if (l < c) return false
        }
        return false
    }

    private fun downloadApk(url: String, version: String, sha256: String) {
        val filename = "cinny-v$version.apk"

        val request = DownloadManager.Request(Uri.parse(url))
            .setTitle("Cinny v$version")
            .setDescription("Downloading update...")
            .setNotificationVisibility(DownloadManager.Request.VISIBILITY_VISIBLE_NOTIFY_COMPLETED)
            .setDestinationInExternalPublicDir(Environment.DIRECTORY_DOWNLOADS, filename)
            .setAllowedOverMetered(true)
            .setAllowedOverRoaming(true)

        val dm = context.getSystemService(Context.DOWNLOAD_SERVICE) as DownloadManager
        dm.enqueue(request)
        Log.i(TAG, "Download queued: $filename")
    }
}
