use crate::{ClipboardOptions, Error};
use std::fmt;
use wasm_bindgen::JsCast;

pub(crate) struct ClipboardImpl;

impl ClipboardImpl {
    pub(crate) fn new(_opts: &ClipboardOptions) -> Result<Self, Error> {
        Ok(Self)
    }

    pub(crate) async fn get_text(&mut self) -> Result<String, Error> {
        let window = web_sys::window().ok_or(Error::NotSupported)?;
        let nav = window.navigator();

        let clipboard = nav.clipboard();
        if clipboard.is_undefined() {
            return Err(Error::NotSupported);
        }

        let promise = clipboard.read_text();
        let js = wasm_bindgen_futures::JsFuture::from(promise)
            .await
            .map_err(map_js_err)?;

        Ok(js.as_string().unwrap_or_default())
    }

    pub(crate) async fn set_text(&mut self, text: &str) -> Result<(), Error> {
        let window = web_sys::window().ok_or(Error::NotSupported)?;
        let nav = window.navigator();

        let clipboard = nav.clipboard();
        if clipboard.is_undefined() {
            return Err(Error::NotSupported);
        }

        let promise = clipboard.write_text(text);
        wasm_bindgen_futures::JsFuture::from(promise)
            .await
            .map_err(map_js_err)?;

        Ok(())
    }
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
