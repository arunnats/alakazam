use std::error::Error;

/// Handles loading and preprocessing of audio files
/// This struct provides methods to load audio from WAV files and convert them
/// into a format suitable for fingerprinting
pub struct AudioLoader;

impl AudioLoader {
    /// Loads audio data from a WAV file and returns normalized samples and sample rate
    ///
    /// # Arguments
    /// * `file_path` - Path to the WAV file
    ///
    /// # Returns
    /// * `Result<(Vec<f32>, u32)>` - Tuple containing:
    ///   - Vector of normalized audio samples (-1.0 to 1.0)
    ///   - Sample rate in Hz
    ///
    /// # Processing Steps
    /// 1. Opens and reads the WAV file
    /// 2. Converts samples to floating point format
    /// 3. Normalizes samples to [-1.0, 1.0] range
    /// 4. Converts multi-channel audio to mono
    pub fn load_from_wav(file_path: &str) -> Result<(Vec<f32>, u32), Box<dyn Error>> {
        println!("Loading audio from: {}", file_path);

        // Open and read WAV file
        let mut reader = hound::WavReader::open(file_path)
            .map_err(|e| format!("Failed to open {}: {}", file_path, e))?;

        let spec = reader.spec();
        println!(
            "Audio specs - Sample rate: {}Hz, Channels: {}, Bits: {}",
            spec.sample_rate, spec.channels, spec.bits_per_sample
        );

        // Convert samples to floating point format based on the WAV file's format
        let samples: Result<Vec<f32>, _> = match spec.sample_format {
            hound::SampleFormat::Float => reader.samples::<f32>().collect(),
            hound::SampleFormat::Int => match spec.bits_per_sample {
                16 => reader
                    .samples::<i16>()
                    .map(|s| s.map(|s| s as f32 / i16::MAX as f32))
                    .collect(),
                24 => reader
                    .samples::<i32>()
                    .map(|s| s.map(|s| s as f32 / ((1 << 23) as f32)))
                    .collect(),
                32 => reader
                    .samples::<i32>()
                    .map(|s| s.map(|s| s as f32 / i32::MAX as f32))
                    .collect(),
                _ => {
                    return Err(format!("Unsupported bit depth: {}", spec.bits_per_sample).into());
                }
            },
        };

        let mut audio_samples = samples?;

        // Convert multi-channel audio to mono by averaging channels
        if spec.channels == 2 {
            println!("Converting stereo to mono...");
            audio_samples = audio_samples
                .chunks(2)
                .map(|chunk| (chunk[0] + chunk[1]) / 2.0)
                .collect();
        } else if spec.channels > 2 {
            println!("Converting multi-channel to mono...");
            audio_samples = audio_samples
                .chunks(spec.channels as usize)
                .map(|chunk| chunk.iter().sum::<f32>() / chunk.len() as f32)
                .collect();
        }

        println!(
            "Loaded {} samples ({:.2} seconds)",
            audio_samples.len(),
            audio_samples.len() as f32 / spec.sample_rate as f32
        );

        Ok((audio_samples, spec.sample_rate))
    }
}
