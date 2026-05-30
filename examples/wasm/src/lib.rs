use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub async fn copy_text(text: String) -> Result<(), JsValue> {
    let clipboard = clipawl::Clipboard::new().map_err(|e| JsValue::from_str(&e.to_string()))?;

    clipboard
        .write(&text)
        .await
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

#[wasm_bindgen]
pub async fn paste_text() -> Result<String, JsValue> {
    let clipboard = clipawl::Clipboard::new().map_err(|e| JsValue::from_str(&e.to_string()))?;

    clipboard
        .read()
        .await
        .map_err(|e| JsValue::from_str(&e.to_string()))
}
