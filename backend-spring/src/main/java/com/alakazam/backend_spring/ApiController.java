package com.alakazam.backend_spring;

import com.alakazam.backend_spring.service.AudioFingerprintService;
import com.alakazam.backend_spring.fingerprinter.Fingerprinter;
import com.alakazam.backend_spring.model.Song;
import org.springframework.beans.factory.annotation.Autowired;
import org.springframework.web.bind.annotation.GetMapping;
import org.springframework.web.bind.annotation.RestController;

import java.util.List;

@RestController
public class ApiController {
    
    @Autowired
    private AudioFingerprintService audioService;
    
    @Autowired
    private Fingerprinter fingerprinter;

    @GetMapping("/")
    public String index() {
        return "Greetings from Spring Boot!";
    } 

    @GetMapping("/haiii")
    public String hallloo() {
        return "haiii :3";
    }
    
    @GetMapping("/test-store")
    public String testStore() {
        try {
            // Generate some dummy audio data for testing
            float[] audioData = new float[44100]; // 1 second of silence
            for (int i = 0; i < audioData.length; i++) {
                audioData[i] = (float) Math.sin(2 * Math.PI * 440 * i / 44100); // 440Hz tone
            }
            
            Song song = audioService.storeSong("Test Song", "Test Artist", "Rock", audioData, 44100);
            return "Stored song with ID: " + song.getId();
        } catch (Exception e) {
            System.out.println(e);
            return "Error: " + e.getMessage();
        }
    }
    
    @GetMapping("/test-search")
    public String testSearch() {
        try {
            // Generate same dummy audio for search
            float[] audioData = new float[44100];
            for (int i = 0; i < audioData.length; i++) {
                audioData[i] = (float) Math.sin(2 * Math.PI * 440 * i / 44100);
            }
            
            List<AudioFingerprintService.MatchResult> results = audioService.searchSong(audioData, 44100);
            
            if (results.isEmpty()) {
                return "No matches found";
            } else {
                AudioFingerprintService.MatchResult best = results.get(0);
                return "Found match: " + best.getSong().getTitle() + " by " + best.getSong().getArtist() + 
                       " (confidence: " + best.getConfidence() + ")";
            }
        } catch (Exception e) {
            return "Error: " + e.getMessage();
        }
    }
    
    @GetMapping("/songs")
    public List<Song> getAllSongs() {
        return audioService.getAllSongs(0, 10);
    }

    @GetMapping("/test-jni")
    public String testJni() {
        try {
            return fingerprinter.testConnection();
        } catch (Exception e) {
            return "JNI Test Failed: " + e.getMessage();
        }
    }

    @GetMapping("/test-store-wav")
    public String testStoreWav() {
        try {
            // Use a hardcoded path to your test WAV file
            String wavPath = "src/main/resources/505.wav";
            
            Song song = audioService.storeSongFromWav("Test WAV Song", "Test Artist", "Rock", wavPath);
            return "Stored WAV song with ID: " + song.getId();
        } catch (Exception e) {
            System.out.println(e);
            return "Error: " + e.getMessage();
        }
    }

    @GetMapping("/test-load-wav")
    public String testLoadWav() {
        try {
            String wavPath = "src/main/resources/505.wav";
            Fingerprinter.AudioData audioData = fingerprinter.loadAudioFromWavFile(wavPath);
            
            return String.format("Loaded WAV: %d samples, %.2f seconds, %d Hz", 
                audioData.getSampleCount(), 
                audioData.getDuration(), 
                audioData.getSampleRate());
        } catch (Exception e) {
            return "Error loading WAV: " + e.getMessage();
        }
    }
}
