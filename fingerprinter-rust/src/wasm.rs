use crate::core::create_hashes_from_wav;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn create_hashes_from_wav_wasm(wav_bytes: &[u8]) -> Result<JsValue, JsValue> {
    let result = create_hashes_from_wav(wav_bytes)
        .map_err(|e| JsValue::from_str(&format!("Error: {}", e)))?;

    serde_wasm_bindgen::to_value(&result)
        .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
}
