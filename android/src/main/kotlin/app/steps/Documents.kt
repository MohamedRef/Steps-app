package app.steps

import android.app.Activity
import android.content.Intent
import android.net.Uri
import android.os.Bundle
import android.provider.OpenableColumns
import android.util.Log
import java.io.File
import kotlin.concurrent.thread

/**
 * Settings › Your data on Android: the system "Save to" dialog for an export,
 * and the system file picker for a restore (Storage Access Framework:
 * Downloads, Drive…, no permission needed).
 *
 * Save: the export is first written to the app's own folder, which the Files
 * app can't open; this invisible activity asks where to put it and copies it
 * there. Open: the picked file is copied into the app's cache, and Rust reads
 * that copy. Rust starts it through [Documents.save] or [Documents.open] and
 * polls [Documents.state].
 */
// Adapted from kabar-app main: android/src/main/kotlin/app/kabar/ExportSaveActivity.kt
class DocumentActivity : Activity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        // Recreated after a configuration change: the dialog is already showing.
        if (savedInstanceState != null) return
        val request =
            if (intent.getStringExtra(EXTRA_PATH) != null) {
                // A YAML file as octet-stream, so no provider adds ".json".
                val name = intent.getStringExtra(EXTRA_NAME) ?: ""
                Intent(Intent.ACTION_CREATE_DOCUMENT)
                    .addCategory(Intent.CATEGORY_OPENABLE)
                    .setType(if (name.endsWith(".json")) "application/json" else "application/octet-stream")
                    .putExtra(Intent.EXTRA_TITLE, intent.getStringExtra(EXTRA_NAME) ?: "steps-export.json")
            } else {
                // Some providers label JSON as octet-stream or plain text.
                Intent(Intent.ACTION_OPEN_DOCUMENT)
                    .addCategory(Intent.CATEGORY_OPENABLE)
                    .setType("*/*")
                    .putExtra(Intent.EXTRA_MIME_TYPES, arrayOf("application/json", "application/octet-stream", "text/plain"))
            }
        try {
            @Suppress("DEPRECATION")
            startActivityForResult(request, REQUEST)
        } catch (e: Exception) {
            Log.w(TAG, "No document UI", e)
            Documents.done(Documents.FAILED, null, null)
            finish()
        }
    }

    // The androidx result API needs a ComponentActivity; this module stays framework-only.
    @Suppress("OVERRIDE_DEPRECATION", "DEPRECATION")
    override fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?) {
        super.onActivityResult(requestCode, resultCode, data)
        val uri = data?.data
        if (requestCode != REQUEST || resultCode != RESULT_OK || uri == null) {
            Documents.done(Documents.CANCELLED, null, null)
            finish()
            return
        }
        val source = intent.getStringExtra(EXTRA_PATH)
        val resolver = contentResolver
        val cache = cacheDir
        thread(name = "steps-documents") {
            try {
                val name = displayName(uri) ?: ""
                if (source != null) {
                    resolver.openOutputStream(uri, "wt").use { out ->
                        requireNotNull(out) { "no output stream" }
                        File(source).inputStream().use { it.copyTo(out) }
                    }
                    Documents.done(Documents.SAVED, name, null)
                } else {
                    val copy = File(cache, "restore.json")
                    resolver.openInputStream(uri).use { input ->
                        requireNotNull(input) { "no input stream" }
                        copy.outputStream().use { out -> copyCapped(input, out) }
                    }
                    Documents.done(Documents.OPENED, name, copy.absolutePath)
                }
            } catch (e: Exception) {
                Log.w(TAG, "Could not copy the document", e)
                if (source == null) File(cache, "restore.json").delete()
                Documents.done(Documents.FAILED, null, null)
            }
            runOnUiThread { finish() }
        }
    }

    override fun onDestroy() {
        super.onDestroy()
        // Closed without a result (e.g. the system killed the dialog).
        if (isFinishing && Documents.state() == Documents.BUSY) Documents.done(Documents.CANCELLED, null, null)
    }

    /** A backup is a few MB; stop a wrong pick (a video, a big PDF) early. */
    private fun copyCapped(input: java.io.InputStream, out: java.io.OutputStream) {
        val buffer = ByteArray(64 * 1024)
        var total = 0L
        while (true) {
            val n = input.read(buffer)
            if (n < 0) break
            total += n
            if (total > MAX_RESTORE_BYTES) throw java.io.IOException("larger than a Steps backup")
            out.write(buffer, 0, n)
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

    companion object {
        private const val TAG = "StepsDocuments"
        private const val REQUEST = 3
        // A real backup is a few MB.
        private const val MAX_RESTORE_BYTES = 50L * 1024 * 1024
        const val EXTRA_PATH = "app.steps.documents.PATH"
        const val EXTRA_NAME = "app.steps.documents.NAME"
    }
}

/** Dialog state shared with Rust over JNI. */
object Documents {
    const val IDLE = 0
    const val BUSY = 1
    const val FAILED = 2
    const val SAVED = 3
    const val CANCELLED = 4
    const val OPENED = 5

    @Volatile private var current = IDLE
    @Volatile private var chosenName: String? = null
    @Volatile private var copyPath: String? = null

    /** Shows the "Save to" dialog for the file at [path], suggesting [name]. */
    @JvmStatic
    fun save(activity: Activity, path: String, name: String) = start(activity, path, name)

    /** Shows the file picker; the picked file is copied into the cache. */
    @JvmStatic
    fun open(activity: Activity) = start(activity, null, null)

    private fun start(activity: Activity, path: String?, name: String?) {
        current = BUSY
        chosenName = null
        copyPath = null
        activity.runOnUiThread {
            try {
                val intent = Intent(activity, DocumentActivity::class.java)
                if (path != null) {
                    intent.putExtra(DocumentActivity.EXTRA_PATH, path).putExtra(DocumentActivity.EXTRA_NAME, name)
                }
                activity.startActivity(intent)
            } catch (e: Exception) {
                Log.w("StepsDocuments", "Could not start the document dialog", e)
                current = FAILED
            }
        }
    }

    /** IDLE, BUSY, FAILED, SAVED, CANCELLED or OPENED. */
    @JvmStatic
    fun state(): Int = current

    /** The name the file was saved as, or the picked file's name. */
    @JvmStatic
    fun name(): String? = chosenName

    /** OPENED only: the cache copy of the picked file. */
    @JvmStatic
    fun path(): String? = copyPath

    /** Back to IDLE once Rust has read a finished state. */
    @JvmStatic
    fun reset() {
        if (current != BUSY) current = IDLE
    }

    internal fun done(state: Int, name: String?, path: String?) {
        chosenName = name
        copyPath = path
        current = state
    }
}
