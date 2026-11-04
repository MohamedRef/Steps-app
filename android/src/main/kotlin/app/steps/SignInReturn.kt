package app.steps

import android.app.Activity
import android.content.Intent
import android.os.Bundle

/**
 * "ارجع لـ Steps" on the Google sign-in page in Chrome (app.steps://signed-in).
 * Invisible: it brings the running Steps to the front and finishes. Android
 * keeps a backgrounded app off the network, so the sign-in Google just
 * approved finishes once Steps is back in front (crates/app/src/cloud/engine.rs).
 * The link carries nothing: the code already reached the app's loopback listener.
 */
class SignInReturnActivity : Activity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        packageManager.getLaunchIntentForPackage(packageName)
            ?.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_RESET_TASK_IF_NEEDED)
            ?.let { startActivity(it) }
        finish()
    }
}
