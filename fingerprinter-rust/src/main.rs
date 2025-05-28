use redis::{Client, Commands, RedisResult};
use rustfft::{num_complex::Complex, FftPlanner};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SongInfo {
    pub name: String,
    pub singer: String,
}

#[derive(Clone, Debug)]
pub struct AudioFingerprinter {
    redis_client: Client,
}

impl AudioFingerprinter {
    pub fn new(redis_url: &str) -> RedisResult<Self> {
        let client = Client::open(redis_url)?;
        Ok(AudioFingerprinter {
            redis_client: client,
        })
    }

    // Generate fingerprint hashes from audio data
    pub fn generate_fingerprint(&self, audio_data: &[f32], sample_rate: u32) -> Vec<u64> {
        let window_size = 1024;
        let hop_size = window_size / 2;
        let mut fingerprints = Vec::new();

        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(window_size);

        // Process audio in overlapping windows
        for window_start in (0..audio_data.len().saturating_sub(window_size)).step_by(hop_size) {
            let window_end = (window_start + window_size).min(audio_data.len());
            let window = &audio_data[window_start..window_end];

            if window.len() == window_size {
                let spectrum = self.compute_spectrum(window, &*fft);
                let peaks = self.extract_peaks(&spectrum);
                let hashes = self.peaks_to_hashes(&peaks);
                fingerprints.extend(hashes);
            }
        }

        fingerprints
    }

    fn compute_spectrum(&self, window: &[f32], fft: &dyn rustfft::Fft<f32>) -> Vec<f32> {
        let mut buffer: Vec<Complex<f32>> = window
            .iter()
            .map(|&x| Complex::new(x * self.hamming_window(window.len()), 0.0))
            .collect();

        fft.process(&mut buffer);

        // Convert to magnitude spectrum
        buffer
            .iter()
            .take(buffer.len() / 2) // Take only positive frequencies
            .map(|c| c.norm())
            .collect()
    }

    fn hamming_window(&self, n: usize) -> f32 {
        // Simplified hamming window - in practice you'd apply this per sample
        0.54 - 0.46 * (2.0 * std::f32::consts::PI / n as f32).cos()
    }

    fn extract_peaks(&self, spectrum: &[f32]) -> Vec<(usize, f32)> {
        let mut peaks = Vec::new();
        let threshold = spectrum.iter().sum::<f32>() / spectrum.len() as f32 * 1.5;

        // Find local maxima above threshold
        for i in 1..spectrum.len() - 1 {
            if spectrum[i] > threshold
                && spectrum[i] > spectrum[i - 1]
                && spectrum[i] > spectrum[i + 1]
            {
                peaks.push((i, spectrum[i]));
            }
        }

        // Sort by magnitude and take top peaks
        peaks.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        peaks.truncate(5); // Take top 5 peaks per window
        peaks
    }

    fn peaks_to_hashes(&self, peaks: &[(usize, f32)]) -> Vec<u64> {
        let mut hashes = Vec::new();

        // Create combinatorial hashes from peak pairs
        for (i, &(freq1, _)) in peaks.iter().enumerate() {
            for &(freq2, _) in peaks.iter().skip(i + 1) {
                // Create hash from frequency pair
                // Simple hash: combine frequencies with bit shifting
                let hash = ((freq1 as u64) << 16) | (freq2 as u64);
                hashes.push(hash);
            }
        }

        hashes
    }

    // Store song fingerprint in Redis
    pub fn store_song(
        &self,
        song_info: &SongInfo,
        audio_data: &[f32],
        sample_rate: u32,
    ) -> RedisResult<()> {
        let mut conn = self.redis_client.get_connection()?;
        let fingerprints = self.generate_fingerprint(audio_data, sample_rate);

        // Generate unique song ID
        let song_id: u64 = conn.incr("song_counter", 1)?;

        // Store song metadata
        let song_key = format!("song:{}", song_id);
        let song_json = serde_json::to_string(song_info).unwrap();
        let _: () = conn.set(&song_key, song_json)?;

        // Store fingerprint mappings
        for hash in fingerprints {
            let hash_key = format!("hash:{}", hash);
            let _: () = conn.sadd(&hash_key, song_id)?;
        }

        println!(
            "Stored song '{}' by '{}' with ID: {}",
            song_info.name, song_info.singer, song_id
        );
        Ok(())
    }

    // Search for song using audio clip
    pub fn search_song(
        &self,
        audio_clip: &[f32],
        sample_rate: u32,
    ) -> RedisResult<Vec<(SongInfo, f32)>> {
        let mut conn = self.redis_client.get_connection()?;
        let query_fingerprints = self.generate_fingerprint(audio_clip, sample_rate);

        let mut song_matches: HashMap<u64, usize> = HashMap::new();

        // Count matches for each song
        for hash in &query_fingerprints {
            let hash_key = format!("hash:{}", hash);
            let song_ids: Vec<u64> = conn.smembers(&hash_key)?;

            for song_id in song_ids {
                *song_matches.entry(song_id).or_insert(0) += 1;
            }
        }

        // Convert to results with confidence scores
        let mut results = Vec::new();
        for (song_id, match_count) in song_matches {
            let song_key = format!("song:{}", song_id);
            if let Ok(song_json) = conn.get::<String, String>(song_key) {
                if let Ok(song_info) = serde_json::from_str::<SongInfo>(&song_json) {
                    // Simple confidence score based on match count
                    let confidence = match_count as f32 / query_fingerprints.len() as f32;
                    results.push((song_info, confidence));
                }
            }
        }

        // Sort by confidence (highest first)
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        Ok(results)
    }
}

// Helper function to load audio from WAV file
pub fn load_audio_from_wav(file_path: &str) -> Result<(Vec<f32>, u32), Box<dyn std::error::Error>> {
    println!("Loading audio from: {}", file_path);

    let mut reader = hound::WavReader::open(file_path)
        .map_err(|e| format!("Failed to open {}: {}", file_path, e))?;

    let spec = reader.spec();
    println!(
        "Audio specs - Sample rate: {}Hz, Channels: {}, Bits: {}",
        spec.sample_rate, spec.channels, spec.bits_per_sample
    );

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

    // Convert stereo to mono if needed
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let fingerprinter = AudioFingerprinter::new("redis://127.0.0.1/")?;

    // // Load and store the full song
    // println!("Loading song.wav...");
    // let (song_audio, song_sample_rate) = load_audio_from_wav("505.wav")?;

    // let song_info = SongInfo {
    //     name: "505".to_string(),
    //     singer: "Arctic Monkeys".to_string(),
    // };

    // println!("Storing song fingerprint...");
    // fingerprinter.store_song(&song_info, &song_audio, song_sample_rate)?;

    // Load and search with test clip
    println!("Loading test.wav...");
    let (test_audio, test_sample_rate) = load_audio_from_wav("record_out2.wav")?;

    println!("Searching for matches...");
    let results = fingerprinter.search_song(&test_audio, test_sample_rate)?;

    // Display results
    if results.is_empty() {
        println!("No matches found!");
    } else {
        println!("Found {} matches:", results.len());
        for (i, (song, confidence)) in results.iter().enumerate().take(5) {
            println!(
                "  {}. {} by {} (confidence: {:.3})",
                i + 1,
                song.name,
                song.singer,
                confidence
            );
        }
    }

    Ok(())
}
