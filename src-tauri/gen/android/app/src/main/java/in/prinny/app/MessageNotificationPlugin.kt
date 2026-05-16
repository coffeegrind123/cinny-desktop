package `in`.prinny.app

import android.app.Activity
import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Intent
import android.graphics.BitmapFactory
import android.os.Build
import androidx.core.app.NotificationCompat
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.Plugin

/**
 * Direct notification path for Android — bypasses tauri-plugin-notification
 * because its `icon` field only resolves drawable resource names, not file
 * paths. We need file paths so per-message sender avatars (downloaded by
 * cache_notification_icon into the app cache dir) can be passed as a
 * Notification.Builder large icon.
 */
@InvokeArg
class ShowMessageNotificationArgs {
    lateinit var title: String
    lateinit var body: String
    var iconPath: String? = null
    var roomId: String? = null
    var eventId: String? = null
    var notificationId: Int? = null
}

@TauriPlugin
class MessageNotificationPlugin(private val activity: Activity) : Plugin(activity) {
    companion object {
        const val CHANNEL_ID = "prinny_messages"
        private var nextId = 1000
    }

    private fun ensureChannel() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            val nm = activity.getSystemService(NotificationManager::class.java) ?: return
            if (nm.getNotificationChannel(CHANNEL_ID) == null) {
                val channel = NotificationChannel(
                    CHANNEL_ID,
                    "Messages",
                    NotificationManager.IMPORTANCE_HIGH,
                ).apply {
                    description = "New Matrix messages"
                }
                nm.createNotificationChannel(channel)
            }
        }
    }

    @Command
    fun show(invoke: Invoke) {
        val args = invoke.parseArgs(ShowMessageNotificationArgs::class.java)
        ensureChannel()

        val openIntent = Intent(activity, MainActivity::class.java).apply {
            flags = Intent.FLAG_ACTIVITY_SINGLE_TOP or Intent.FLAG_ACTIVITY_CLEAR_TOP
            args.roomId?.let { putExtra("roomId", it) }
            args.eventId?.let { putExtra("eventId", it) }
        }
        val pendingIntent = PendingIntent.getActivity(
            activity,
            (args.notificationId ?: nextId),
            openIntent,
            PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT,
        )

        val builder = NotificationCompat.Builder(activity, CHANNEL_ID)
            .setContentTitle(args.title)
            .setContentText(args.body)
            .setStyle(NotificationCompat.BigTextStyle().bigText(args.body))
            .setSmallIcon(android.R.drawable.stat_notify_chat)
            .setAutoCancel(true)
            .setContentIntent(pendingIntent)
            .setPriority(NotificationCompat.PRIORITY_HIGH)
            .setCategory(NotificationCompat.CATEGORY_MESSAGE)

        args.iconPath?.let { path ->
            try {
                BitmapFactory.decodeFile(path)?.let { bitmap ->
                    builder.setLargeIcon(bitmap)
                }
            } catch (_: Throwable) {
            }
        }

        val notificationId = args.notificationId ?: nextId++
        val nm = activity.getSystemService(NotificationManager::class.java)
        nm?.notify(notificationId, builder.build())

        invoke.resolve()
    }
}
