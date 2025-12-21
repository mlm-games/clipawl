use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub async fn copy_text(text: String) -> Result<(), JsValue> {
    let mut clipboard = clipawl::Clipboard::new().map_err(|e| JsValue::from_str(&e.to_string()))?;

    clipboard
        .set_text(&text)
        .await
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

#[wasm_bindgen]
pub async fn paste_text() -> Result<String, JsValue> {
    let mut clipboard = clipawl::Clipboard::new().map_err(|e| JsValue::from_str(&e.to_string()))?;

    clipboard
        .get_text()
        .await
        .map_err(|e| JsValue::from_str(&e.to_string()))
}
