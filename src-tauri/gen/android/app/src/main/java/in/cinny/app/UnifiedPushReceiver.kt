package `in`.cinny.app

import android.content.Context
import android.util.Log
import org.unifiedpush.android.connector.MessagingReceiver
import org.unifiedpush.android.connector.UnifiedPush
import org.unifiedpush.android.connector.data.PushEndpoint
import org.unifiedpush.android.connector.data.PushMessage

class UnifiedPushReceiver : MessagingReceiver() {

    companion object {
        const val TAG = "UnifiedPushReceiver"
    }

    override fun onNewEndpoint(
        context: Context,
        endpoint: PushEndpoint,
        instance: String,
    ) {
        Log.i(TAG, "New endpoint received: ${endpoint.url}")
        UnifiedPushPlugin.instance?.onNewEndpoint(endpoint.url)
    }

    override fun onRegistrationFailed(
        context: Context,
        reason: FailedReason,
        instance: String,
    ) {
        Log.w(TAG, "Registration failed: $reason")
        UnifiedPushPlugin.instance?.onRegistrationFailed(reason.toString())
    }

    override fun onUnregistered(
        context: Context,
        instance: String,
    ) {
        Log.i(TAG, "Unregistered")
        UnifiedPushPlugin.instance?.onUnregistered()
    }

    override fun onMessage(
        context: Context,
        message: PushMessage,
        instance: String,
    ) {
        Log.i(TAG, "Push message received (${message.message.size} bytes, encrypted=${message.isEncrypted})")
        UnifiedPushPlugin.instance?.onMessage(message.message)
    }
}
