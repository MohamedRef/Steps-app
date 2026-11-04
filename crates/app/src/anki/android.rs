//! Android side of «export to Anki», through JNI from Rust using the JavaVM
//! and Activity that tao/wry register in `ndk_context` (adapted from
//! batna-app main: crates/app/src/platform/android.rs).
//!
//! 1. MediaStore puts the file in `Download/Steps/` (Android 10+, no storage
//!    permission). Our earlier copy is deleted first, so the name stays
//!    `steps.apkg` instead of becoming `steps (1).apkg`.
//! 2. An `ACTION_VIEW` intent with a read grant on that `content://` URI opens
//!    it; AnkiDroid shows its import screen. No FileProvider is needed,
//!    because MediaStore URIs can be granted directly.

#![allow(unsafe_code)]

use jni::objects::{JObject, JString};
use jni::{Env, JValue, JavaVM, jni_sig, jni_str};

/// Where the file lands, for the message under the button.
const RELATIVE_PATH: &str = "Download/Steps/";
/// The types AnkiDroid's import filter accepts, best first.
const VIEW_TYPES: &[&str] = &["application/apkg", "application/octet-stream", "*/*"];
/// `Intent.FLAG_GRANT_READ_URI_PERMISSION | FLAG_ACTIVITY_NEW_TASK`.
const VIEW_FLAGS: i32 = 0x0000_0001 | 0x1000_0000;

fn with_activity<T>(f: impl FnOnce(&mut Env, &JObject) -> jni::errors::Result<T>) -> Result<T, String> {
    let ctx = ndk_context::android_context();
    // SAFETY: tao initializes ndk_context with the process JavaVM and a global
    // reference to the Activity before any app code runs; both live for the
    // whole process.
    let vm = unsafe { JavaVM::from_raw(ctx.vm().cast()) };
    vm.attach_current_thread(|env| {
        // SAFETY: see above; the Activity is a valid global reference.
        let activity = unsafe { JObject::from_raw(env, ctx.context().cast()) };
        f(env, &activity)
    })
    .map_err(java_error)
}

/// A readable message; Java exceptions arrive already caught and cleared.
fn java_error(e: jni::errors::Error) -> String {
    match e {
        jni::errors::Error::CaughtJavaException { name, msg, .. } => format!("{name}: {msg}"),
        other => other.to_string(),
    }
}

/// Saves `bytes` as `Download/Steps/<name>` and opens it in Anki. Returns
/// where it was saved, even when no app could open it.
pub fn save_and_open(name: &str, bytes: &[u8]) -> Result<String, String> {
    let sdk = with_activity(|env, _| {
        env.get_static_field(jni_str!("android/os/Build$VERSION"), jni_str!("SDK_INT"), jni_sig!("I"))?.i()
    })?;
    if sdk < 29 {
        return Err(format!("Android {sdk} is older than Android 10"));
    }
    let opened = with_activity(|env, activity| {
        let resolver = env
            .call_method(
                activity,
                jni_str!("getContentResolver"),
                jni_sig!("()Landroid/content/ContentResolver;"),
                &[],
            )?
            .l()?;
        let collection = env
            .get_static_field(
                jni_str!("android/provider/MediaStore$Downloads"),
                jni_str!("EXTERNAL_CONTENT_URI"),
                jni_sig!("Landroid/net/Uri;"),
            )?
            .l()?;

        // Drop our previous copy. Rows other apps own are not touched.
        let selection = env.new_string("relative_path=? AND _display_name=?")?;
        let args = env.new_object_array(2, jni_str!("java/lang/String"), JObject::null())?;
        let path_arg = env.new_string(RELATIVE_PATH)?;
        let name_arg = env.new_string(name)?;
        args.set_element(env, 0, &path_arg)?;
        args.set_element(env, 1, &name_arg)?;
        env.call_method(
            &resolver,
            jni_str!("delete"),
            jni_sig!("(Landroid/net/Uri;Ljava/lang/String;[Ljava/lang/String;)I"),
            &[JValue::Object(&collection), JValue::Object(&selection), JValue::Object(&args)],
        )?;

        // Insert the new row as pending, write the bytes, then publish it.
        let values = env.new_object(jni_str!("android/content/ContentValues"), jni_sig!("()V"), &[])?;
        put_string(env, &values, "_display_name", name)?;
        put_string(env, &values, "mime_type", "application/octet-stream")?;
        put_string(env, &values, "relative_path", RELATIVE_PATH)?;
        put_int(env, &values, "is_pending", 1)?;
        let uri = env
            .call_method(
                &resolver,
                jni_str!("insert"),
                jni_sig!("(Landroid/net/Uri;Landroid/content/ContentValues;)Landroid/net/Uri;"),
                &[JValue::Object(&collection), JValue::Object(&values)],
            )?
            .l()?;
        if uri.is_null() {
            return Err(jni::errors::Error::NullPtr("MediaStore refused the new file"));
        }
        let mode = env.new_string("w")?;
        let out = env
            .call_method(
                &resolver,
                jni_str!("openOutputStream"),
                jni_sig!("(Landroid/net/Uri;Ljava/lang/String;)Ljava/io/OutputStream;"),
                &[JValue::Object(&uri), JValue::Object(&mode)],
            )?
            .l()?;
        let data = env.byte_array_from_slice(bytes)?;
        env.call_method(&out, jni_str!("write"), jni_sig!("([B)V"), &[JValue::Object(&data)])?;
        env.call_method(&out, jni_str!("close"), jni_sig!("()V"), &[])?;
        let done = env.new_object(jni_str!("android/content/ContentValues"), jni_sig!("()V"), &[])?;
        put_int(env, &done, "is_pending", 0)?;
        env.call_method(
            &resolver,
            jni_str!("update"),
            jni_sig!(
                "(Landroid/net/Uri;Landroid/content/ContentValues;Ljava/lang/String;[Ljava/lang/String;)I"
            ),
            &[
                JValue::Object(&uri),
                JValue::Object(&done),
                JValue::Object(&JObject::null()),
                JValue::Object(&JObject::null()),
            ],
        )?;

        // Open it: the first type some app (AnkiDroid) accepts wins.
        let action = env.new_string("android.intent.action.VIEW")?;
        for ty in VIEW_TYPES {
            let intent = env.new_object(
                jni_str!("android/content/Intent"),
                jni_sig!("(Ljava/lang/String;)V"),
                &[JValue::Object(&action)],
            )?;
            let ty = env.new_string(ty)?;
            env.call_method(
                &intent,
                jni_str!("setDataAndType"),
                jni_sig!("(Landroid/net/Uri;Ljava/lang/String;)Landroid/content/Intent;"),
                &[JValue::Object(&uri), JValue::Object(&ty)],
            )?;
            env.call_method(
                &intent,
                jni_str!("addFlags"),
                jni_sig!("(I)Landroid/content/Intent;"),
                &[JValue::Int(VIEW_FLAGS)],
            )?;
            match env.call_method(
                activity,
                jni_str!("startActivity"),
                jni_sig!("(Landroid/content/Intent;)V"),
                &[JValue::Object(&intent)],
            ) {
                Ok(_) => return Ok(true),
                // ActivityNotFoundException: no app for this type, try the next.
                Err(jni::errors::Error::CaughtJavaException { .. }) => continue,
                Err(e) => return Err(e),
            }
        }
        Ok(false)
    })?;
    let place = format!("{RELATIVE_PATH}{name}");
    if opened { Ok(place) } else { Err(format!("saved in {place}, but no app opened it: install AnkiDroid")) }
}

fn put_string(env: &mut Env, values: &JObject, key: &str, value: &str) -> jni::errors::Result<()> {
    let key: JString = env.new_string(key)?;
    let value: JString = env.new_string(value)?;
    env.call_method(
        values,
        jni_str!("put"),
        jni_sig!("(Ljava/lang/String;Ljava/lang/String;)V"),
        &[JValue::Object(&key), JValue::Object(&value)],
    )?;
    Ok(())
}

fn put_int(env: &mut Env, values: &JObject, key: &str, value: i32) -> jni::errors::Result<()> {
    let key: JString = env.new_string(key)?;
    let boxed = env
        .call_static_method(
            jni_str!("java/lang/Integer"),
            jni_str!("valueOf"),
            jni_sig!("(I)Ljava/lang/Integer;"),
            &[JValue::Int(value)],
        )?
        .l()?;
    env.call_method(
        values,
        jni_str!("put"),
        jni_sig!("(Ljava/lang/String;Ljava/lang/Integer;)V"),
        &[JValue::Object(&key), JValue::Object(&boxed)],
    )?;
    Ok(())
}
