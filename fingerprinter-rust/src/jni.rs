use crate::core::{generate_query_fingerprint, generate_song_fingerprint};
use jni::objects::{JByteArray, JClass};
use jni::sys::{jbyteArray, jint, jstring};
use jni::JNIEnv;
use serde_json;

#[no_mangle]
pub extern "system" fn Java_com_alakazam_Fingerprinter_generateSongFingerprint(
    env: JNIEnv,
    _class: JClass,
    audio_data: jbyteArray,
    sample_rate: jint,
) -> jstring {
    match fingerprint_common(&env, audio_data, sample_rate, true) {
        Ok(json_str) => match env.new_string(json_str) {
            Ok(jstr) => jstr.into_raw(),
            Err(_) => std::ptr::null_mut(),
        },
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_alakazam_Fingerprinter_generateQueryFingerprint(
    env: JNIEnv,
    _class: JClass,
    audio_data: jbyteArray,
    sample_rate: jint,
) -> jstring {
    match fingerprint_common(&env, audio_data, sample_rate, false) {
        Ok(json_str) => match env.new_string(json_str) {
            Ok(jstr) => jstr.into_raw(),
            Err(_) => std::ptr::null_mut(),
        },
        Err(_) => std::ptr::null_mut(),
    }
}

fn fingerprint_common(
    env: &JNIEnv,
    audio_data: jbyteArray,
    sample_rate: jint,
    is_song: bool,
) -> Result<String, String> {
    let audio_bytes = env
        .convert_byte_array(unsafe { JByteArray::from_raw(audio_data) })
        .map_err(|e| format!("Byte array conversion failed: {:?}", e))?;

    let audio_f32: Vec<f32> = audio_bytes
        .chunks_exact(4)
        .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect();

    let json = if is_song {
        let fingerprint = generate_song_fingerprint(&audio_f32, sample_rate as u32)?;
        serde_json::to_string(&fingerprint).map_err(|e| e.to_string())?
    } else {
        let fingerprint = generate_query_fingerprint(&audio_f32, sample_rate as u32)?;
        serde_json::to_string(&fingerprint).map_err(|e| e.to_string())?
    };

    Ok(json)
}
