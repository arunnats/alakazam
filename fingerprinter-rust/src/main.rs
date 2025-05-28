use redis::{Client, Commands, RedisResult};
use rustfft::{FftPlanner, num_complex::Complex};
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

#[derive(Clone, Debug)]
pub struct FrequencyBands {
    pub bass: (usize, usize),     // 20-250 Hz
    pub low_mid: (usize, usize),  // 250-500 Hz
    pub mid: (usize, usize),      // 500-2000 Hz
    pub high_mid: (usize, usize), // 2000-4000 Hz
    pub treble: (usize, usize),   // 4000-8000 Hz
    pub presence: (usize, usize), // 8000+ Hz
}

impl AudioFingerprinter {
    pub fn new(redis_url: &str) -> RedisResult<Self> {
        let client = Client::open(redis_url)?;
        Ok(AudioFingerprinter {
            redis_client: client,
        })
    }

    // create frequency bands to map it better
    fn create_frequency_bands(&self, fft_size: usize, sample_rate: u32) -> FrequencyBands {
        let freq_resolution = sample_rate as f32 / fft_size as f32;

        // Modified bands to focus more on human voice range (85-255 Hz for male, 165-255 Hz for female)
        // and add more tolerance in the mid-range where most singing occurs
        FrequencyBands {
            bass: (
                self.freq_to_bin(20.0, freq_resolution),
                self.freq_to_bin(300.0, freq_resolution), // Extended bass range
            ),
            low_mid: (
                self.freq_to_bin(300.0, freq_resolution),
                self.freq_to_bin(800.0, freq_resolution), // Extended low-mid for voice fundamentals
            ),
            mid: (
                self.freq_to_bin(800.0, freq_resolution),
                self.freq_to_bin(3000.0, freq_resolution), // Extended mid-range for voice harmonics
            ),
            high_mid: (
                self.freq_to_bin(3000.0, freq_resolution),
                self.freq_to_bin(5000.0, freq_resolution), // Reduced high-mid range
            ),
            treble: (
                self.freq_to_bin(5000.0, freq_resolution),
                self.freq_to_bin(8000.0, freq_resolution),
            ),
            presence: (
                self.freq_to_bin(8000.0, freq_resolution),
                self.freq_to_bin(20000.0, freq_resolution),
            ),
        }
    }

    fn freq_to_bin(&self, freq: f32, freq_resolution: f32) -> usize {
        (freq / freq_resolution).round() as usize
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
                let peaks = self.extract_peaks(&spectrum, sample_rate);
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

    fn extract_peaks(&self, spectrum: &[f32], sample_rate: u32) -> Vec<(usize, f32, String)> {
        let bands = self.create_frequency_bands(spectrum.len() * 2, sample_rate);
        let mut peaks = Vec::new();

        // Extract peaks from each frequency band with different thresholds
        let band_configs = [
            ("bass", bands.bass, 3, 1.1),       // More peaks, lower threshold
            ("low_mid", bands.low_mid, 4, 1.0), // Most important for voice
            ("mid", bands.mid, 4, 1.0),         // Most important for voice
            ("high_mid", bands.high_mid, 2, 1.2),
            ("treble", bands.treble, 1, 1.3),
            ("presence", bands.presence, 1, 1.4),
        ];

        for (band_name, (start, end), max_peaks, threshold_multiplier) in band_configs {
            let band_spectrum = &spectrum[start..end.min(spectrum.len())];
            let band_threshold = band_spectrum.iter().sum::<f32>() / band_spectrum.len() as f32
                * threshold_multiplier;

            let mut band_peaks = Vec::new();

            // Use a wider window for peak detection to be more tolerant
            let window_size = 3;
            for i in window_size..band_spectrum.len() - window_size {
                let window = &band_spectrum[i - window_size..i + window_size + 1];
                let center_value = band_spectrum[i];

                // Check if center is a peak within the window
                if center_value > band_threshold
                    && center_value
                        >= *window
                            .iter()
                            .max_by(|a, b| a.partial_cmp(b).unwrap())
                            .unwrap()
                {
                    band_peaks.push((start + i, center_value, band_name.to_string()));
                }
            }

            // Sort by magnitude and take top peaks
            band_peaks.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
            band_peaks.truncate(max_peaks);
            peaks.extend(band_peaks);
        }

        peaks
    }

    fn peaks_to_hashes(&self, peaks: &[(usize, f32, String)]) -> Vec<u64> {
        let mut hashes = Vec::new();
        let mut band_groups: std::collections::HashMap<String, Vec<(usize, f32)>> =
            std::collections::HashMap::new();

        // Group peaks by frequency band
        for (freq, amp, band) in peaks {
            band_groups
                .entry(band.clone())
                .or_insert_with(Vec::new)
                .push((*freq, *amp));
        }

        // Generate more specific hashes
        for (band_name, band_peaks) in &band_groups {
            // Sort peaks by frequency for more consistent hashing
            let mut sorted_peaks = band_peaks.clone();
            sorted_peaks.sort_by_key(|&(freq, _)| freq);

            // Create hashes with more specific combinations
            for i in 0..sorted_peaks.len() {
                for j in (i + 1)..sorted_peaks.len() {
                    let (freq1, amp1) = sorted_peaks[i];
                    let (freq2, amp2) = sorted_peaks[j];

                    // Include amplitude information in the hash
                    let amp_ratio = (amp1 / amp2 * 100.0) as u8;

                    // Create a more specific hash that includes:
                    // - Band information (6 bits)
                    // - Frequency difference (16 bits)
                    // - Amplitude ratio (8 bits)
                    // - Frequency sum (16 bits)
                    let band_id = self.band_name_to_id(band_name);
                    let freq_diff = (freq2 as i32 - freq1 as i32).abs() as u16;
                    let freq_sum = (freq1 + freq2) as u16;

                    let hash = ((band_id as u64) << 58)
                        | ((freq_diff as u64) << 42)
                        | ((amp_ratio as u64) << 34)
                        | ((freq_sum as u64) << 18);

                    hashes.push(hash);
                }
            }
        }

        hashes
    }

    fn band_name_to_id(&self, band_name: &str) -> u8 {
        match band_name {
            "bass" => 1,
            "low_mid" => 2,
            "mid" => 3,
            "high_mid" => 4,
            "treble" => 5,
            "presence" => 6,
            _ => 0,
        }
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

        let mut song_matches: HashMap<u64, (usize, Vec<u64>)> = HashMap::new();

        // Count matches for each song with hash tracking
        for hash in &query_fingerprints {
            let hash_key = format!("hash:{}", hash);
            let song_ids: Vec<u64> = conn.smembers(&hash_key)?;

            for song_id in song_ids {
                let entry = song_matches.entry(song_id).or_insert((0, Vec::new()));
                entry.0 += 1;
                entry.1.push(*hash);
            }
        }

        // Convert to results with improved confidence scores
        let mut results = Vec::new();
        for (song_id, (match_count, matched_hashes)) in song_matches {
            let song_key = format!("song:{}", song_id);
            if let Ok(song_json) = conn.get::<String, String>(song_key) {
                if let Ok(song_info) = serde_json::from_str::<SongInfo>(&song_json) {
                    // Calculate a more sophisticated confidence score
                    let total_hashes = query_fingerprints.len() as f32;
                    let unique_matches = matched_hashes.len() as f32;

                    // Base confidence on unique matches
                    let base_confidence = unique_matches / total_hashes;

                    // Apply a penalty for low match counts
                    let match_ratio = match_count as f32 / unique_matches;
                    let match_penalty = if match_ratio > 2.0 {
                        0.8 // Heavy penalty for too many duplicate matches
                    } else if match_ratio > 1.5 {
                        0.9 // Medium penalty
                    } else {
                        1.0 // No penalty
                    };

                    // Apply a minimum threshold
                    let confidence = if base_confidence < 0.1 {
                        0.0 // Too few matches
                    } else {
                        base_confidence * match_penalty
                    };

                    if confidence > 0.0 {
                        results.push((song_info, confidence));
                    }
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

    // Load and store the full song
    println!("Loading song.wav...");
    let (song_audio, song_sample_rate) = load_audio_from_wav("505.wav")?;

    let song_info = SongInfo {
        name: "505".to_string(),
        singer: "Arctic Monkeys".to_string(),
    };

    println!("Storing song fingerprint...");
    fingerprinter.store_song(&song_info, &song_audio, song_sample_rate)?;

    println!("Loading song.wav...");
    let (song_audio, song_sample_rate) = load_audio_from_wav("song2.wav")?;

    let song_info = SongInfo {
        name: "LINKE HIM".to_string(),
        singer: "TYLER".to_string(),
    };

    println!("Storing song fingerprint...");
    fingerprinter.store_song(&song_info, &song_audio, song_sample_rate)?;

    println!("Loading song.wav...");
    let (song_audio, song_sample_rate) = load_audio_from_wav("input.wav")?;

    let song_info = SongInfo {
        name: "we fell in october".to_string(),
        singer: "girl in red".to_string(),
    };

    println!("Storing song fingerprint...");
    fingerprinter.store_song(&song_info, &song_audio, song_sample_rate)?;

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

    // Load and search with test clip
    println!("Loading test.wav...");
    let (test_audio, test_sample_rate) = load_audio_from_wav("record_out1.wav")?;

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

    // Load and search with test clip
    println!("Loading test.wav...");
    let (test_audio, test_sample_rate) = load_audio_from_wav("in1.wav")?;

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

    // Load and search with test clip
    println!("Loading test.wav...");
    let (test_audio, test_sample_rate) = load_audio_from_wav("in2.wav")?;

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

    // Load and search with test clip
    println!("Loading test.wav...");
    let (test_audio, test_sample_rate) = load_audio_from_wav("in4.wav")?;

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
