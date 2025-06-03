use crate::core::{generate_query_fingerprint, generate_song_fingerprint};
use console_error_panic_hook;
use serde_wasm_bindgen::to_value;
use wasm_bindgen::prelude::*;

fn fingerprint_common(
    audio_bytes: &[u8],
    sample_rate: u32,
    is_song: bool,
) -> Result<JsValue, JsValue> {
    console_error_panic_hook::set_once();

    // let audio_bytes: Vec<u8> = audio_bytes
    //     .iter()
    //     .map(|b| i8::from_ne_bytes([*b]) as i16 + 128)
    //     .map(|v| v as u8)
    //     .collect();

    if audio_bytes.len() % 4 != 0 {
        return Err(JsValue::from_str("Audio byte length not divisible by 4"));
    }

    let mut audio_f32: Vec<f32> = audio_bytes
        .chunks_exact(4)
        .map(|chunk| {
            let bytes = [chunk[0], chunk[1], chunk[2], chunk[3]];
            f32::from_le_bytes(bytes)
        })
        .collect();

    if audio_f32.len() % 2 == 0 {
        audio_f32 = audio_f32
            .chunks(2)
            .map(|chunk| (chunk[0] + chunk[1]) / 2.0)
            .collect();
    }

    let raw_result = if is_song {
        generate_song_fingerprint(&audio_f32, sample_rate).map(|fp| {
            fp.hashes
                .into_iter()
                .map(|h| h.to_string())
                .collect::<Vec<_>>()
        })
    } else {
        generate_query_fingerprint(&audio_f32, sample_rate).map(|fp| {
            fp.hashes
                .into_iter()
                .map(|h| h.to_string())
                .collect::<Vec<_>>()
        })
    };

    raw_result
        .map_err(|e| JsValue::from_str(&e.to_string()))
        .and_then(|s| to_value(&s).map_err(|e| JsValue::from_str(&e.to_string())))
}

#[wasm_bindgen]
pub fn generate_song_fingerprint_wasm(
    audio_bytes: &[u8],
    sample_rate: u32,
) -> Result<JsValue, JsValue> {
    fingerprint_common(audio_bytes, sample_rate, true)
}

#[wasm_bindgen]
pub fn generate_query_fingerprint_wasm(
    audio_bytes: &[u8],
    sample_rate: u32,
) -> Result<JsValue, JsValue> {
    fingerprint_common(audio_bytes, sample_rate, false)
}

#[wasm_bindgen]
pub fn test_wasm() -> String {
    "WASM is working!".to_string()
}
