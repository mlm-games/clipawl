use crate::{Error, Result, SetTextOptions};
use wasm_bindgen::JsCast;

pub(crate) struct ClipboardImpl;

impl ClipboardImpl {
    pub(crate) fn new() -> Result<Self> {
        Ok(Self)
    }

    pub(crate) async fn get_text(&mut self) -> Result<String> {
        let window = web_sys::window().ok_or(Error::NotSupported)?;
        let nav = window.navigator();

        // web-sys exposes navigator.clipboard() as a getter returning Clipboard.
        // In some runtimes it may still be undefined; guard that.
        let clipboard = nav.clipboard();
        if clipboard.as_ref().is_undefined() {
            return Err(Error::NotSupported);
        }

        let promise = clipboard.read_text();
        let js = wasm_bindgen_futures::JsFuture::from(promise)
            .await
            .map_err(map_js_err)?;

        Ok(js.as_string().unwrap_or_default())
    }

    pub(crate) async fn set_text(&mut self, text: &str, _options: SetTextOptions) -> Result<()> {
        let window = web_sys::window().ok_or(Error::NotSupported)?;
        let nav = window.navigator();

        let clipboard = nav.clipboard();
        if clipboard.as_ref().is_undefined() {
            return Err(Error::NotSupported);
        }

        let promise = clipboard.write_text(text);
        wasm_bindgen_futures::JsFuture::from(promise)
            .await
            .map_err(map_js_err)?;

        Ok(())
    }
}

fn map_js_err(err: wasm_bindgen::JsValue) -> Error {
    // NotAllowedError is the common failure mode when permissions/user activation fail.
    if let Some(dom) = err.dyn_ref::<web_sys::DomException>() {
        match dom.name().as_str() {
            "NotAllowedError" => return Error::PermissionDenied,
            "NotFoundError" => return Error::Unavailable,
            _ => return Error::Platform(format!("DOMException {}: {}", dom.name(), dom.message())),
        }
    }
    Error::Platform(format!("{err:?}"))
}
