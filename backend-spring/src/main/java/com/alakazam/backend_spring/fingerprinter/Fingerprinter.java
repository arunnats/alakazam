package com.alakazam.backend_spring.fingerprinter;

import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.annotation.JsonProperty;
import org.springframework.stereotype.Component;
import java.io.*;
import java.nio.file.Files;
import java.nio.file.StandardCopyOption;

@Component
public class Fingerprinter {
    
    private static final ObjectMapper objectMapper = new ObjectMapper();
    
    static {
        try {
            loadNativeLibrary();
        } catch (Exception e) {
            throw new RuntimeException("Failed to load native library", e);
        }
    }
    
    private static void loadNativeLibrary() throws IOException {
        String libraryName = "libfingerprinter_rust.so";
        String osName = System.getProperty("os.name").toLowerCase();
        
        // Adjust library name based on OS
        if (osName.contains("windows")) {
            libraryName = "fingerprinter_rust.dll";
        } else if (osName.contains("mac")) {
            libraryName = "libfingerprinter_rust.dylib";
        }
        
        // Extract library from resources to temp file
        InputStream libraryStream = Fingerprinter.class.getClassLoader()
            .getResourceAsStream(libraryName);
        
        if (libraryStream == null) {
            throw new RuntimeException("Native library not found in resources: " + libraryName);
        }
        
        // Create temp file
        File tempLibrary = File.createTempFile("fingerprinter_rust", 
            osName.contains("windows") ? ".dll" : ".so");
        tempLibrary.deleteOnExit();
        
        // Copy library to temp file
        Files.copy(libraryStream, tempLibrary.toPath(), StandardCopyOption.REPLACE_EXISTING);
        libraryStream.close();
        
        // Load the library using absolute path
        System.load(tempLibrary.getAbsolutePath());
    }
    
    // Native method declarations
    public static native String generateSongFingerprint(byte[] audioData, int sampleRate);
    public static native String generateQueryFingerprint(byte[] audioData, int sampleRate);
    public static native String testFunc();
    public static native String loadAudioFromWav(String filePath);

    public AudioData loadAudioFromWavFile(String filePath) {
        try {
            String jsonResult = loadAudioFromWav(filePath);
            if (jsonResult == null) {
                throw new RuntimeException("Native function returned null");
            }
            return objectMapper.readValue(jsonResult, AudioData.class);
        } catch (Exception e) {
            throw new RuntimeException("Failed to load audio from WAV file", e);
        }
    }

    public String testConnection() {
        try {
            String result = testFunc();
            return result != null ? result : "null returned";
        } catch (Exception e) {
            return "Error: " + e.getMessage();
        }
    }

    public SongFingerprint generateSongFingerprintObj(float[] audioData, int sampleRate) {
        try {
            byte[] audioBytes = floatArrayToByteArray(audioData);
            String jsonResult = generateSongFingerprint(audioBytes, sampleRate);
            if (jsonResult == null) {
                throw new RuntimeException("Native function returned null");
            }
            return objectMapper.readValue(jsonResult, SongFingerprint.class);
        } catch (Exception e) {
            throw new RuntimeException("Failed to generate song fingerprint", e);
        }
    }
    
    public QueryFingerprint generateQueryFingerprintObj(float[] audioData, int sampleRate) {
        try {
            byte[] audioBytes = floatArrayToByteArray(audioData);
            String jsonResult = generateQueryFingerprint(audioBytes, sampleRate);
            if (jsonResult == null) {
                throw new RuntimeException("Native function returned null");
            }
            return objectMapper.readValue(jsonResult, QueryFingerprint.class);
        } catch (Exception e) {
            throw new RuntimeException("Failed to generate query fingerprint", e);
        }
    }
    
    // Helper method to convert float array to byte array (little-endian)
    private byte[] floatArrayToByteArray(float[] floats) {
        byte[] bytes = new byte[floats.length * 4];
        for (int i = 0; i < floats.length; i++) {
            int bits = Float.floatToIntBits(floats[i]);
            bytes[i * 4] = (byte) (bits & 0xFF);
            bytes[i * 4 + 1] = (byte) ((bits >> 8) & 0xFF);
            bytes[i * 4 + 2] = (byte) ((bits >> 16) & 0xFF);
            bytes[i * 4 + 3] = (byte) ((bits >> 24) & 0xFF);
        }
        return bytes;
    }
    
    // Data classes matching your Rust structs
    public static class SongFingerprint {
        @JsonProperty("hashes")
        public long[] hashes;
        
        @JsonProperty("metadata")
        public SongMetadata metadata;
        
        // Default constructor for Jackson
        public SongFingerprint() {}
        
        // Getters and setters
        public long[] getHashes() { return hashes; }
        public void setHashes(long[] hashes) { this.hashes = hashes; }
        
        public SongMetadata getMetadata() { return metadata; }
        public void setMetadata(SongMetadata metadata) { this.metadata = metadata; }
    }
    
    public static class QueryFingerprint {
        @JsonProperty("hashes")
        public long[] hashes;
        
        @JsonProperty("duration")
        public float duration;
        
        // Default constructor for Jackson
        public QueryFingerprint() {}
        
        // Getters and setters
        public long[] getHashes() { return hashes; }
        public void setHashes(long[] hashes) { this.hashes = hashes; }
        
        public float getDuration() { return duration; }
        public void setDuration(float duration) { this.duration = duration; }
    }
    
    public static class SongMetadata {
        @JsonProperty("duration")
        public float duration;
        
        @JsonProperty("sample_rate")
        public int sampleRate;
        
        @JsonProperty("hash_count")
        public int hashCount;
        
        // Default constructor for Jackson
        public SongMetadata() {}
        
        // Getters and setters
        public float getDuration() { return duration; }
        public void setDuration(float duration) { this.duration = duration; }
        
        public int getSampleRate() { return sampleRate; }
        public void setSampleRate(int sampleRate) { this.sampleRate = sampleRate; }
        
        public int getHashCount() { return hashCount; }
        public void setHashCount(int hashCount) { this.hashCount = hashCount; }
    }

    public static class AudioData {
        @JsonProperty("audio_data")
        public float[] audioData;
        
        @JsonProperty("sample_rate")
        public int sampleRate;
        
        @JsonProperty("duration")
        public float duration;
        
        @JsonProperty("sample_count")
        public int sampleCount;
        
        // Default constructor
        public AudioData() {}
        
        // Getters and setters
        public float[] getAudioData() { return audioData; }
        public void setAudioData(float[] audioData) { this.audioData = audioData; }
        
        public int getSampleRate() { return sampleRate; }
        public void setSampleRate(int sampleRate) { this.sampleRate = sampleRate; }
        
        public float getDuration() { return duration; }
        public void setDuration(float duration) { this.duration = duration; }
        
        public int getSampleCount() { return sampleCount; }
        public void setSampleCount(int sampleCount) { this.sampleCount = sampleCount; }
    }    
    
    // Utility method to test if the library is loaded correctly
    public boolean isLibraryLoaded() {
        try {
            // Test with dummy data
            byte[] testData = new byte[4];
            generateSongFingerprint(testData, 44100);
            return true;
        } catch (Exception e) {
            return false;
        }
    }
}