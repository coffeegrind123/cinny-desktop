package `in`.cinny.app

import android.Manifest
import android.app.Activity
import android.webkit.WebView
import app.tauri.annotation.Command
import app.tauri.annotation.Permission
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Plugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import org.unifiedpush.android.connector.UnifiedPush
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.launch

val upScope = CoroutineScope(Dispatchers.Main + SupervisorJob())

@TauriPlugin(
    permissions = [
        Permission(
            strings = [Manifest.permission.POST_NOTIFICATIONS],
            alias = "notifications"
        )
    ]
)
class UnifiedPushPlugin(private val activity: Activity) : Plugin(activity) {

    companion object {
        const val TAG = "UnifiedPushPlugin"
        var instance: UnifiedPushPlugin? = null
    }

    private var savedEndpoint: String? = null

    override fun load(webView: WebView) {
        super.load(webView)
        instance = this
    }

    @Command
    fun register(invoke: Invoke) {
        upScope.launch {
            try {
                UnifiedPush.tryUseCurrentOrDefaultDistributor(activity) { success ->
                    if (!success) {
                        invoke.reject("No UnifiedPush distributor available. Install ntfy or NextPush.")
                        return@tryUseCurrentOrDefaultDistributor
                    }

                    UnifiedPush.register(activity)
                    // The endpoint will arrive via onNewEndpoint -> trigger event
                    // We resolve the invoke when we get it, or timeout
                    instance = this@UnifiedPushPlugin
                    savedEndpoint = null // reset, will be set by onNewEndpoint

                    // Start a timer to check for endpoint
                    upScope.launch {
                        var attempts = 0
                        while (savedEndpoint == null && attempts < 30) {
                            kotlinx.coroutines.delay(1000)
                            attempts++
                        }
                        if (savedEndpoint != null) {
                            val result = JSObject()
                            result.put("endpoint", savedEndpoint!!)
                            invoke.resolve(result)
                        } else {
                            invoke.reject("Timed out waiting for UnifiedPush endpoint")
                        }
                    }
                }
            } catch (e: Exception) {
                invoke.reject("UnifiedPush registration error: ${e.message}", e)
            }
        }
    }

    @Command
    fun getEndpoint(invoke: Invoke) {
        val endpoint = savedEndpoint
        if (endpoint != null) {
            val result = JSObject()
            result.put("endpoint", endpoint)
            invoke.resolve(result)
        } else {
            invoke.reject("No UnifiedPush endpoint available")
        }
    }

    @Command
    fun getDistributors(invoke: Invoke) {
        try {
            val distributors = UnifiedPush.getDistributors(activity)
            val result = JSObject()
            val arr = org.json.JSONArray()
            for (d in distributors) {
                arr.put(d)
            }
            result.put("distributors", arr.toString())
            invoke.resolve(result)
        } catch (e: Exception) {
            invoke.reject("Failed to get distributors: ${e.message}", e)
        }
    }

    @Command
    override fun requestPermissions(invoke: Invoke) {
        upScope.launch {
            requestPermissionForAlias("notifications", invoke, "requestPermissionsCallback")
        }
    }

    @app.tauri.annotation.PermissionCallback
    fun requestPermissionsCallback(invoke: Invoke) {
        val granted = getPermissionState("notifications").toString().lowercase() == "granted"
        val result = JSObject()
        result.put("granted", granted)
        invoke.resolve(result)
    }

    // Called by UnifiedPushReceiver
    fun onNewEndpoint(endpoint: String) {
        savedEndpoint = endpoint
        val data = JSObject()
        data.put("endpoint", endpoint)
        trigger("endpoint-received", data)
    }

    fun onRegistrationFailed(reason: String) {
        val data = JSObject()
        data.put("reason", reason)
        trigger("registration-failed", data)
    }

    fun onUnregistered() {
        savedEndpoint = null
        trigger("unregistered", JSObject())
    }

    fun onMessage(message: ByteArray) {
        val body = message.toString(Charsets.UTF_8)
        val data = JSObject()
        data.put("body", body)
        trigger("message-received", data)
    }
}
