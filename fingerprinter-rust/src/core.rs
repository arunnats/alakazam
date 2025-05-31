use crate::fingerprint::AudioFingerprinter;
use crate::models::{QueryFingerprint, SongFingerprint, SongMetadata};

/// Core fingerprinting function that generates hashes from audio data
pub fn generate_song_fingerprint(
    audio_data: &[f32],
    sample_rate: u32,
) -> Result<SongFingerprint, String> {
    let fingerprinter = AudioFingerprinter::new("dummy")
        .map_err(|e| format!("Failed to create fingerprinter: {}", e))?;

    let hashes = fingerprinter.generate_fingerprint(audio_data, sample_rate);
    let hash_count = hashes.len();

    Ok(SongFingerprint {
        hashes,
        metadata: SongMetadata {
            duration: audio_data.len() as f32 / sample_rate as f32,
            sample_rate,
            hash_count,
        },
    })
}

/// Core fingerprinting function for query audio clips
pub fn generate_query_fingerprint(
    audio_clip: &[f32],
    sample_rate: u32,
) -> Result<QueryFingerprint, String> {
    let fingerprinter = AudioFingerprinter::new("dummy")
        .map_err(|e| format!("Failed to create fingerprinter: {}", e))?;

    let hashes = fingerprinter.generate_fingerprint(audio_clip, sample_rate);

    Ok(QueryFingerprint {
        hashes,
        duration: audio_clip.len() as f32 / sample_rate as f32,
    })
}
