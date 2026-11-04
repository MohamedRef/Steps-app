package app.steps

import android.Manifest
import android.app.Activity
import android.app.AlarmManager
import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.net.Uri
import android.os.Build
import android.os.PowerManager
import android.provider.Settings
import org.json.JSONArray
import org.json.JSONObject
import java.time.LocalDate
import java.time.LocalTime
import java.time.ZoneId

/**
 * Steps's daily study reminders, adapted from batna-app main:
 * android/src/main/kotlin/…/Reminders.kt. Rust plans the next few
 * (`crates/app/src/reminders/plan.rs`) and hands them over as JSON; each
 * becomes an inexact [AlarmManager] alarm (no exact-alarm permission) that
 * [ReminderReceiver] posts. Offline, no server.
 *
 * The schedule is replaced, never added to, every time the app re-plans, so
 * a met target silences the rest of the day once the app has seen it. It is
 * kept in SharedPreferences to survive reboots and updates. Alarms are armed
 * from each reminder's local date and time in the device's zone, and re-armed
 * when the zone or the clock changes, so Egypt's daylight saving never moves
 * 19:00.
 */
object ReminderBridge {
    const val CHANNEL = "study"
    private const val PREFS = "steps_reminders"
    private const val KEY = "schedule"
    private const val REPEAT = "repeat"
    /** Alarms are request codes 0 until this; Rust never sends more. */
    const val MAX = 32
    /** Days the fallback plans ahead when the app hasn't re-planned. */
    private const val FALLBACK_DAYS = 7L
    private const val PERMISSION_REQUEST = 4120

    /**
     * `[{"at": unix ms, "date": "YYYY-MM-DD", "time": "HH:MM", "title", "text"}]`,
     * soonest first. `[]` clears them (another device rings, or reminders off).
     */
    @JvmStatic
    fun schedule(context: Context, json: String) {
        val app = context.applicationContext
        app.getSharedPreferences(PREFS, Context.MODE_PRIVATE).edit().putString(KEY, json).apply()
        arm(app, json)
    }

    /**
     * The fallback for when Steps isn't opened for days:
     * `{"times": ["HH:MM"], "days": "1111110" (Saturday first), "title", "text",
     * "channel", "channel_about"}` (the channel's name and description, in
     * the app's language, since every UI string comes from Rust).
     * When the last armed reminder fires, or nothing armed is left after a
     * reboot, the next [FALLBACK_DAYS] days are armed from it with its
     * general text (the due counts would be stale). `{}` turns it off.
     */
    @JvmStatic
    fun repeat(context: Context, json: String) {
        context.applicationContext.getSharedPreferences(PREFS, Context.MODE_PRIVATE)
            .edit().putString(REPEAT, json).apply()
    }

    /** Re-arms the stored schedule (after a reboot, an update or a clock change). */
    fun rearm(context: Context) {
        val json = context.getSharedPreferences(PREFS, Context.MODE_PRIVATE).getString(KEY, null) ?: "[]"
        if (arm(context, json) == 0) extend(context)
    }

    /** Arms the next days from the fallback, so reminders never run dry. */
    fun extend(context: Context) {
        val prefs = context.getSharedPreferences(PREFS, Context.MODE_PRIVATE)
        val spec = try {
            JSONObject(prefs.getString(REPEAT, null) ?: return)
        } catch (e: org.json.JSONException) {
            return
        }
        val times = spec.optJSONArray("times") ?: return
        val days = spec.optString("days")
        if (times.length() == 0 || days.length != 7) return
        val now = java.time.LocalDateTime.now()
        val items = JSONArray()
        var day = now.toLocalDate()
        while (day <= now.toLocalDate().plusDays(FALLBACK_DAYS) && items.length() < MAX) {
            // DayOfWeek: Monday 1 … Sunday 7; Steps's week starts on Saturday.
            if (days[(day.dayOfWeek.value + 1) % 7] == '1') {
                for (t in 0 until times.length()) {
                    val time = try { LocalTime.parse(times.getString(t)) } catch (e: Exception) { continue }
                    if (!day.atTime(time).isAfter(now)) continue
                    items.put(
                        JSONObject()
                            .put("at", 0)
                            .put("date", day.toString())
                            .put("time", times.getString(t))
                            .put("title", spec.optString("title"))
                            .put("text", spec.optString("text")),
                    )
                }
            }
            day = day.plusDays(1)
        }
        val json = items.toString()
        prefs.edit().putString(KEY, json).apply()
        arm(context, json)
    }

    /** Arms `json`'s reminders still ahead; returns how many. */
    private fun arm(context: Context, json: String): Int {
        val alarms = context.getSystemService(Context.ALARM_SERVICE) as AlarmManager
        for (i in 0 until MAX) {
            alarms.cancel(pending(context, i, null, null))
        }
        val items = try { JSONArray(json) } catch (e: org.json.JSONException) { JSONArray() }
        val now = System.currentTimeMillis()
        val ahead = (0 until minOf(items.length(), MAX)).mapNotNull { i ->
            val item = items.getJSONObject(i)
            val at = wallClock(item.optString("date"), item.optString("time")) ?: item.getLong("at")
            if (at > now) Pair(i, at) else null
        }
        for ((n, entry) in ahead.withIndex()) {
            val (i, at) = entry
            val item = items.getJSONObject(i)
            val intent = pending(context, i, item.getString("title"), item.getString("text"), n == ahead.size - 1)
            // Inexact while idle: Doze may hold it a few minutes, which suits a nudge.
            alarms.setAndAllowWhileIdle(AlarmManager.RTC_WAKEUP, at, intent)
        }
        return ahead.size
    }

    /** The local date and time in the device's zone now, as unix ms. */
    private fun wallClock(date: String, time: String): Long? = try {
        LocalDate.parse(date).atTime(LocalTime.parse(time))
            .atZone(ZoneId.systemDefault()).toInstant().toEpochMilli()
    } catch (e: Exception) {
        null
    }

    private fun pending(
        context: Context,
        code: Int,
        title: String?,
        text: String?,
        last: Boolean = false,
    ): PendingIntent {
        val intent = Intent(context, ReminderReceiver::class.java).setAction("app.steps.REMIND")
        title?.let { intent.putExtra("title", it) }
        text?.let { intent.putExtra("text", it) }
        intent.putExtra("last", last)
        return PendingIntent.getBroadcast(
            context, code, intent, PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
        )
    }

    /** Whether reminders can show: the app's notifications are on. */
    @JvmStatic
    fun enabled(context: Context): Boolean =
        (context.getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager).areNotificationsEnabled()

    /** Shows the system prompt for notifications on Android 13+, if not granted yet. */
    @JvmStatic
    fun requestPermission(activity: Activity) {
        if (Build.VERSION.SDK_INT >= 33 &&
            activity.checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS) != PackageManager.PERMISSION_GRANTED
        ) {
            activity.requestPermissions(arrayOf(Manifest.permission.POST_NOTIFICATIONS), PERMISSION_REQUEST)
        }
    }

    /** Opens Steps's notification page in Android's settings (after a "don't ask again"). */
    @JvmStatic
    fun openNotificationSettings(activity: Activity) {
        val intent = Intent(Settings.ACTION_APP_NOTIFICATION_SETTINGS)
            .putExtra(Settings.EXTRA_APP_PACKAGE, activity.packageName)
        try {
            activity.startActivity(intent)
        } catch (e: Exception) {
            openAppSettings(activity)
        }
    }

    /** Opens Steps's app page (battery: "Unrestricted" keeps One UI from sleeping its alarms). */
    @JvmStatic
    fun openAppSettings(activity: Activity) {
        activity.startActivity(
            Intent(Settings.ACTION_APPLICATION_DETAILS_SETTINGS, Uri.fromParts("package", activity.packageName, null)),
        )
    }

    /**
     * Whether Android lets Steps run its alarms freely. Samsung's One UI puts
     * apps unused for a few days to sleep, and their alarms stop.
     */
    @JvmStatic
    fun batteryUnrestricted(context: Context): Boolean {
        val pm = context.getSystemService(Context.POWER_SERVICE) as PowerManager
        return pm.isIgnoringBatteryOptimizations(context.packageName)
    }

    fun show(context: Context, title: String, text: String) {
        val nm = context.getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
        val names = context.getSharedPreferences(PREFS, Context.MODE_PRIVATE).getString(REPEAT, null)
            ?.let { runCatching { JSONObject(it) }.getOrNull() }
        nm.createNotificationChannel(
            NotificationChannel(
                CHANNEL,
                names?.optString("channel")?.ifBlank { null } ?: "Steps",
                NotificationManager.IMPORTANCE_DEFAULT,
            ).apply {
                names?.optString("channel_about")?.ifBlank { null }?.let { description = it }
            },
        )
        val open = context.packageManager.getLaunchIntentForPackage(context.packageName)
            ?.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_RESET_TASK_IF_NEEDED)
        val tap = open?.let {
            PendingIntent.getActivity(context, 0, it, PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE)
        }
        val note = Notification.Builder(context, CHANNEL)
            .setSmallIcon(app.steps.bridge.R.drawable.ic_stat_steps)
            .setContentTitle(title)
            .setContentText(text)
            .setStyle(Notification.BigTextStyle().bigText(text))
            .setContentIntent(tap)
            .setAutoCancel(true)
            .build()
        // One reminder at a time: a newer one replaces an unread older one.
        nm.notify(CHANNEL.hashCode(), note)
    }
}

/** Posts a scheduled reminder, and re-arms the schedule after a reboot, an update or a clock change. */
class ReminderReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        when (intent.action) {
            Intent.ACTION_BOOT_COMPLETED, Intent.ACTION_MY_PACKAGE_REPLACED,
            Intent.ACTION_TIMEZONE_CHANGED, Intent.ACTION_TIME_CHANGED -> ReminderBridge.rearm(context)
            else -> {
                val title = intent.getStringExtra("title") ?: return
                val text = intent.getStringExtra("text") ?: return
                if (ReminderBridge.enabled(context)) ReminderBridge.show(context, title, text)
                // The app hasn't re-planned since: keep the next days covered.
                if (intent.getBooleanExtra("last", false)) ReminderBridge.extend(context)
            }
        }
    }
}
