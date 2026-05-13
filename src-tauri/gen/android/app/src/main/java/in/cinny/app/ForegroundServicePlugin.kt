package `in`.cinny.app

import android.app.Activity
import android.content.Intent
import android.content.SharedPreferences
import android.os.Build
import android.webkit.WebView
import androidx.preference.PreferenceManager
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

    /**
     * Toggle background service mode: hides the persistent notification
     * while keeping the process alive. Same as FairEmail's isBackgroundService().
     * On Android 8+, the user can also disable the channel in system settings.
     */
    @Command
    fun setBackgroundService(invoke: Invoke) {
        val enabled = invoke.parseJSObject().optBoolean("enabled", false)
        val prefs = PreferenceManager.getDefaultSharedPreferences(activity)
        prefs.edit().putBoolean("background_service", enabled).apply()

        if (enabled) {
            activity.stopService(Intent(activity, ForegroundService::class.java))
            // Restart without foreground notification
            val intent = Intent(activity, ForegroundService::class.java)
            activity.startService(intent)
        } else {
            // Restart with foreground notification
            val intent = Intent(activity, ForegroundService::class.java)
            activity.stopService(intent)
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                activity.startForegroundService(intent)
            } else {
                activity.startService(intent)
            }
        }
        invoke.resolve()
    }

    @Command
    fun isBackgroundService(invoke: Invoke) {
        val result = JSObject()
        result.put("enabled", ForegroundService.isBackgroundService(activity))
        invoke.resolve(result)
    }
}
