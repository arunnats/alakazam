use crate::core::create_hashes_from_wav;
use jni::objects::{JByteArray, JClass};
use jni::sys::{jbyteArray, jstring};
use jni::JNIEnv;
use serde_json;

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
