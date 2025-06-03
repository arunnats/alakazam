package com.alakazam.backend_spring;

import com.alakazam.backend_spring.service.AudioFingerprintService;
import com.alakazam.backend_spring.data.Search;
import com.alakazam.backend_spring.data.Search.MatchResultDetailed;
import com.alakazam.backend_spring.fingerprinter.Fingerprinter;
import com.alakazam.backend_spring.model.Song;
import org.springframework.beans.factory.annotation.Autowired;
import org.springframework.web.bind.annotation.GetMapping;
import org.springframework.web.bind.annotation.PostMapping;
import org.springframework.web.bind.annotation.RequestBody;
import org.springframework.web.bind.annotation.RestController;

import java.util.List;
import java.util.Map;

@RestController
public class ApiController {
    
    @Autowired
    private AudioFingerprintService audioService;
    
    @Autowired
    private Fingerprinter fingerprinter;

    @Autowired
    private Search search;

    @GetMapping("/")
    public String index() {
        return "Greetings from Spring Boot!";
    } 

    @GetMapping("/haiii")
    public String hallloo() {
        return "haiii :3";
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
            String wavPath = "src/main/resources/song2.wav";
            
            Song song = audioService.storeSongFromWav("Like him", "taylr", "genre", wavPath);
            return "Stored WAV song with ID: " + song.getId();
        } catch (Exception e) {
            System.out.println(e);
            return "Error: " + e.getMessage();
        }
    }

    @GetMapping("/test-search-wav")
    public String testSearchWav() {
        try {
            // Load the entire WAV file as the search query (no splitting)
            String wavPath = "src/main/resources/in2.wav";
            Fingerprinter.AudioData audioData = fingerprinter.loadAudioFromWavFile(wavPath);
            
            // Generate query fingerprint from the ENTIRE audio file
            Fingerprinter.QueryFingerprint queryFingerprint = 
                fingerprinter.generateQueryFingerprintObj(audioData.getAudioData(), audioData.getSampleRate());
            
            // Search using the same logic as Rust implementation
            List<MatchResultDetailed> results = search.searchRedis(queryFingerprint.getHashes());

            System.out.println(results);
            
            if (results.isEmpty()) {
                System.out.println("nahi");
                return "No matches found";
            } else {
                StringBuilder response = new StringBuilder();
                response.append("Found ").append(results.size()).append(" matches:\n");
                
                for (int i = 0; i < Math.min(3, results.size()); i++) {
                    MatchResultDetailed result = results.get(i);
                    response.append(String.format("%d. %s by %s (confidence: %.3f)\n", 
                        i + 1,
                        result.getSong().getTitle(), 
                        result.getSong().getArtist(),
                        result.getConfidence()));
                }
                System.out.println(response);
                return response.toString();
            }
        } catch (Exception e) {
            return "Error: " + e.getMessage();
        }
    }

    @PostMapping("/search")
    public List<Search.MatchResultDetailed> search(@RequestBody Map<String, Object> body) {
        List<String> hashes = (List<String>) body.get("hashes");
        long[] hashArray = hashes.stream().mapToLong(Long::parseLong).toArray();
        return search.searchRedis(hashArray);
    }
}
