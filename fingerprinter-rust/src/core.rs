use crate::fingerprint::AudioFingerprinter;
use crate::models::AudioHashes;

pub use hound::WavReader;
pub use std::io::Cursor;

/// Core function that processes WAV bytes and returns hashes
pub fn create_hashes_from_wav(wav_bytes: &[u8]) -> Result<AudioHashes, Box<dyn std::error::Error>> {
    // Decode WAV file
    let mut cursor = Cursor::new(wav_bytes);
    let mut reader = WavReader::new(&mut cursor)?;
    let spec = reader.spec();

    // Convert samples to normalized f32
    let samples: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => reader.samples::<f32>().collect::<Result<_, _>>()?,
        hound::SampleFormat::Int => match spec.bits_per_sample {
            16 => reader
                .samples::<i16>()
                .map(|s| s.map(|s| s as f32 / i16::MAX as f32))
                .collect::<Result<_, _>>()?,
            24 => reader
                .samples::<i32>()
                .map(|s| s.map(|s| s as f32 / (1 << 23) as f32))
                .collect::<Result<_, _>>()?,
            32 => reader
                .samples::<i32>()
                .map(|s| s.map(|s| s as f32 / i32::MAX as f32))
                .collect::<Result<_, _>>()?,
            _ => return Err(format!("Unsupported bit depth: {}", spec.bits_per_sample).into()),
        },
    };

    // Convert to mono
    let audio_data = if spec.channels > 1 {
        samples
            .chunks(spec.channels as usize)
            .map(|chunk| chunk.iter().sum::<f32>() / chunk.len() as f32)
            .collect()
    } else {
        samples
    };

    // Generate fingerprints
    let fingerprinter = AudioFingerprinter::new();
    let hashes_u64 = fingerprinter.generate_fingerprint(&audio_data, spec.sample_rate);

    // Convert to strings
    let hashes = hashes_u64.into_iter().map(|h| h.to_string()).collect();

    Ok(AudioHashes {
        hashes,
        sample_rate: spec.sample_rate,
        duration_seconds: audio_data.len() as f32 / spec.sample_rate as f32,
    })
}
