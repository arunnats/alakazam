use crate::models::SongInfo;
use redis::{Client, Commands, RedisResult};

/// Manages storage and retrieval of song fingerprints in Redis
/// This struct handles all Redis operations including:
/// - Storing song metadata and fingerprints
/// - Searching for matching songs
/// - Managing the song ID counter
pub struct RedisStorage {
    client: Client,
}

impl RedisStorage {
    /// Creates a new RedisStorage instance
    ///
    /// # Arguments
    /// * `redis_url` - URL of the Redis server (e.g., "redis://127.0.0.1/")
    pub fn new(redis_url: &str) -> RedisResult<Self> {
        let client = Client::open(redis_url)?;
        Ok(RedisStorage { client })
    }

    /// Stores a song's metadata and fingerprints in Redis
    ///
    /// # Storage Structure
    /// - Song metadata is stored as JSON in "song:{id}" keys
    /// - Fingerprints are stored in sets at "hash:{hash}" keys
    /// - Each hash set contains IDs of songs that have that fingerprint
    ///
    /// # Arguments
    /// * `song_info` - Song metadata (name, artist)
    /// * `fingerprints` - Vector of fingerprint hashes
    pub fn store_song(&self, song_info: &SongInfo, fingerprints: &[u64]) -> RedisResult<()> {
        let mut conn = self.client.get_connection()?;

        // Generate unique song ID using Redis counter
        let song_id: u64 = conn.incr("song_counter", 1)?;

        // Store song metadata as JSON
        let song_key = format!("song:{}", song_id);
        let song_json = serde_json::to_string(song_info).unwrap();
        let _: () = conn.set(&song_key, song_json)?;

        // Store fingerprint mappings
        // Each hash points to a set of song IDs that contain that hash
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

    /// Searches for songs matching the given fingerprints
    ///
    /// # Search Process
    /// 1. For each fingerprint, find all songs that contain it
    /// 2. Count matches for each song
    /// 3. Calculate confidence scores based on:
    ///    - Number of unique matches
    ///    - Ratio of matches to total fingerprints
    ///    - Penalty for duplicate matches
    ///
    /// # Arguments
    /// * `query_fingerprints` - Vector of fingerprint hashes to search for
    ///
    /// # Returns
    /// Vector of (SongInfo, confidence) tuples, sorted by confidence
    pub fn search_song(&self, query_fingerprints: &[u64]) -> RedisResult<Vec<(SongInfo, f32)>> {
        let mut conn = self.client.get_connection()?;
        let mut song_matches: std::collections::HashMap<u64, (usize, Vec<u64>)> =
            std::collections::HashMap::new();

        // Count matches for each song with hash tracking
        for hash in query_fingerprints {
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
                    // Calculate confidence score based on:
                    // 1. Base confidence from unique matches
                    // 2. Penalty for duplicate matches
                    // 3. Minimum threshold to filter out weak matches
                    let total_hashes = query_fingerprints.len() as f32;
                    let unique_matches = matched_hashes.len() as f32;
                    let base_confidence = unique_matches / total_hashes;

                    // Apply penalty for low match counts
                    let match_ratio = match_count as f32 / unique_matches;
                    let match_penalty = if match_ratio > 2.0 {
                        0.8 // Heavy penalty for too many duplicate matches
                    } else if match_ratio > 1.5 {
                        0.9 // Medium penalty
                    } else {
                        1.0 // No penalty
                    };

                    // Apply minimum threshold
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
