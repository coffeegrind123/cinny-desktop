package `in`.cinny.app

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.Intent
import android.content.pm.ServiceInfo
import android.os.Build
import android.os.IBinder

/**
 * Foreground service that keeps the Matrix WebSocket alive in the background.
 *
 * Pattern taken from FairEmail's ServiceSynchronize:
 * - Persistent notification with IMPORTANCE_LOW (silent, collapsed in tray)
 * - FLAG_NO_CLEAR prevents accidental dismissal
 * - FOREGROUND_SERVICE_DEFAULT for proper Android 14+ behavior
 * - User can hide the notification by disabling the channel in system settings;
 *   the foreground service continues running regardless.
 */
class ForegroundService : Service() {

    companion object {
        const val CHANNEL_ID = "cinny_foreground"
        const val NOTIFICATION_ID = 1
        var isRunning = false
            private set
    }

    override fun onCreate() {
        super.onCreate()
        createNotificationChannel()
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        val notification = buildNotification()
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.UPSIDE_DOWN_CAKE) {
            startForeground(
                NOTIFICATION_ID,
                notification,
                ServiceInfo.FOREGROUND_SERVICE_TYPE_DATA_SYNC
            )
        } else {
            startForeground(NOTIFICATION_ID, notification)
        }
        isRunning = true
        return START_STICKY
    }

    override fun onBind(intent: Intent?): IBinder? = null

    override fun onDestroy() {
        isRunning = false
        super.onDestroy()
    }

    private fun createNotificationChannel() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            val channel = NotificationChannel(
                CHANNEL_ID,
                "Background service",
                NotificationManager.IMPORTANCE_LOW
            ).apply {
                description = "Shown when Cinny keeps the Matrix connection alive in the background. " +
                    "Disable this channel to hide the notification; the service will continue running."
                setShowBadge(false)
            }
            val manager = getSystemService(NotificationManager::class.java)
            manager.createNotificationChannel(channel)
        }
    }

    /**
     * Build the persistent foreground notification.
     *
     * FairEmail-inspired: PRIORITY_MIN (no sound/vibration), CATEGORY_SERVICE,
     * FLAG_NO_CLEAR (non-dismissible), setShowWhen(false), VISIBILITY_SECRET
     * (no content on lockscreen). The notification is as unobtrusive as possible
     * while still satisfying Android's foreground service requirement.
     */
    private fun buildNotification(): Notification {
        val openIntent = PendingIntent.getActivity(
            this,
            0,
            Intent(this, MainActivity::class.java),
            PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT
        )

        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            val n = Notification.Builder(this, CHANNEL_ID)
                .setContentTitle("Cinny")
                .setContentText("Connected to Matrix")
                .setSmallIcon(android.R.drawable.ic_dialog_info)
                .setOngoing(true)
                .setContentIntent(openIntent)
                .setShowWhen(false)
                .setCategory(Notification.CATEGORY_SERVICE)
                .setVisibility(Notification.VISIBILITY_SECRET)
                .build()
            n.flags = n.flags or Notification.FLAG_NO_CLEAR
            return n
        } else {
            @Suppress("DEPRECATION")
            val n = Notification.Builder(this)
                .setContentTitle("Cinny")
                .setContentText("Connected to Matrix")
                .setSmallIcon(android.R.drawable.ic_dialog_info)
                .setOngoing(true)
                .setContentIntent(openIntent)
                .setPriority(Notification.PRIORITY_MIN)
                .build()
            n.flags = n.flags or Notification.FLAG_NO_CLEAR
            return n
        }
    }
}
