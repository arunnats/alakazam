package com.alakazam.backend_spring.model;

import java.time.Instant;
import lombok.Data;
import lombok.NoArgsConstructor;
import lombok.AllArgsConstructor;

@Data
@NoArgsConstructor
@AllArgsConstructor
public class Song {
    private Long id;
    private String title;
    private String artist;
    private String genre;
    private float duration;
    private int sampleRate;
    private int hashCount;
    private String uploadDate;
    
    // Constructors
    public Song(String title, String artist, String genre, float duration, int sampleRate, int hashCount) {
        this.title = title;
        this.artist = artist;
        this.genre = genre;
        this.duration = duration;
        this.sampleRate = sampleRate;
        this.hashCount = hashCount;
        this.uploadDate = Instant.now().toString(); 
    }
}
