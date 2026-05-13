package `in`.prinny.app

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.os.Build
import android.util.Log
import org.unifiedpush.android.connector.FailedReason
import org.unifiedpush.android.connector.MessagingReceiver
import org.unifiedpush.android.connector.UnifiedPush
import org.unifiedpush.android.connector.data.PushEndpoint
import org.unifiedpush.android.connector.data.PushMessage

class UnifiedPushReceiver : MessagingReceiver() {

    companion object {
        const val TAG = "UnifiedPushReceiver"
        const val CHANNEL_ID = "cinny_messages"
        const val NOTIFICATION_ID = 100
    }

    private fun createNotificationChannel(context: Context) {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            val channel = NotificationChannel(
                CHANNEL_ID,
                "Messages",
                NotificationManager.IMPORTANCE_HIGH
            ).apply {
                description = "Incoming Matrix messages"
                setShowBadge(true)
            }
            val manager = context.getSystemService(NotificationManager::class.java)
            manager.createNotificationChannel(channel)
        }
    }

    /**
     * Show a system notification directly from the receiver.
     * Used when the app is not running (plugin instance is null).
     */
    private fun showNotification(context: Context, body: String) {
        createNotificationChannel(context)

        val openIntent = PendingIntent.getActivity(
            context,
            0,
            Intent(context, MainActivity::class.java),
            PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT
        )

        val notification = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            Notification.Builder(context, CHANNEL_ID)
                .setContentTitle("Cinny")
                .setContentText(body)
                .setSmallIcon(android.R.drawable.ic_dialog_info)
                .setAutoCancel(true)
                .setContentIntent(openIntent)
                .build()
        } else {
            @Suppress("DEPRECATION")
            Notification.Builder(context)
                .setContentTitle("Cinny")
                .setContentText(body)
                .setSmallIcon(android.R.drawable.ic_dialog_info)
                .setAutoCancel(true)
                .setContentIntent(openIntent)
                .build()
        }

        val manager = context.getSystemService(NotificationManager::class.java)
        manager.notify(NOTIFICATION_ID, notification)
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
        Log.i(TAG, "Push message received (${message.content.size} bytes, decrypted=${message.decrypted})")
        val body = message.content.toString(Charsets.UTF_8)

        val plugin = UnifiedPushPlugin.instance
        if (plugin != null) {
            // App is running — forward to the plugin which triggers Matrix sync
            plugin.onMessage(message.content)
        } else {
            // App is not running — show a notification directly
            showNotification(context, body)
        }
    }
}
