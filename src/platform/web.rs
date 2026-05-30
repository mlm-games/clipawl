use crate::{ClipboardOptions, Error};
use js_sys::{Array, Object, Reflect, Uint8Array};
use std::fmt;
use wasm_bindgen::{JsCast, JsValue};

pub(crate) struct ClipboardImpl;

impl ClipboardImpl {
    pub(crate) fn new(_opts: &ClipboardOptions) -> Result<Self, Error> {
        Ok(Self)
    }

    pub(crate) async fn mime_types(&self) -> Result<Vec<String>, Error> {
        let clipboard = get_clipboard()?;
        let promise = clipboard.read();
        let items = wasm_bindgen_futures::JsFuture::from(promise)
            .await
            .map_err(map_js_err)?;
        let items = Array::from(&items);
        let mut mimes = Vec::new();
        for i in 0..items.length() {
            let val = items.get(i);
            if val.is_undefined() || val.is_null() {
                continue;
            }
            let item = match val.dyn_into::<web_sys::ClipboardItem>() {
                Ok(item) => item,
                Err(_) => continue,
            };
            let types_val =
                js_sys::Reflect::get(&item, &JsValue::from("types")).unwrap_or(JsValue::UNDEFINED);
            let types_arr = match types_val.dyn_into::<Array>() {
                Ok(arr) => arr,
                Err(_) => continue,
            };
            for j in 0..types_arr.length() {
                if let Some(s) = types_arr.get(j).as_string() {
                    if !mimes.contains(&s) {
                        mimes.push(s);
                    }
                }
            }
        }
        Ok(mimes)
    }

    pub(crate) async fn read(&self, mime_type: &str) -> Result<Vec<u8>, Error> {
        if mime_type == "text/plain" {
            let clipboard = get_clipboard()?;
            let promise = clipboard.read_text();
            let js = wasm_bindgen_futures::JsFuture::from(promise)
                .await
                .map_err(map_js_err)?;
            return Ok(js.as_string().unwrap_or_default().into_bytes());
        }

        let clipboard = get_clipboard()?;
        let promise = clipboard.read();
        let items = wasm_bindgen_futures::JsFuture::from(promise)
            .await
            .map_err(map_js_err)?;
        let items = Array::from(&items);
        let item = items.get(0);
        if item.is_undefined() {
            return Ok(Vec::new());
        }
        let item = item
            .dyn_into::<web_sys::ClipboardItem>()
            .map_err(|_| Error::Unavailable("not a ClipboardItem"))?;
        let blob_promise = item.get_type(mime_type);
        let blob = wasm_bindgen_futures::JsFuture::from(blob_promise)
            .await
            .map_err(map_js_err)?;
        let blob = blob
            .dyn_into::<web_sys::Blob>()
            .map_err(|_| Error::Unavailable("clipboard data not a Blob"))?;
        let buf_promise = blob.array_buffer();
        let buf = wasm_bindgen_futures::JsFuture::from(buf_promise)
            .await
            .map_err(map_js_err)?;
        let uint8 = Uint8Array::new(&buf);
        let mut vec = vec![0u8; uint8.length() as usize];
        uint8.copy_to(&mut vec);
        Ok(vec)
    }

    pub(crate) async fn write(&self, mime_type: &str, data: &[u8]) -> Result<(), Error> {
        if mime_type == "text/plain" {
            let text = std::str::from_utf8(data).map_err(|e| Error::platform("web: utf-8", e))?;
            let clipboard = get_clipboard()?;
            let promise = clipboard.write_text(text);
            wasm_bindgen_futures::JsFuture::from(promise)
                .await
                .map_err(map_js_err)?;
            return Ok(());
        }

        let clipboard = get_clipboard()?;
        let obj = Object::new();

        let bag = web_sys::BlobPropertyBag::new();
        bag.set_type(mime_type);

        // Text-based MIME types go as strings; binary as Uint8Array blobs
        if mime_type.starts_with("text/") || mime_type == "application/rtf" {
            let text = std::str::from_utf8(data).map_err(|e| Error::platform("web: utf-8", e))?;
            let blob = web_sys::Blob::new_with_str_sequence_and_options(
                &Array::of1(&JsValue::from(text)),
                &bag,
            )
            .map_err(|e| Error::platform("web: create blob", JsError(format!("{:?}", e))))?;
            Reflect::set(&obj, &JsValue::from(mime_type), &blob)
                .map_err(|e| Error::platform("web: set blob", JsError(format!("{:?}", e))))?;
        } else {
            let uint8 = Uint8Array::from(data);
            let blob =
                web_sys::Blob::new_with_u8_array_sequence_and_options(&Array::of1(&uint8), &bag)
                    .map_err(|e| {
                        Error::platform("web: create blob", JsError(format!("{:?}", e)))
                    })?;
            Reflect::set(&obj, &JsValue::from(mime_type), &blob)
                .map_err(|e| Error::platform("web: set blob", JsError(format!("{:?}", e))))?;
        }

        let item = web_sys::ClipboardItem::new_with_record_from_str_to_blob_promise(&obj).map_err(
            |e| Error::platform("web: create ClipboardItem", JsError(format!("{:?}", e))),
        )?;
        let arr = Array::of1(&item);
        let promise = clipboard.write(&arr);
        wasm_bindgen_futures::JsFuture::from(promise)
            .await
            .map_err(map_js_err)?;
        Ok(())
    }
}

fn get_clipboard() -> Result<web_sys::Clipboard, Error> {
    let window = web_sys::window().ok_or(Error::NotSupported)?;
    let nav = window.navigator();
    let clipboard = nav.clipboard();
    if clipboard.is_undefined() {
        return Err(Error::NotSupported);
    }
    Ok(clipboard)
}

/// Helper struct to wrap JS error strings into a std::error::Error
#[derive(Debug)]
struct JsError(String);

impl fmt::Display for JsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for JsError {}

fn map_js_err(err: wasm_bindgen::JsValue) -> Error {
    if let Some(dom) = err.dyn_ref::<web_sys::DomException>() {
        match dom.name().as_str() {
            "NotAllowedError" => return Error::PermissionDenied("User activation required"),
            "NotFoundError" => return Error::Unavailable("Clipboard API returned NotFound"),
            _ => {
                return Error::platform(
                    "web dom exception",
                    JsError(format!("{}: {}", dom.name(), dom.message())),
                );
            }
        }
    }

    Error::platform("web api error", JsError(format!("{:?}", err)))
}
