use serde::{Deserialize, Serialize};

/// Represents metadata for a song
/// This structure is serializable/deserializable for storage in Redis
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SongInfo {
    pub name: String,   // Name of the song
    pub singer: String, // Name of the artist/singer
}

/// Defines frequency bands used in the fingerprinting algorithm
/// Each band represents a range of frequencies that are analyzed separately
/// This allows for more accurate matching by considering different frequency ranges
/// independently, as different types of sounds (bass, vocals, etc.) occupy different bands
#[derive(Clone, Debug)]
pub struct FrequencyBands {
    pub bass: (usize, usize), // 20-300 Hz: Low frequency sounds, bass instruments
    pub low_mid: (usize, usize), // 300-800 Hz: Lower mid-range, voice fundamentals
    pub mid: (usize, usize),  // 800-3000 Hz: Mid-range, most important for voice
    pub high_mid: (usize, usize), // 3000-5000 Hz: Upper mid-range, voice harmonics
    pub treble: (usize, usize), // 5000-8000 Hz: High frequencies, cymbals, etc.
    pub presence: (usize, usize), // 8000+ Hz: Very high frequencies, air and presence
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SongFingerprint {
    pub hashes: Vec<u64>,
    pub metadata: SongMetadata,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct QueryFingerprint {
    pub hashes: Vec<u64>,
    pub duration: f32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SongMetadata {
    pub duration: f32,
    pub sample_rate: u32,
    pub hash_count: usize,
}

#[derive(serde::Serialize)]
pub struct AudioLoadResult {
    pub audio_data: Vec<f32>,
    pub sample_rate: u32,
    pub duration: f32,
    pub sample_count: usize,
}

#[derive(Serialize)]
pub struct SerializableHash {
    pub(crate) hash: String,
    offset: u32,
}
