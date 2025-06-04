use crate::audio::AudioLoader;
use crate::core::{create_hashes_from_wav, generate_query_fingerprint, generate_song_fingerprint};
use crate::models::AudioLoadResult;
use jni::objects::{JByteArray, JClass, JString};
use jni::sys::{jbyteArray, jint, jstring};
use jni::JNIEnv;
use serde_json;

#[no_mangle]
pub extern "system" fn Java_com_alakazam_backend_1spring_fingerprinter_Fingerprinter_loadAudioFromWav(
    mut env: JNIEnv,
    _class: JClass,
    file_path: JString,
) -> jstring {
    print!("Checka/n");
    // Convert Java string to Rust string
    let file_path_str: String = match env.get_string(&file_path) {
        Ok(java_str) => java_str.into(),
        Err(_) => return std::ptr::null_mut(),
    };
    print!("Checkb/n");
    // Load audio using your AudioLoader
    let (audio_data, sample_rate) = match AudioLoader::load_from_wav(&file_path_str) {
        Ok(data) => data,
        Err(e) => {
            eprintln!("Failed to load audio: {}", e);
            return std::ptr::null_mut();
        }
    };
    print!("Checkc/n");
    // Create a result struct to return as JSON
    let duration = audio_data.len() as f32 / sample_rate as f32;
    let sample_count = audio_data.len();

    let result = AudioLoadResult {
        audio_data,
        sample_rate,
        duration,
        sample_count,
    };

    // Serialize to JSON
    let json = match serde_json::to_string(&result) {
        Ok(json) => json,
        Err(_) => return std::ptr::null_mut(),
    };

    // Return as Java string
    match env.new_string(json) {
        Ok(jstring) => jstring.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_alakazam_backend_1spring_fingerprinter_Fingerprinter_generateSongFingerprint(
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
pub extern "system" fn Java_com_alakazam_backend_1spring_fingerprinter_Fingerprinter_generateQueryFingerprint(
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

#[no_mangle]
pub extern "system" fn Java_com_alakazam_backend_1spring_fingerprinter_Fingerprinter_testFunc(
    env: JNIEnv,
    _class: JClass,
) -> jstring {
    match env.new_string("success") {
        Ok(jstr) => jstr.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

fn fingerprint_common(
    env: &JNIEnv,
    audio_data: jbyteArray,
    sample_rate: jint,
    is_song: bool,
) -> Result<String, String> {
    println!("Check1");

    let audio_bytes = env
        .convert_byte_array(unsafe { JByteArray::from_raw(audio_data) })
        .map_err(|e| format!("Byte array conversion failed: {:?}", e))?;

    println!("Check2 - Audio bytes length: {}", audio_bytes.len());

    let audio_f32: Vec<f32> = audio_bytes
        .chunks_exact(4)
        .map(|chunk| {
            let bytes = [
                chunk[0] as u8,
                chunk[1] as u8,
                chunk[2] as u8,
                chunk[3] as u8,
            ];
            f32::from_le_bytes(bytes)
        })
        .collect();

    println!("Check3 - Audio f32 length: {}", audio_f32.len());

    let json = if is_song {
        println!("Calling generate_song_fingerprint...");
        let fingerprint = generate_song_fingerprint(&audio_f32, sample_rate as u32)
            .map_err(|e| format!("Fingerprint generation failed: {}", e))?;
        println!("Fingerprint generated successfully");
        serde_json::to_string(&fingerprint).map_err(|e| e.to_string())?
    } else {
        let fingerprint = generate_query_fingerprint(&audio_f32, sample_rate as u32)
            .map_err(|e| format!("Query fingerprint generation failed: {}", e))?;
        serde_json::to_string(&fingerprint).map_err(|e| e.to_string())?
    };

    println!("JSON serialization successful");
    Ok(json)
}

#[no_mangle]
pub extern "system" fn Java_com_alakazam_backend_1spring_fingerprinter_Fingerprinter_createHashesFromWav(
    env: JNIEnv,
    _class: JClass,
    wav_bytes: jbyteArray, // This is a raw JNI pointer
) -> jstring {
    // Convert raw jbyteArray to JByteArray first
    let java_bytes = unsafe { JByteArray::from_raw(wav_bytes) };

    // Now convert to Rust Vec<u8>
    let bytes = env
        .convert_byte_array(java_bytes)
        .expect("Failed to convert byte array");

    // Rest of your code remains the same
    match create_hashes_from_wav(&bytes) {
        Ok(result) => {
            let json = serde_json::to_string(&result).expect("Failed to serialize result");
            env.new_string(json)
                .expect("Failed to create JVM string")
                .into_raw()
        }
        Err(e) => env
            .new_string(format!("Error: {}", e))
            .expect("Failed to create error string")
            .into_raw(),
    }
}
