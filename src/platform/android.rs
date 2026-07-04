//! Android clipboard implementation via JNI + ClipboardManager.

use crate::{ClipboardOptions, Error};
use jni::errors::Result as JniResult;
use jni::objects::{JObject, JString, JValue};
use jni::sys::{jint, jobject};
use jni::{Env, jni_sig, jni_str};

impl From<jni::errors::Error> for Error {
    fn from(e: jni::errors::Error) -> Self {
        Error::platform("jni", e)
    }
}

pub(crate) struct ClipboardImpl;

impl ClipboardImpl {
    pub(crate) fn new(_opts: &ClipboardOptions) -> Result<Self, Error> {
        Ok(Self)
    }

    pub(crate) async fn mime_types(&self) -> Result<Vec<String>, Error> {
        crate::exec::unblock(|| with_jni_env(get_mime_types_jni)).await
    }

    pub(crate) async fn read(&self, mime_type: &str) -> Result<Vec<u8>, Error> {
        match mime_type {
            "text/plain" => {
                let s = crate::exec::unblock(|| with_jni_env(get_text_jni)).await?;
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
                crate::exec::unblock(move || with_jni_env(|env| set_text_jni(env, &s))).await
            }
            _ => Err(Error::NotSupported),
        }
    }
}

fn with_context<'local, T>(
    env: &mut Env<'local>,
    f: impl FnOnce(&mut Env<'local>, &JObject<'local>) -> Result<T, Error>,
) -> Result<T, Error> {
    let android_ctx = ndk_context::android_context();
    // Convert the raw global ref from ndk-context into a proper JNI local ref
    // so JNI manages its lifetime (deleted when the local frame unwinds).
    let raw: jobject = android_ctx.context() as *mut _;
    let global = unsafe { JObject::from_raw(env, raw) };
    let context = env
        .new_local_ref(&global)
        .map_err(|e| Error::platform("android: new_local_ref(context)", e))?;
    f(env, &context)
}

fn with_jni_env<T>(f: impl FnOnce(&mut Env<'_>) -> Result<T, Error>) -> Result<T, Error> {
    let android_ctx = ndk_context::android_context();
    let vm = unsafe { jni::JavaVM::from_raw(android_ctx.vm().cast()) };

    let mut result: Option<Result<T, Error>> = None;

    vm.attach_current_thread(|env| {
        result = Some(f(env));
        JniResult::Ok(())
    })
    .map_err(|e| Error::platform("android: attach_current_thread", e))?;

    result.unwrap_or_else(|| Err(Error::Unavailable("android: jni env unavailable")))
}

fn get_mime_types_jni(env: &mut Env<'_>) -> Result<Vec<String>, Error> {
    with_context(env, |env, context| {
        let manager = get_clipboard_manager(env, context)?;

        // Use getPrimaryClipDescription() — lighter than getPrimaryClip()
        // because it doesn't materialize the clip data.
        let description = env
            .call_method(
                manager,
                jni_str!("getPrimaryClipDescription"),
                jni_sig!("()Landroid/content/ClipDescription;"),
                &[],
            )
            .map_err(|e| Error::platform("android: getPrimaryClipDescription", e))?
            .l()
            .map_err(|e| Error::platform("android: getPrimaryClipDescription result", e))?;

        if description.is_null() {
            return Ok(Vec::new());
        }

        let count: jint = env
            .call_method(
                &description,
                jni_str!("getMimeTypeCount"),
                jni_sig!("()I"),
                &[],
            )
            .map_err(|e| Error::platform("android: getMimeTypeCount", e))?
            .i()
            .map_err(|e| Error::platform("android: getMimeTypeCount result", e))?;

        let mut mimes = Vec::new();
        for i in 0..count {
            // Wrap each iteration in a local frame to avoid accumulating
            // JNI local references across iterations.
            env.with_local_frame(16, |env| {
                let mime = env
                    .call_method(
                        &description,
                        jni_str!("getMimeType"),
                        jni_sig!("(I)Ljava/lang/String;"),
                        &[JValue::Int(i)],
                    )
                    .map_err(|e| Error::platform("android: getMimeType", e))?
                    .l()
                    .map_err(|e| Error::platform("android: getMimeType result", e))?;

                if !mime.is_null() {
                    if let Ok(jstr) = JString::cast_local(env, mime) {
                        if let Ok(s) = jstr.try_to_string(env) {
                            if !mimes.contains(&s) {
                                mimes.push(s);
                            }
                        }
                    }
                }
                Ok::<_, Error>(())
            })
            .map_err(|e| Error::platform("android: local frame", e))?;
        }

        Ok(mimes)
    })
}

fn get_clipboard_manager<'local>(
    env: &mut Env<'local>,
    context: &JObject<'local>,
) -> Result<JObject<'local>, Error> {
    let context_class = env
        .find_class(jni_str!("android/content/Context"))
        .map_err(|e| Error::platform("android: find_class(Context)", e))?;

    let service_field = env
        .get_static_field(
            context_class,
            jni_str!("CLIPBOARD_SERVICE"),
            jni_sig!("Ljava/lang/String;"),
        )
        .map_err(|e| Error::platform("android: get CLIPBOARD_SERVICE", e))?;

    let service_name = service_field
        .l()
        .map_err(|e| Error::platform("android: extract CLIPBOARD_SERVICE", e))?;

    let manager = env
        .call_method(
            context,
            jni_str!("getSystemService"),
            jni_sig!("(Ljava/lang/String;)Ljava/lang/Object;"),
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

fn get_text_jni(env: &mut Env<'_>) -> Result<String, Error> {
    with_context(env, |env, context| {
        let manager = get_clipboard_manager(env, context)?;

        let clip = env
            .call_method(
                manager,
                jni_str!("getPrimaryClip"),
                jni_sig!("()Landroid/content/ClipData;"),
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

        let count: jint = env
            .call_method(&clip, jni_str!("getItemCount"), jni_sig!("()I"), &[])
            .map_err(|e| Error::platform("android: getItemCount", e))?
            .i()
            .map_err(|e| Error::platform("android: getItemCount result", e))?;

        if count <= 0 {
            return Err(Error::Unavailable("clipboard is empty"));
        }

        let item = env
            .call_method(
                clip,
                jni_str!("getItemAt"),
                jni_sig!("(I)Landroid/content/ClipData$Item;"),
                &[JValue::Int(0)],
            )
            .map_err(|e| Error::platform("android: getItemAt(0)", e))?
            .l()
            .map_err(|e| Error::platform("android: getItemAt result", e))?;

        let char_seq = env
            .call_method(
                item,
                jni_str!("coerceToText"),
                jni_sig!("(Landroid/content/Context;)Ljava/lang/CharSequence;"),
                &[JValue::Object(context)],
            )
            .map_err(|e| Error::platform("android: coerceToText", e))?
            .l()
            .map_err(|e| Error::platform("android: coerceToText result", e))?;

        if char_seq.is_null() {
            return Err(Error::Unavailable("clipboard item has no text"));
        }

        let jstring_obj = env
            .call_method(
                char_seq,
                jni_str!("toString"),
                jni_sig!("()Ljava/lang/String;"),
                &[],
            )
            .map_err(|e| Error::platform("android: toString", e))?
            .l()
            .map_err(|e| Error::platform("android: toString result", e))?;

        let jstring = JString::cast_local(env, jstring_obj)
            .map_err(|e| Error::platform("android: cast to JString", e))?;
        let rust_string = jstring
            .try_to_string(env)
            .map_err(|e| Error::platform("android: get_string", e))?;

        Ok(rust_string)
    })
}

fn set_text_jni(env: &mut Env<'_>, text: &str) -> Result<(), Error> {
    with_context(env, |env, context| {
        let manager = get_clipboard_manager(env, context)?;

        let clipdata_class = env
            .find_class(jni_str!("android/content/ClipData"))
            .map_err(|e| Error::platform("android: find_class(ClipData)", e))?;

        let label = JString::from_str(env, "clipawl")
            .map_err(|e| Error::platform("android: new_string(label)", e))?;

        let value = JString::from_str(env, text)
            .map_err(|e| Error::platform("android: new_string(text)", e))?;

        let clipdata = env
            .call_static_method(
                clipdata_class,
                jni_str!("newPlainText"),
                jni_sig!(
                    "(Ljava/lang/CharSequence;Ljava/lang/CharSequence;)Landroid/content/ClipData;"
                ),
                &[JValue::Object(&*label), JValue::Object(&*value)],
            )
            .map_err(|e| Error::platform("android: newPlainText", e))?
            .l()
            .map_err(|e| Error::platform("android: newPlainText result", e))?;

        env.call_method(
            manager,
            jni_str!("setPrimaryClip"),
            jni_sig!("(Landroid/content/ClipData;)V"),
            &[JValue::Object(&clipdata)],
        )
        .map_err(|e| Error::platform("android: setPrimaryClip", e))?;

        Ok(())
    })
}
