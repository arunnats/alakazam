use crate::core::{generate_query_fingerprint, generate_song_fingerprint};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn generate_song_fingerprint_wasm(audio_data: &[f32], sample_rate: u32) -> String {
    let fingerprint = generate_song_fingerprint(audio_data, sample_rate);
    serde_json::to_string(&fingerprint).unwrap()
}

#[wasm_bindgen]
pub fn generate_query_fingerprint_wasm(audio_data: &[f32], sample_rate: u32) -> String {
    let fingerprint = generate_query_fingerprint(audio_data, sample_rate);
    serde_json::to_string(&fingerprint).unwrap()
}
