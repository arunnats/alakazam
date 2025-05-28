use crate::models::{FrequencyBands, SongInfo};
use rustfft::{FftPlanner, num_complex::Complex};

/// Main fingerprinting engine that handles audio fingerprint generation and matching
/// This struct implements the core audio fingerprinting algorithm which:
/// 1. Converts audio to frequency domain using FFT
/// 2. Extracts significant peaks in different frequency bands
/// 3. Creates robust hashes from peak combinations
/// 4. Stores and searches fingerprints using Redis
pub struct AudioFingerprinter {
    storage: crate::storage::RedisStorage,
}

impl AudioFingerprinter {
    /// Creates a new AudioFingerprinter instance
    ///
    /// # Arguments
    /// * `redis_url` - URL of the Redis server for storage
    pub fn new(redis_url: &str) -> redis::RedisResult<Self> {
        let storage = crate::storage::RedisStorage::new(redis_url)?;
        Ok(AudioFingerprinter { storage })
    }

    /// Creates frequency bands for the fingerprinting algorithm
    /// These bands are optimized for human voice and music:
    /// - Bass: Low frequency sounds (20-300 Hz)
    /// - Low-mid: Voice fundamentals (300-800 Hz)
    /// - Mid: Most important for voice (800-3000 Hz)
    /// - High-mid: Voice harmonics (3000-5000 Hz)
    /// - Treble: High frequencies (5000-8000 Hz)
    /// - Presence: Very high frequencies (8000+ Hz)
    fn create_frequency_bands(&self, fft_size: usize, sample_rate: u32) -> FrequencyBands {
        let freq_resolution = sample_rate as f32 / fft_size as f32;

        FrequencyBands {
            bass: (
                self.freq_to_bin(20.0, freq_resolution),
                self.freq_to_bin(300.0, freq_resolution),
            ),
            low_mid: (
                self.freq_to_bin(300.0, freq_resolution),
                self.freq_to_bin(800.0, freq_resolution),
            ),
            mid: (
                self.freq_to_bin(800.0, freq_resolution),
                self.freq_to_bin(3000.0, freq_resolution),
            ),
            high_mid: (
                self.freq_to_bin(3000.0, freq_resolution),
                self.freq_to_bin(5000.0, freq_resolution),
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

    /// Converts a frequency to its corresponding FFT bin index
    fn freq_to_bin(&self, freq: f32, freq_resolution: f32) -> usize {
        (freq / freq_resolution).round() as usize
    }

    /// Generates fingerprints from audio data
    ///
    /// # Process
    /// 1. Process audio in overlapping windows
    /// 2. Convert each window to frequency domain using FFT
    /// 3. Extract significant peaks in each frequency band
    /// 4. Create hashes from peak combinations
    ///
    /// # Arguments
    /// * `audio_data` - Vector of audio samples
    /// * `sample_rate` - Sample rate in Hz
    ///
    /// # Returns
    /// Vector of fingerprint hashes
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

    /// Computes the magnitude spectrum of a window using FFT
    /// Applies a Hamming window to reduce spectral leakage
    fn compute_spectrum(&self, window: &[f32], fft: &dyn rustfft::Fft<f32>) -> Vec<f32> {
        let mut buffer: Vec<Complex<f32>> = window
            .iter()
            .map(|&x| Complex::new(x * self.hamming_window(window.len()), 0.0))
            .collect();

        fft.process(&mut buffer);

        // Convert to magnitude spectrum
        buffer
            .iter()
            .take(buffer.len() / 2)
            .map(|c| c.norm())
            .collect()
    }

    /// Applies a Hamming window to reduce spectral leakage
    fn hamming_window(&self, n: usize) -> f32 {
        0.54 - 0.46 * (2.0 * std::f32::consts::PI / n as f32).cos()
    }

    /// Extracts significant peaks from the spectrum
    ///
    /// # Process
    /// 1. Process each frequency band separately
    /// 2. Use different thresholds and peak counts for each band
    /// 3. Apply local peak detection with a sliding window
    ///
    /// # Returns
    /// Vector of (frequency_bin, amplitude, band_name) tuples
    fn extract_peaks(&self, spectrum: &[f32], sample_rate: u32) -> Vec<(usize, f32, String)> {
        let bands = self.create_frequency_bands(spectrum.len() * 2, sample_rate);
        let mut peaks = Vec::new();

        // Configure peak detection for each band
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

            // Use a sliding window for peak detection
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

    /// Converts peaks to robust hashes
    ///
    /// # Hash Structure (64 bits)
    /// - Band ID (6 bits)
    /// - Frequency difference (16 bits)
    /// - Amplitude ratio (8 bits)
    /// - Frequency sum (16 bits)
    ///
    /// This structure makes the hashes robust to:
    /// - Time shifts (using frequency differences)
    /// - Volume changes (using amplitude ratios)
    /// - Frequency shifts (using band information)
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

        // Generate hashes from peak combinations
        for (band_name, band_peaks) in &band_groups {
            let mut sorted_peaks = band_peaks.clone();
            sorted_peaks.sort_by_key(|&(freq, _)| freq);

            for i in 0..sorted_peaks.len() {
                for j in (i + 1)..sorted_peaks.len() {
                    let (freq1, amp1) = sorted_peaks[i];
                    let (freq2, amp2) = sorted_peaks[j];

                    // Include amplitude information in the hash
                    let amp_ratio = (amp1 / amp2 * 100.0) as u8;

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

    /// Converts band name to a unique ID
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

    /// Stores a song's fingerprints in Redis
    pub fn store_song(
        &self,
        song_info: &SongInfo,
        audio_data: &[f32],
        sample_rate: u32,
    ) -> redis::RedisResult<()> {
        let fingerprints = self.generate_fingerprint(audio_data, sample_rate);
        self.storage.store_song(song_info, &fingerprints)
    }

    /// Searches for a song matching the given audio clip
    pub fn search_song(
        &self,
        audio_clip: &[f32],
        sample_rate: u32,
    ) -> redis::RedisResult<Vec<(SongInfo, f32)>> {
        let fingerprints = self.generate_fingerprint(audio_clip, sample_rate);
        self.storage.search_song(&fingerprints)
    }
}
