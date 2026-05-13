package `in`.cinny.app

import android.app.AlarmManager
import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.content.SharedPreferences
import android.content.pm.ServiceInfo
import android.net.ConnectivityManager
import android.net.Network
import android.net.NetworkCapabilities
import android.net.NetworkRequest
import android.os.Build
import android.os.IBinder
import android.os.SystemClock
import androidx.core.content.ContextCompat
import androidx.preference.PreferenceManager

/**
 * Foreground service that keeps the Matrix WebSocket alive in the background.
 *
 * Pattern taken from FairEmail's ServiceSynchronize:
 * - Persistent notification on IMPORTANCE_MIN channel (collapsed, no sound/icon)
 * - FLAG_NO_CLEAR prevents accidental dismissal
 * - ConnectivityManager callback detects network changes (WiFi → mobile, etc.)
 * - AlarmManager watchdog wakes the service every 15 min to prevent silent death
 * - User can enable background_service mode to hide the notification entirely
 */
class ForegroundService : Service() {

    companion object {
        const val CHANNEL_ID = "cinny_foreground"
        const val NOTIFICATION_ID = 1
        const val PI_WATCHDOG = 10
        const val WATCHDOG_INTERVAL_MS = 15 * 60 * 1000L // 15 minutes
        var isRunning = false
            private set
    }

    private var connectivityCallback: ConnectivityManager.NetworkCallback? = null
    private var connectionChangedReceiver: ConnectionChangedReceiver? = null

    // Network callback — wakes the service when internet becomes available
    private val networkCallback = object : ConnectivityManager.NetworkCallback() {
        override fun onAvailable(network: Network) {
            startWithForeground(this@ForegroundService)
        }

        override fun onCapabilitiesChanged(network: Network, caps: NetworkCapabilities) {
            if (caps.hasCapability(NetworkCapabilities.NET_CAPABILITY_INTERNET)) {
                startWithForeground(this@ForegroundService)
            }
        }
    }

    override fun onCreate() {
        super.onCreate()

        if (isBackgroundService(this)) {
            stopForeground(true)
        } else {
            startWithForeground(this)
        }

        // Listen for network changes (FairEmail pattern)
        val cm = getSystemService(ConnectivityManager::class.java)
        val builder = NetworkRequest.Builder()
            .addCapability(NetworkCapabilities.NET_CAPABILITY_INTERNET)
        connectivityCallback = networkCallback
        cm.registerNetworkCallback(builder.build(), networkCallback)

        // Broadcast receiver for connectivity/airplane mode changes
        connectionChangedReceiver = ConnectionChangedReceiver()
        val iif = IntentFilter().apply {
            addAction(ConnectivityManager.CONNECTIVITY_ACTION)
            addAction(Intent.ACTION_AIRPLANE_MODE_CHANGED)
        }
        ContextCompat.registerReceiver(
            this, connectionChangedReceiver, iif,
            ContextCompat.RECEIVER_NOT_EXPORTED
        )

        // Schedule watchdog alarm
        scheduleWatchdog(this)
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        if (isBackgroundService(this)) {
            stopForeground(true)
        } else {
            startWithForeground(this)
        }
        isRunning = true
        return START_STICKY
    }

    override fun onBind(intent: Intent?): IBinder? = null

    override fun onDestroy() {
        isRunning = false

        connectivityCallback?.let {
            val cm = getSystemService(ConnectivityManager::class.java)
            cm.unregisterNetworkCallback(it)
        }
        connectionChangedReceiver?.let {
            try { unregisterReceiver(it) } catch (_: Exception) {}
        }

        // Reschedule watchdog before dying so we come back
        if (!isBackgroundService(this)) {
            scheduleWatchdog(this)
        }

        super.onDestroy()
    }

    private fun createNotificationChannel() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            val channel = NotificationChannel(
                CHANNEL_ID,
                "Background service",
                NotificationManager.IMPORTANCE_MIN // FairEmail: collapsed, no sound
            ).apply {
                description = "Shown when Cinny keeps the Matrix connection alive. " +
                    "Disable this channel to hide the notification; the service will continue running."
                setShowBadge(false)
            }
            val manager = getSystemService(NotificationManager::class.java)
            manager.createNotificationChannel(channel)
        }
    }

    private fun buildNotification(): Notification {
        createNotificationChannel()

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

    /**
     * Broadcast receiver that restarts the foreground notification when
     * connectivity or airplane mode changes.
     */
    inner class ConnectionChangedReceiver : android.content.BroadcastReceiver() {
        override fun onReceive(context: Context, intent: Intent) {
            if (!isBackgroundService(this@ForegroundService)) {
                startWithForeground(this@ForegroundService)
            }
        }
    }

    companion object {
        fun startWithForeground(service: ForegroundService) {
            try {
                service.startForeground(
                    NOTIFICATION_ID,
                    service.buildNotification(),
                    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.UPSIDE_DOWN_CAKE)
                        ServiceInfo.FOREGROUND_SERVICE_TYPE_DATA_SYNC
                    else 0
                )
            } catch (_: Exception) {}
        }

        /**
         * Periodic alarm that restarts the service if Android killed it during
         * doze or due to memory pressure. FairEmail uses the same pattern
         * with setAndAllowWhileIdle.
         */
        fun scheduleWatchdog(context: Context) {
            if (isBackgroundService(context)) return

            val am = context.getSystemService(AlarmManager::class.java) ?: return
            val intent = Intent(context, ForegroundService::class.java)
            val pi = PendingIntent.getService(
                context, PI_WATCHDOG, intent,
                PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT
            )

            val trigger = SystemClock.elapsedRealtime() + WATCHDOG_INTERVAL_MS
            try {
                am.setExactAndAllowWhileIdle(AlarmManager.ELAPSED_REALTIME_WAKEUP, trigger, pi)
            } catch (_: Exception) {}
        }

        fun isBackgroundService(context: Context): Boolean {
            if (Build.VERSION.SDK_INT < Build.VERSION_CODES.O) {
                val prefs = PreferenceManager.getDefaultSharedPreferences(context)
                return prefs.getBoolean("background_service", false)
            }
            return false // Android 8+ uses channel settings instead
        }
    }
}
