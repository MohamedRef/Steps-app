package app.steps

import android.app.Activity
import android.content.Context
import android.content.Intent
import android.net.Uri
import android.os.Build
import android.os.Bundle
import android.provider.OpenableColumns
import android.util.Log
import org.json.JSONObject
import java.io.File
import java.io.InputStream
import java.io.OutputStream
import kotlin.concurrent.thread

/**
 * Share-to-Steps: text shared from any app, or picked with "Steps" in the
 * text-selection menu, becomes a draft of Mohamed's own card. This invisible
 * activity stores it as [PendingShare] JSON (`{text, subject, app}`), brings
 * Steps to the front and closes; Rust picks it up (`own::share`).
 *
 * A file shared, or opened with Steps ("Open with": an Excel sheet, a zip,
 * JSON or YAML), is copied into the app and kept as [PendingFile] JSON
 * (`{name, path}`); Rust opens Settings › Your data on it (`inbox`).
 */
// Adapted from batna-app main: android/src/main/kotlin/…/ShareActivity.kt and PendingShare.kt
class ShareActivity : Activity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val received = intent
        val file = sharedFile(received)
        if (file != null) {
            // A file can be big or come from Drive: copy it off the UI thread,
            // while this invisible activity (and its read grant) is alive.
            thread(name = "steps-share-file") {
                keepFile(file)
                runOnUiThread {
                    openSteps()
                    finish()
                }
            }
            return
        }
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

    /** The file shared ("Share") or opened ("Open with"), if this is one. */
    private fun sharedFile(received: Intent?): Uri? =
        when (received?.action) {
            Intent.ACTION_VIEW -> received.data
            Intent.ACTION_SEND -> stream(received)
            else -> null
        }

    private fun stream(received: Intent): Uri? =
        if (Build.VERSION.SDK_INT >= 33) {
            received.getParcelableExtra(Intent.EXTRA_STREAM, Uri::class.java)
        } else {
            @Suppress("DEPRECATION")
            received.getParcelableExtra(Intent.EXTRA_STREAM)
        }

    /** Copies the file in and leaves its name for Rust; an empty path if it couldn't be read. */
    private fun keepFile(uri: Uri) {
        val name = displayName(uri) ?: uri.lastPathSegment?.substringAfterLast('/') ?: ""
        val copy = PendingFile.copy(applicationContext)
        val path =
            try {
                contentResolver.openInputStream(uri).use { input ->
                    requireNotNull(input) { "no input stream" }
                    copy.outputStream().use { out -> copyCapped(input, out) }
                }
                copy.absolutePath
            } catch (e: Exception) {
                Log.w(TAG, "Could not copy the shared file", e)
                copy.delete()
                ""
            }
        try {
            PendingFile.put(applicationContext, JSONObject().put("name", name).put("path", path).toString())
        } catch (e: Exception) {
            Log.w(TAG, "Could not keep the shared file", e)
        }
    }

    private fun displayName(uri: Uri): String? =
        try {
            contentResolver.query(uri, arrayOf(OpenableColumns.DISPLAY_NAME), null, null, null)?.use {
                if (it.moveToFirst()) it.getString(0) else null
            }
        } catch (e: Exception) {
            null
        }

    /** A file of questions or a backup is a few MB; stop a wrong one (a video) early. */
    private fun copyCapped(input: InputStream, out: OutputStream) {
        val buffer = ByteArray(64 * 1024)
        var total = 0L
        while (true) {
            val n = input.read(buffer)
            if (n < 0) break
            total += n
            if (total > MAX_FILE_BYTES) throw java.io.IOException("larger than a Steps file")
            out.write(buffer, 0, n)
        }
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
        // The same cap as a picked file (Documents.kt).
        const val MAX_FILE_BYTES = 50L * 1024 * 1024
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

/**
 * The one shared file waiting for Rust: its copy in the app's own folder, and
 * `{name, path}` beside it, in files so a cold start still sees them.
 */
object PendingFile {
    private const val FILE_NAME = "pending-file.json"
    private const val COPY_NAME = "shared-file"

    private fun file(context: Context) = File(context.noBackupFilesDir, FILE_NAME)

    /** Where the shared file is copied; Rust deletes it once read. */
    fun copy(context: Context) = File(context.noBackupFilesDir, COPY_NAME)

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

    /** Returns the waiting file's JSON and forgets it, or null if nothing is waiting. */
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
