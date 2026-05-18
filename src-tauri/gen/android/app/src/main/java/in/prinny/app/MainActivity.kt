package `in`.prinny.app

import android.Manifest
import android.content.Intent
import android.content.pm.PackageManager
import android.os.Build
import android.os.Bundle
import android.view.View
import android.view.ViewGroup
import android.webkit.PermissionRequest
import android.webkit.WebChromeClient
import android.webkit.WebView
import androidx.activity.enableEdgeToEdge
import androidx.activity.result.contract.ActivityResultContracts
import androidx.core.content.ContextCompat

class MainActivity : TauriActivity() {
    /**
     * The Element Call iframe (and any other WebRTC content embedded in
     * the WebView) calls `navigator.mediaDevices.getUserMedia`, which
     * triggers `WebChromeClient.onPermissionRequest` on Android. Tauri's
     * default chrome client denies these requests, so calls silently
     * never connect mic/camera.
     *
     * We swap the WebView's chrome client out for one that maps each
     * requested resource to its Android runtime permission, prompts the
     * user when needed, and grants whatever the OS allowed.
     */

    private var pendingWebPermissionRequest: PermissionRequest? = null

    private val webPermissionLauncher =
        registerForActivityResult(ActivityResultContracts.RequestMultiplePermissions()) { granted ->
            val req = pendingWebPermissionRequest ?: return@registerForActivityResult
            pendingWebPermissionRequest = null

            val allowed = req.resources.filter { resource ->
                when (resource) {
                    PermissionRequest.RESOURCE_AUDIO_CAPTURE ->
                        granted[Manifest.permission.RECORD_AUDIO] == true ||
                            hasRuntimePermission(Manifest.permission.RECORD_AUDIO)
                    PermissionRequest.RESOURCE_VIDEO_CAPTURE ->
                        granted[Manifest.permission.CAMERA] == true ||
                            hasRuntimePermission(Manifest.permission.CAMERA)
                    else -> false
                }
            }.toTypedArray()

            if (allowed.isNotEmpty()) req.grant(allowed) else req.deny()
        }

    override fun onCreate(savedInstanceState: Bundle?) {
        enableEdgeToEdge()
        super.onCreate(savedInstanceState)
        UpdateChecker(this).check()

        // Tauri creates the WebView during super.onCreate via JNI. Hook
        // the chrome client on the next post so the view hierarchy is
        // settled. Retry once after a short delay in case the WebView
        // is attached lazily on some devices.
        window.decorView.post { installRtcPermissionHandler() }
        window.decorView.postDelayed({
            if (!chromeClientInstalled) installRtcPermissionHandler()
        }, 1500L)

        // Cold start from a notification tap: forward the roomId/eventId
        // extras to the JS layer so it can navigate to the room. The
        // plugin may not have loaded yet — MessageNotificationPlugin
        // stashes the click and replays it on load().
        handleNotificationIntent(intent)
    }

    // Hot start: the activity is already running and the system delivers
    // a new intent (FLAG_ACTIVITY_SINGLE_TOP is set on the PendingIntent
    // we build in MessageNotificationPlugin.show). Forward the extras
    // straight to the plugin.
    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        setIntent(intent)
        handleNotificationIntent(intent)
    }

    private fun handleNotificationIntent(intent: Intent?) {
        val roomId = intent?.getStringExtra("roomId") ?: return
        val eventId = intent.getStringExtra("eventId") ?: return
        if (roomId.isEmpty()) return
        MessageNotificationPlugin.deliverClick(roomId, eventId)
    }

    private var chromeClientInstalled = false

    private fun installRtcPermissionHandler() {
        if (chromeClientInstalled) return
        val webView = findWebView(window.decorView as? ViewGroup) ?: return
        webView.webChromeClient = RtcChromeClient()
        chromeClientInstalled = true
    }

    private fun findWebView(root: ViewGroup?): WebView? {
        if (root == null) return null
        for (i in 0 until root.childCount) {
            val child: View = root.getChildAt(i)
            if (child is WebView) return child
            if (child is ViewGroup) {
                findWebView(child)?.let { return it }
            }
        }
        return null
    }

    private fun hasRuntimePermission(perm: String): Boolean =
        ContextCompat.checkSelfPermission(this, perm) == PackageManager.PERMISSION_GRANTED

    private inner class RtcChromeClient : WebChromeClient() {
        override fun onPermissionRequest(request: PermissionRequest) {
            // RESOURCE_PROTECTED_MEDIA_ID (DRM) gets auto-granted — it's
            // the standard policy for WebView and unrelated to mic/cam.
            val resources = request.resources
            val toRequest = mutableListOf<String>()
            val canGrantImmediately = mutableListOf<String>()

            for (resource in resources) {
                when (resource) {
                    PermissionRequest.RESOURCE_AUDIO_CAPTURE -> {
                        if (hasRuntimePermission(Manifest.permission.RECORD_AUDIO)) {
                            canGrantImmediately.add(resource)
                        } else {
                            toRequest.add(Manifest.permission.RECORD_AUDIO)
                        }
                    }
                    PermissionRequest.RESOURCE_VIDEO_CAPTURE -> {
                        if (hasRuntimePermission(Manifest.permission.CAMERA)) {
                            canGrantImmediately.add(resource)
                        } else {
                            toRequest.add(Manifest.permission.CAMERA)
                        }
                    }
                    // Anything else (DRM, MIDI, etc.) — pass through to
                    // the default behavior, which denies.
                }
            }

            if (toRequest.isEmpty()) {
                if (canGrantImmediately.isNotEmpty()) {
                    request.grant(canGrantImmediately.toTypedArray())
                } else {
                    request.deny()
                }
                return
            }

            // We need to prompt for at least one runtime permission.
            // Stash the request so the launcher callback can grant/deny
            // after the system dialog returns. Only one outstanding
            // request is supported — if a second one arrives mid-flight,
            // deny it.
            if (pendingWebPermissionRequest != null) {
                request.deny()
                return
            }
            pendingWebPermissionRequest = request
            webPermissionLauncher.launch(toRequest.toTypedArray())
        }

        override fun onPermissionRequestCanceled(request: PermissionRequest?) {
            super.onPermissionRequestCanceled(request)
            if (pendingWebPermissionRequest === request) {
                pendingWebPermissionRequest = null
            }
        }
    }
}
