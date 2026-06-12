//! Android clipboard implementation via JNI + ClipboardManager.

use crate::{ClipboardOptions, Error};
use jni::JNIEnv;
use jni::objects::{JObject, JString, JValue};
use jni::sys::jobject;

pub(crate) struct ClipboardImpl;

impl ClipboardImpl {
    pub(crate) fn new(_opts: &ClipboardOptions) -> Result<Self, Error> {
        Ok(Self)
    }

    pub(crate) async fn mime_types(&self) -> Result<Vec<String>, Error> {
        tokio::task::block_in_place(|| with_jni_env(get_mime_types_jni))
    }

    pub(crate) async fn read(&self, mime_type: &str) -> Result<Vec<u8>, Error> {
        match mime_type {
            "text/plain" => {
                let s = tokio::task::block_in_place(|| with_jni_env(get_text_jni))?;
                Ok(s.into_bytes())
            }
            _ => Err(Error::NotSupported),
        }
    }

    pub(crate) async fn write(&self, mime_type: &str, data: &[u8]) -> Result<(), Error> {
        match mime_type {
            "text/plain" => {
                let s =
                    std::str::from_utf8(data).map_err(|e| Error::platform("android: utf-8", e))?;
                let s = s.to_owned();
                tokio::task::block_in_place(move || {
                    with_jni_env(|env, ctx| set_text_jni(env, ctx, &s))
                })
            }
            _ => Err(Error::NotSupported),
        }
    }
}

fn get_mime_types_jni<'local, 'ctx>(
    env: &mut JNIEnv<'local>,
    context: &JObject<'ctx>,
) -> Result<Vec<String>, Error> {
    let manager = get_clipboard_manager(env, context)?;

    let clip = env
        .call_method(
            manager,
            "getPrimaryClip",
            "()Landroid/content/ClipData;",
            &[],
        )
        .map_err(|e| Error::platform("android: getPrimaryClip", e))?
        .l()
        .map_err(|e| Error::platform("android: getPrimaryClip result", e))?;

    if clip.is_null() {
        return Ok(Vec::new());
    }

    let description = env
        .call_method(
            &clip,
            "getDescription",
            "()Landroid/content/ClipDescription;",
            &[],
        )
        .map_err(|e| Error::platform("android: getDescription", e))?
        .l()
        .map_err(|e| Error::platform("android: getDescription result", e))?;

    if description.is_null() {
        return Ok(Vec::new());
    }

    let count = env
        .call_method(&description, "getMimeTypeCount", "()I", &[])
        .map_err(|e| Error::platform("android: getMimeTypeCount", e))?
        .i()
        .map_err(|e| Error::platform("android: getMimeTypeCount result", e))?;

    let mut mimes = Vec::new();
    for i in 0..count {
        let mime = env
            .call_method(
                &description,
                "getMimeType",
                "(I)Ljava/lang/String;",
                &[JValue::Int(i)],
            )
            .map_err(|e| Error::platform("android: getMimeType", e))?
            .l()
            .map_err(|e| Error::platform("android: getMimeType result", e))?;

        if mime.is_null() {
            continue;
        }

        let jstr = JString::from(mime);
        if let Ok(s) = env.get_string(&jstr) {
            mimes.push(s.to_string_lossy().into_owned());
        }
    }

    Ok(mimes)
}

fn with_jni_env<T, F>(f: F) -> Result<T, Error>
where
    F: FnOnce(&mut JNIEnv<'_>, &JObject<'_>) -> Result<T, Error>,
{
    let android_ctx = ndk_context::android_context();

    let vm = unsafe { jni::JavaVM::from_raw(android_ctx.vm().cast()) }
        .map_err(|e| Error::platform("android: JavaVM::from_raw", e))?;

    let mut env = vm
        .attach_current_thread()
        .map_err(|e| Error::platform("android: attach_current_thread", e))?;

    // SAFETY: ndk_context provides a live Context pointer; we must not delete it.
    let context =
        std::mem::ManuallyDrop::new(unsafe { JObject::from_raw(android_ctx.context() as jobject) });

    let result = f(&mut env, &context);

    result
}

fn get_clipboard_manager<'local, 'ctx>(
    env: &mut JNIEnv<'local>,
    context: &JObject<'ctx>,
) -> Result<JObject<'local>, Error> {
    let context_class = env
        .find_class("android/content/Context")
        .map_err(|e| Error::platform("android: find_class(Context)", e))?;

    let service_field = env
        .get_static_field(context_class, "CLIPBOARD_SERVICE", "Ljava/lang/String;")
        .map_err(|e| Error::platform("android: get CLIPBOARD_SERVICE", e))?;

    let service_name = service_field
        .l()
        .map_err(|e| Error::platform("android: extract CLIPBOARD_SERVICE", e))?;

    let manager = env
        .call_method(
            context,
            "getSystemService",
            "(Ljava/lang/String;)Ljava/lang/Object;",
            &[JValue::Object(&service_name)],
        )
        .map_err(|e| Error::platform("android: getSystemService", e))?
        .l()
        .map_err(|e| Error::platform("android: getSystemService result", e))?;

    if manager.is_null() {
        return Err(Error::NotSupported);
    }

    Ok(manager)
}

fn get_text_jni<'local, 'ctx>(
    env: &mut JNIEnv<'local>,
    context: &JObject<'ctx>,
) -> Result<String, Error> {
    let manager = get_clipboard_manager(env, context)?;

    // ClipboardManager.getPrimaryClip()
    let clip = env
        .call_method(
            manager,
            "getPrimaryClip",
            "()Landroid/content/ClipData;",
            &[],
        )
        .map_err(|e| Error::platform("android: getPrimaryClip", e))?
        .l()
        .map_err(|e| Error::platform("android: getPrimaryClip result", e))?;

    if clip.is_null() {
        return Err(Error::PermissionDenied(
            "getPrimaryClip returned null (app may lack focus)",
        ));
    }

    // Check item count
    let count = env
        .call_method(&clip, "getItemCount", "()I", &[])
        .map_err(|e| Error::platform("android: getItemCount", e))?
        .i()
        .map_err(|e| Error::platform("android: getItemCount result", e))?;

    if count <= 0 {
        return Err(Error::Unavailable("clipboard is empty"));
    }

    // Get first item
    let item = env
        .call_method(
            clip,
            "getItemAt",
            "(I)Landroid/content/ClipData$Item;",
            &[JValue::Int(0)],
        )
        .map_err(|e| Error::platform("android: getItemAt(0)", e))?
        .l()
        .map_err(|e| Error::platform("android: getItemAt result", e))?;

    // coerceToText(Context)
    let char_seq = env
        .call_method(
            item,
            "coerceToText",
            "(Landroid/content/Context;)Ljava/lang/CharSequence;",
            &[JValue::Object(context)],
        )
        .map_err(|e| Error::platform("android: coerceToText", e))?
        .l()
        .map_err(|e| Error::platform("android: coerceToText result", e))?;

    if char_seq.is_null() {
        return Err(Error::Unavailable("clipboard item has no text"));
    }

    // CharSequence.toString()
    let jstring_obj = env
        .call_method(char_seq, "toString", "()Ljava/lang/String;", &[])
        .map_err(|e| Error::platform("android: toString", e))?
        .l()
        .map_err(|e| Error::platform("android: toString result", e))?;

    let jstring = JString::from(jstring_obj);
    let rust_string = env
        .get_string(&jstring)
        .map_err(|e| Error::platform("android: get_string", e))?
        .to_string_lossy()
        .into_owned();

    Ok(rust_string)
}

fn set_text_jni<'local, 'ctx>(
    env: &mut JNIEnv<'local>,
    context: &JObject<'ctx>,
    text: &str,
) -> Result<(), Error> {
    let manager = get_clipboard_manager(env, context)?;

    let clipdata_class = env
        .find_class("android/content/ClipData")
        .map_err(|e| Error::platform("android: find_class(ClipData)", e))?;

    let label = env
        .new_string("clipawl")
        .map_err(|e| Error::platform("android: new_string(label)", e))?;

    let value = env
        .new_string(text)
        .map_err(|e| Error::platform("android: new_string(text)", e))?;

    let label_obj = JObject::from(label);
    let value_obj = JObject::from(value);

    // ClipData.newPlainText(label, text)
    let clipdata = env
        .call_static_method(
            clipdata_class,
            "newPlainText",
            "(Ljava/lang/CharSequence;Ljava/lang/CharSequence;)Landroid/content/ClipData;",
            &[JValue::Object(&label_obj), JValue::Object(&value_obj)],
        )
        .map_err(|e| Error::platform("android: newPlainText", e))?
        .l()
        .map_err(|e| Error::platform("android: newPlainText result", e))?;

    // setPrimaryClip
    env.call_method(
        manager,
        "setPrimaryClip",
        "(Landroid/content/ClipData;)V",
        &[JValue::Object(&clipdata)],
    )
    .map_err(|e| Error::platform("android: setPrimaryClip", e))?;

    Ok(())
}
