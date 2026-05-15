package `in`.prinny.app

import android.app.Activity
import android.content.Context
import android.util.Log
import app.tauri.annotation.Command
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.JSObject
import app.tauri.plugin.Invoke
import app.tauri.plugin.Plugin
import kotlinx.coroutines.*
import org.json.JSONObject
import java.io.File
import java.io.InputStreamReader
import java.net.HttpURLConnection
import java.net.URL
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.TimeUnit

@TauriPlugin
class YtDlpPlugin(private val activity: Activity) : Plugin(activity) {

    companion object {
        private const val TAG = "YtDlpPlugin"
        private const val BASENAME = "prinny-ytdlp"
        private const val PYTHON_DIR = "python"
        private const val YTDLP_DIR = "yt-dlp"
        private const val YTDLP_BIN = "yt-dlp"

        // deniscerri/ytdlnis-packages releases provide prebuilt Python for Android
        private const val PACKAGES_REPO = "https://github.com/deniscerri/ytdlnis-packages/releases"
        private const val PYTHON_VERSION = "v3.12.11"
    }

    private val baseDir: File by lazy {
        File(activity.noBackupFilesDir, BASENAME).apply { if (!exists()) mkdir() }
    }
    private val pythonDir: File by lazy { File(baseDir, PYTHON_DIR) }
    private val ytdlpDir: File by lazy { File(baseDir, YTDLP_DIR) }
    private val ytdlpBin: File by lazy { File(ytdlpDir, YTDLP_BIN) }
    private val pythonBin: File by lazy {
        File(pythonDir, "usr/bin/python3").takeIf { it.exists() }
            ?: File(pythonDir, "usr/bin/python")
    }

    private val job = SupervisorJob()
    private val scope = CoroutineScope(Dispatchers.IO + job)
    private val activeDownloads = ConcurrentHashMap<String, Process>()

    private var pythonReady = false
    private var ytdlpReady = false

    init {
        scope.launch { ensurePackages() }
    }

    private suspend fun ensurePackages() = withContext(Dispatchers.IO) {
        if (pythonReady && ytdlpReady) return@withContext

        pythonDir.mkdirs()
        ytdlpDir.mkdirs()

        if (!pythonReady) {
            try {
                downloadPython()
                pythonReady = true
            } catch (e: Exception) {
                Log.e(TAG, "Failed to setup Python: ${e.message}")
            }
        }

        if (!ytdlpReady) {
            try {
                if (!ytdlpBin.exists()) {
                    downloadYtDlp()
                }
                ytdlpReady = true
            } catch (e: Exception) {
                Log.e(TAG, "Failed to setup yt-dlp: ${e.message}")
            }
        }
    }

    private fun downloadPython() {
        val zipFile = File(baseDir, "python.zip")
        try {
            val abi = android.os.Build.SUPPORTED_ABIS[0] // e.g. "arm64-v8a"
            val assetName = "python-$PYTHON_VERSION-android-$abi.zip"

            val releaseUrl = "$PACKAGES_REPO/download/$PYTHON_VERSION/$assetName"
            Log.i(TAG, "Downloading Python from $releaseUrl")
            downloadFile(releaseUrl, zipFile)

            Log.i(TAG, "Extracting Python...")
            pythonDir.mkdirs()
            val process = ProcessBuilder("unzip", "-o", zipFile.absolutePath, "-d", pythonDir.absolutePath)
                .redirectErrorStream(true)
                .start()
            process.waitFor(120, TimeUnit.SECONDS)
            zipFile.delete()

            // Make executable
            listOf("python3", "python").forEach { name ->
                File(pythonDir, "usr/bin/$name").let {
                    if (it.exists()) it.setExecutable(true)
                }
            }
            Log.i(TAG, "Python ready")
        } catch (e: Exception) {
            Log.e(TAG, "Python download failed", e)
            zipFile.delete()
            throw e
        }
    }

    private fun downloadYtDlp() {
        val url = "https://api.github.com/repos/yt-dlp/yt-dlp/releases/latest"
        try {
            val json = fetchJsonFromUrl(url) ?: throw Exception("Failed to fetch release info")
            val release = JSONObject(json)
            val assets = release.getJSONArray("assets")

            var downloadUrl: String? = null
            for (i in 0 until assets.length()) {
                val asset = assets.getJSONObject(i)
                if (asset.getString("name") == "yt-dlp") {
                    downloadUrl = asset.getString("browser_download_url")
                    break
                }
            }

            if (downloadUrl == null) throw Exception("No yt-dlp binary found in release")

            Log.i(TAG, "Downloading yt-dlp from $downloadUrl")
            ytdlpDir.mkdirs()
            downloadFile(downloadUrl, ytdlpBin)
            ytdlpBin.setExecutable(true)
            Log.i(TAG, "yt-dlp ready")
        } catch (e: Exception) {
            Log.e(TAG, "yt-dlp download failed", e)
            throw e
        }
    }

    private fun downloadFile(urlStr: String, dest: File) {
        val url = URL(urlStr)
        val conn = url.openConnection() as HttpURLConnection
        conn.requestMethod = "GET"
        conn.connectTimeout = 15000
        conn.readTimeout = 60000
        conn.setRequestProperty("User-Agent", "prinny-ytdlp")

        conn.inputStream.use { input ->
            dest.outputStream().use { output ->
                input.copyTo(output, 8192)
            }
        }
        conn.disconnect()
    }

    private fun fetchJsonFromUrl(urlStr: String): String? {
        val url = URL(urlStr)
        val conn = url.openConnection() as HttpURLConnection
        conn.requestMethod = "GET"
        conn.connectTimeout = 10000
        conn.readTimeout = 10000
        conn.setRequestProperty("User-Agent", "prinny-ytdlp")

        return if (conn.responseCode == HttpURLConnection.HTTP_OK) {
            conn.inputStream.bufferedReader().use { it.readText() }
        } else {
            null
        }.also { conn.disconnect() }
    }

    // ---- Tauri Commands ----

    @Command
    suspend fun getVersion(invoke: Invoke) = withContext(Dispatchers.IO) {
        ensurePackages()
        if (!ytdlpReady) {
            invoke.reject("yt-dlp is not available yet")
            return@withContext
        }
        try {
            val result = execute(arrayOf("--version"))
            val version = result.out.trim()
            val js = JSObject()
            js.put("version", version)
            js.put("source", "bundled")
            invoke.resolve(js)
        } catch (e: Exception) {
            invoke.reject(e.message ?: "Unknown error")
        }
    }

    @Command
    suspend fun getVideoInfo(invoke: Invoke) = withContext(Dispatchers.IO) {
        ensurePackages()
        val url = invoke.getString("url") ?: run {
            invoke.reject("Missing url parameter")
            return@withContext
        }
        try {
            val result = execute(arrayOf("--dump-json", "--no-download", "--no-warnings", url))
            val json = JSONObject(result.out)
            val js = JSObject()
            js.put("title", json.optString("title", "Unknown"))
            js.put("duration", json.optLong("duration", -1))
            js.put("uploader", json.optString("uploader", null))
            invoke.resolve(js)
        } catch (e: Exception) {
            invoke.reject(e.message ?: "Unknown error")
        }
    }

    @Command
    suspend fun downloadVideo(invoke: Invoke) = withContext(Dispatchers.IO) {
        ensurePackages()
        val url = invoke.getString("url") ?: run {
            invoke.reject("Missing url parameter")
            return@withContext
        }
        val quality = invoke.getString("quality")

        try {
            val downloadDir = android.os.Environment.getExternalStoragePublicDirectory(
                android.os.Environment.DIRECTORY_DOWNLOADS
            )
            val args = mutableListOf(
                "--output", "${downloadDir.absolutePath}/%(title)s.%(ext)s",
                "--newline", "--progress", "--no-warnings",
                "--merge-output-format", "mp4"
            )
            if (quality != null) {
                when (quality) {
                    "best" -> args.addAll(listOf("-f", "bestvideo[ext=mp4]+bestaudio[ext=mp4]/best[ext=mp4]"))
                    "worst" -> args.addAll(listOf("-f", "worstvideo[ext=mp4]+worstaudio[ext=mp4]/worst[ext=mp4]"))
                    else -> args.addAll(listOf("-f", "$quality+bestaudio[ext=mp4]/best[ext=mp4]"))
                }
            } else {
                args.addAll(listOf("-f", "bestvideo[ext=mp4]+bestaudio[ext=mp4]/best[ext=mp4]"))
            }
            args.add(url)

            val processId = url.hashCode().toString()
            executeWithProgress(args, processId)
            invoke.resolve("Download completed")
        } catch (e: CancellationException) {
            invoke.reject("Download cancelled")
        } catch (e: Exception) {
            invoke.reject(e.message ?: "Download failed")
        }
    }

    @Command
    fun cancelDownload(invoke: Invoke) {
        // Kill all active download processes
        activeDownloads.values.forEach { process ->
            process.destroyForcibly()
        }
        activeDownloads.clear()
        invoke.resolve()
    }

    @Command
    suspend fun checkUpdate(invoke: Invoke) = withContext(Dispatchers.IO) {
        try {
            val current = execute(arrayOf("--version")).out.trim()
            val releaseJson = fetchJsonFromUrl(
                "https://api.github.com/repos/yt-dlp/yt-dlp/releases/latest"
            )
            val release = JSONObject(releaseJson ?: "{}")
            val latest = release.optString("tag_name", "").removePrefix("v")
            val result = JSObject()
            result.put("update_available", current != latest)
            invoke.resolve(result)
        } catch (e: Exception) {
            invoke.reject(e.message ?: "Check failed")
        }
    }

    @Command
    suspend fun downloadBinary(invoke: Invoke) = withContext(Dispatchers.IO) {
        try {
            if (ytdlpBin.exists()) ytdlpBin.delete()
            downloadYtDlp()
            val version = execute(arrayOf("--version")).out.trim()
            invoke.resolve(version)
        } catch (e: Exception) {
            invoke.reject(e.message ?: "Download failed")
        }
    }

    // ---- Execution ----

    data class ExecResult(val out: String, val err: String, val code: Int)

    private fun execute(args: Array<String>): ExecResult {
        val cmd = if (pythonBin.exists()) {
            arrayOf(pythonBin.absolutePath, ytdlpBin.absolutePath) + args
        } else {
            arrayOf(ytdlpBin.absolutePath) + args
        }

        val pb = ProcessBuilder(*cmd).redirectErrorStream(false)
        pb.environment().apply {
            if (pythonBin.exists()) {
                this["HOME"] = pythonDir.absolutePath + "/usr"
                this["PYTHONHOME"] = pythonDir.absolutePath + "/usr"
                this["PATH"] = pythonDir.absolutePath + "/usr/bin:" + (System.getenv("PATH") ?: "")
                val libDir = if (File(pythonDir, "usr/lib").exists()) {
                    pythonDir.absolutePath + "/usr/lib"
                } else {
                    activity.applicationInfo.nativeLibraryDir
                }
                this["LD_LIBRARY_PATH"] = libDir
            }
        }

        val process = pb.start()
        val out = process.inputStream.bufferedReader().readText()
        val err = process.errorStream.bufferedReader().readText()
        val code = process.waitFor()

        if (code != 0) {
            throw Exception("yt-dlp exited with code $code: $err")
        }
        return ExecResult(out, err, code)
    }

    private fun executeWithProgress(args: List<String>, processId: String) {
        val cmd = if (pythonBin.exists()) {
            arrayOf(pythonBin.absolutePath, ytdlpBin.absolutePath) + args
        } else {
            arrayOf(ytdlpBin.absolutePath) + args
        }

        val pb = ProcessBuilder(*cmd).redirectErrorStream(true)
        pb.environment().apply {
            if (pythonBin.exists()) {
                this["HOME"] = pythonDir.absolutePath + "/usr"
                this["PYTHONHOME"] = pythonDir.absolutePath + "/usr"
                this["PATH"] = pythonDir.absolutePath + "/usr/bin:" + (System.getenv("PATH") ?: "")
                val libDir = File(pythonDir, "usr/lib").takeIf { it.exists() }
                    ?.absolutePath ?: activity.applicationInfo.nativeLibraryDir
                this["LD_LIBRARY_PATH"] = libDir
            }
        }

        val process = pb.start()
        activeDownloads[processId] = process

        val reader = InputStreamReader(process.inputStream)
        val buffer = StringBuilder()
        val charBuf = CharArray(1024)

        try {
            var charsRead: Int
            while (reader.read(charBuf).also { charsRead = it } != -1) {
                buffer.append(charBuf, 0, charsRead)
                // Emit complete lines as progress events
                var newlineIdx: Int
                while (buffer.indexOf('\n').also { newlineIdx = it } != -1) {
                    val line = buffer.substring(0, newlineIdx).trim()
                    buffer.delete(0, newlineIdx + 1)
                    if (line.isNotEmpty()) {
                        val payload = JSObject()
                        payload.put("payload", line)
                        activity.runOnUiThread {
                            invoke("ytdlp-output", payload)
                        }
                    }
                }
            }
            // Emit remaining
            if (buffer.isNotEmpty()) {
                val payload = JSObject()
                payload.put("payload", buffer.toString().trim())
                activity.runOnUiThread {
                    invoke("ytdlp-output", payload)
                }
            }
        } finally {
            reader.close()
            activeDownloads.remove(processId)
        }

        val code = process.waitFor()
        if (code != 0) {
            throw Exception("yt-dlp exited with code $code")
        }

        // Emit completion
        val payload = JSObject()
        payload.put("payload", "DOWNLOAD_COMPLETE")
        activity.runOnUiThread {
            invoke("ytdlp-output", payload)
        }
    }

    override fun onDestroy() {
        job.cancel()
        activeDownloads.values.forEach { it.destroyForcibly() }
        activeDownloads.clear()
        super.onDestroy()
    }
}
