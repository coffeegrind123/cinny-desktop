package `in`.cinny.app

import android.app.Activity
import android.content.Intent
import android.os.Build
import android.webkit.WebView
import app.tauri.annotation.Command
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.JSObject
import app.tauri.plugin.Invoke
import app.tauri.plugin.Plugin

@TauriPlugin
class ForegroundServicePlugin(private val activity: Activity) : Plugin(activity) {

    @Command
    fun startForeground(invoke: Invoke) {
        val intent = Intent(activity, ForegroundService::class.java)
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            activity.startForegroundService(intent)
        } else {
            activity.startService(intent)
        }
        invoke.resolve()
    }

    @Command
    fun stopForeground(invoke: Invoke) {
        val intent = Intent(activity, ForegroundService::class.java)
        activity.stopService(intent)
        invoke.resolve()
    }

    @Command
    fun isForegroundRunning(invoke: Invoke) {
        val result = JSObject()
        result.put("running", ForegroundService.isRunning)
        invoke.resolve(result)
    }
}
