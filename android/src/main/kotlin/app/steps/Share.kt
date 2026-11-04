package app.steps

import android.app.Activity
import android.content.Context
import android.content.Intent
import android.os.Bundle
import android.util.Log
import org.json.JSONObject
import java.io.File

/**
 * Share-to-Steps: text shared from any app, or picked with "Steps" in the
 * text-selection menu, becomes a draft of Mohamed's own card. This invisible
 * activity stores it as [PendingShare] JSON (`{text, subject, app}`), brings
 * Steps to the front and closes; Rust picks it up (`own::share`).
 */
// Adapted from batna-app main: android/src/main/kotlin/…/ShareActivity.kt and PendingShare.kt
class ShareActivity : Activity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val received = intent
        // "android-app://com.android.chrome" -> "com.android.chrome"; null when Android doesn't say.
        val app = referrer?.takeIf { it.scheme == "android-app" }?.host
        val text =
            when (received?.action) {
                Intent.ACTION_PROCESS_TEXT -> {
                    // We only read the selection; the other app keeps its text unchanged.
                    setResult(RESULT_CANCELED)
                    received.getCharSequenceExtra(Intent.EXTRA_PROCESS_TEXT)?.toString()
                }
                Intent.ACTION_SEND -> received.getCharSequenceExtra(Intent.EXTRA_TEXT)?.toString()
                else -> null
            }
        if (!text.isNullOrBlank()) {
            try {
                val json =
                    JSONObject()
                        .put("text", text.take(MAX_CHARS))
                        .put("subject", received?.getStringExtra(Intent.EXTRA_SUBJECT)?.take(MAX_SUBJECT))
                        .put("app", app)
                PendingShare.put(applicationContext, json.toString())
            } catch (e: Exception) {
                Log.w(TAG, "Could not keep the shared text", e)
            }
        }
        openSteps()
        finish()
    }

    /** Same intent the launcher uses: resumes the existing task instead of restarting the app. */
    private fun openSteps() {
        val launch = packageManager.getLaunchIntentForPackage(packageName) ?: return
        launch.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_RESET_TASK_IF_NEEDED)
        startActivity(launch)
    }

    private companion object {
        const val TAG = "StepsShare"
        // A card's back holds 3,000 characters; a little more lets Rust trim at a word.
        const val MAX_CHARS = 4000
        // The card's "from" line holds 200 (`own::MAX_SOURCE`); Rust trims the rest.
        const val MAX_SUBJECT = 400
    }
}

/** The one shared item waiting for Rust, kept in a file so a cold start still sees it. */
object PendingShare {
    private const val FILE_NAME = "pending-share.json"

    private fun file(context: Context) = File(context.noBackupFilesDir, FILE_NAME)

    @JvmStatic
    @Synchronized
    fun put(context: Context, json: String) {
        val target = file(context)
        val tmp = File(target.path + ".tmp")
        tmp.writeText(json)
        if (!tmp.renameTo(target)) {
            target.delete()
            tmp.renameTo(target)
        }
    }

    /** Returns the pending item's JSON and forgets it, or null if nothing is waiting. */
    @JvmStatic
    @Synchronized
    fun take(context: Context): String? {
        val target = file(context)
        if (!target.exists()) return null
        val json = target.readText()
        target.delete()
        return json.ifBlank { null }
    }
}
