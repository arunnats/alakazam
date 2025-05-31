package com.alakazam.backend_spring.service;

import com.alakazam.backend_spring.fingerprinter.Fingerprinter;
import com.alakazam.backend_spring.model.Song;

import lombok.Getter;

import org.springframework.beans.factory.annotation.Autowired;
import org.springframework.data.redis.core.RedisTemplate;
import org.springframework.stereotype.Service;

import java.util.*;
import java.util.stream.Collectors;

@Service
public class AudioFingerprintService {
    @Autowired
    private RedisTemplate<String, Object> redisTemplate;
    
    @Autowired
    private Fingerprinter fingerprinter;

    public Song storeSongFromWav(String title, String artist, String genre, String wavFilePath) {
        // Load audio using Rust
        Fingerprinter.AudioData audioData = fingerprinter.loadAudioFromWavFile(wavFilePath);
        
        // Generate fingerprint
        Fingerprinter.SongFingerprint fingerprint = 
            fingerprinter.generateSongFingerprintObj(audioData.getAudioData(), audioData.getSampleRate());
        
        // Store song (rest of your existing logic)
        Long songId = redisTemplate.opsForValue().increment("song_counter");
        
        Song song = new Song(title, artist, genre, 
            fingerprint.getMetadata().getDuration(), 
            fingerprint.getMetadata().getSampleRate(), 
            fingerprint.getMetadata().getHashCount());
        song.setId(songId);
        
        // Store in Redis
        String songKey = "song:" + songId;
        redisTemplate.opsForValue().set(songKey, song);
        
        for (long hash : fingerprint.getHashes()) {
            String hashKey = "hash:" + hash;
            redisTemplate.opsForSet().add(hashKey, songId);
        }
        
        redisTemplate.opsForZSet().add("songs:all", songId, songId);
        createSearchIndexes(songId, title, artist, genre);
        
        System.out.println("Stored song '" + title + "' from WAV file with ID: " + songId);
        return song;
    }
    
    // Store song fingerprint in Redis (same logic as Rust)
    public Song storeSong(String title, String artist, String genre, float[] audioData, int sampleRate) {
        // Generate fingerprint using Rust
        Fingerprinter.SongFingerprint fingerprint = 
            fingerprinter.generateSongFingerprintObj(audioData, sampleRate);
        
        // Generate unique song ID
        Long songId = redisTemplate.opsForValue().increment("song_counter");
        
        // Create song object
        Song song = new Song(title, artist, genre, 
            fingerprint.getMetadata().getDuration(), 
            fingerprint.getMetadata().getSampleRate(), 
            fingerprint.getMetadata().getHashCount());
        song.setId(songId);
        
        // Store song metadata
        String songKey = "song:" + songId;
        redisTemplate.opsForValue().set(songKey, song);
        
        // Store fingerprint hashes (same as Rust: hash:12345 -> Set{songId})
        for (long hash : fingerprint.getHashes()) {
            String hashKey = "hash:" + hash;
            redisTemplate.opsForSet().add(hashKey, songId);
        }
        
        // Add to master song list
        redisTemplate.opsForZSet().add("songs:all", songId, songId);
        
        // Create search indexes
        createSearchIndexes(songId, title, artist, genre);
        
        System.out.println("Stored song '" + title + "' by '" + artist + "' with ID: " + songId);
        return song;
    }
    
    // Search for song using audio clip (same logic as Rust)
    public List<MatchResult> searchSong(float[] audioClip, int sampleRate) {
        // Generate query fingerprint
        Fingerprinter.QueryFingerprint queryFingerprint = 
            fingerprinter.generateQueryFingerprintObj(audioClip, sampleRate);
        
        Map<Long, Integer> songMatches = new HashMap<>();
        
        // Count matches for each song (same as Rust)
        for (long hash : queryFingerprint.getHashes()) {
            String hashKey = "hash:" + hash;
            Set<Object> songIds = redisTemplate.opsForSet().members(hashKey);
            
            if (songIds != null) {
                for (Object songIdObj : songIds) {
                    Long songId = (Long) songIdObj;
                    songMatches.merge(songId, 1, Integer::sum);
                }
            }
        }
        
        // Calculate confidence and return results
        return calculateConfidenceScores(songMatches, queryFingerprint.getHashes().length);
    }
    
    // Get all songs with pagination
    public List<Song> getAllSongs(int page, int size) {
        long start = (long) page * size;
        long end = start + size - 1;
        
        Set<Object> songIds = redisTemplate.opsForZSet().range("songs:all", start, end);
        
        return songIds.stream()
            .map(id -> (Song) redisTemplate.opsForValue().get("song:" + id))
            .filter(Objects::nonNull)
            .collect(Collectors.toList());
    }
    
    // Search songs by text
    public List<Song> searchSongsByText(String query) {
        Set<Long> matchingSongIds = new HashSet<>();
        String[] words = query.toLowerCase().split("\\s+");
        
        for (String word : words) {
            // Search in titles
            Set<Object> titleMatches = redisTemplate.opsForSet().members("title:" + word);
            if (titleMatches != null) {
                titleMatches.forEach(id -> matchingSongIds.add((Long) id));
            }
            
            // Search in artists
            Set<Object> artistMatches = redisTemplate.opsForSet().members("artist:" + word);
            if (artistMatches != null) {
                artistMatches.forEach(id -> matchingSongIds.add((Long) id));
            }
        }
        
        return matchingSongIds.stream()
            .map(id -> (Song) redisTemplate.opsForValue().get("song:" + id))
            .filter(Objects::nonNull)
            .collect(Collectors.toList());
    }
    
    // Get total song count
    public long getTotalSongCount() {
        return redisTemplate.opsForZSet().count("songs:all", Double.NEGATIVE_INFINITY, Double.POSITIVE_INFINITY);
    }
    
    // Helper method to create search indexes
    private void createSearchIndexes(Long songId, String title, String artist, String genre) {
        // Artist index
        redisTemplate.opsForSet().add("artist:" + artist.toLowerCase(), songId);
        
        // Title word index
        for (String word : title.toLowerCase().split("\\s+")) {
            if (word.length() > 2) {
                redisTemplate.opsForSet().add("title:" + word, songId);
            }
        }
        
        // Genre index
        if (genre != null && !genre.isEmpty()) {
            redisTemplate.opsForSet().add("genre:" + genre.toLowerCase(), songId);
        }
    }
    
    // Helper method to calculate confidence scores
    private List<MatchResult> calculateConfidenceScores(Map<Long, Integer> songMatches, int totalQueryHashes) {
        return songMatches.entrySet().stream()
            .map(entry -> {
                Long songId = entry.getKey();
                Integer matchCount = entry.getValue();
                float confidence = (float) matchCount / totalQueryHashes;
                
                Song song = (Song) redisTemplate.opsForValue().get("song:" + songId);
                return new MatchResult(song, confidence, matchCount, totalQueryHashes);
            })
            .filter(result -> result.getSong() != null)
            .sorted((a, b) -> Float.compare(b.getConfidence(), a.getConfidence()))
            .collect(Collectors.toList());
    }
    
    @Getter
    public static class MatchResult {
        private Song song;
        private float confidence;
        private int matchCount;
        private int totalQueryHashes;
        
        public MatchResult(Song song, float confidence, int matchCount, int totalQueryHashes) {
            this.song = song;
            this.confidence = confidence;
            this.matchCount = matchCount;
            this.totalQueryHashes = totalQueryHashes;
        }
    }
}
