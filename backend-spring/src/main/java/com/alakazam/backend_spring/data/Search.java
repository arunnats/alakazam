package com.alakazam.backend_spring.data;

import com.alakazam.backend_spring.model.Song;
import com.fasterxml.jackson.databind.ObjectMapper;

import org.springframework.beans.factory.annotation.Autowired;
import org.springframework.data.redis.core.RedisTemplate;
import org.springframework.stereotype.Component;

import java.util.*;
import lombok.Data;
import lombok.AllArgsConstructor;

@Component
public class Search {
    
    @Autowired
    private RedisTemplate<String, Object> redisTemplate;

    private ObjectMapper objectMapper = new ObjectMapper();
    
    public List<MatchResultDetailed> searchRedis(long[] queryFingerprints) {
        Map<Long, MatchData> songMatches = new HashMap<>();
        
        // Count matches for each song
        for (long hash : queryFingerprints) {
            String hashKey = "hash:" + hash;
            Set<Object> songIds = redisTemplate.opsForSet().members(hashKey);
            
            if (songIds != null) {
                for (Object songIdObj : songIds) {
                    Long songId = Long.valueOf(songIdObj.toString());
                    MatchData matchData = songMatches.computeIfAbsent(songId, k -> new MatchData());
                    matchData.matchCount++;
                    matchData.matchedHashes.add(hash);
                }                
            }
        }

        System.out.println("teastcc\n");
        
        // Calculate confidence scores (same logic as Rust)
        List<MatchResultDetailed> results = new ArrayList<>();
        float totalHashes = queryFingerprints.length;
        
        for (Map.Entry<Long, MatchData> entry : songMatches.entrySet()) {
            Long songId = entry.getKey();
            MatchData matchData = entry.getValue();

            String songKey = "song:" + songId;
            Object songObj = redisTemplate.opsForValue().get(songKey);

            Song song = null;
            if (songObj instanceof Song) {
                song = (Song) songObj;
            } else if (songObj instanceof LinkedHashMap) {
                song = objectMapper.convertValue(songObj, Song.class);
            }

            if (song != null) {
                float uniqueMatches = matchData.matchedHashes.size();
                float baseConfidence = uniqueMatches / totalHashes;
                
                // Apply penalty for duplicate matches (same as Rust)
                float matchRatio = matchData.matchCount / uniqueMatches;
                float matchPenalty = matchRatio > 2.0f ? 0.8f : (matchRatio > 1.5f ? 0.9f : 1.0f);
                
                // Apply minimum threshold (same as Rust)
                float confidence = baseConfidence < 0.1f ? 0.0f : baseConfidence * matchPenalty;
                
                if (confidence > 0.0f) {
                    results.add(new MatchResultDetailed(song, confidence, matchData.matchCount, (int) uniqueMatches, queryFingerprints.length));
                }
            }
        }

        System.out.println("teastdd\n");
        
        // Sort by confidence (highest first)
        results.sort((a, b) -> Float.compare(b.getConfidence(), a.getConfidence()));

        System.out.println(results);
        return results;
    }
    
    // Helper classes
    private static class MatchData {
        int matchCount = 0;
        Set<Long> matchedHashes = new HashSet<>();
    }
    
    @Data
    @AllArgsConstructor
    public static class MatchResultDetailed {
        private Song song;
        private float confidence;
        private int matchCount;
        private int uniqueMatches;
        private int totalQueryHashes;
    }        
}