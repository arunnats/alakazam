package com.alakazam.backend_spring.model;

import com.fasterxml.jackson.annotation.JsonProperty;
import java.time.Instant;

public class Song {
    private Long id;
    private String title;
    private String artist;
    private String genre;
    private float duration;
    private int sampleRate;
    private int hashCount;
    private Instant uploadDate;
    
    // Constructors
    public Song() {}
    
    public Song(String title, String artist, String genre, float duration, int sampleRate, int hashCount) {
        this.title = title;
        this.artist = artist;
        this.genre = genre;
        this.duration = duration;
        this.sampleRate = sampleRate;
        this.hashCount = hashCount;
        this.uploadDate = Instant.now();
    }
    
    // Getters and setters
    public Long getId() { return id; }
    public void setId(Long id) { this.id = id; }
    
    public String getTitle() { return title; }
    public void setTitle(String title) { this.title = title; }
    
    public String getArtist() { return artist; }
    public void setArtist(String artist) { this.artist = artist; }
    
    public String getGenre() { return genre; }
    public void setGenre(String genre) { this.genre = genre; }
    
    public float getDuration() { return duration; }
    public void setDuration(float duration) { this.duration = duration; }
    
    public int getSampleRate() { return sampleRate; }
    public void setSampleRate(int sampleRate) { this.sampleRate = sampleRate; }
    
    public int getHashCount() { return hashCount; }
    public void setHashCount(int hashCount) { this.hashCount = hashCount; }
    
    public Instant getUploadDate() { return uploadDate; }
    public void setUploadDate(Instant uploadDate) { this.uploadDate = uploadDate; }
}
