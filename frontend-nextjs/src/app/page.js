"use client";
import React, { useRef, useState } from "react";
import { FFmpeg } from "@ffmpeg/ffmpeg";
import { fetchFile } from "@ffmpeg/util";

async function convertWebmToWav(blob) {
  const ffmpeg = new FFmpeg();

  // Load FFmpeg
  await ffmpeg.load();

  // Write the input file
  await ffmpeg.writeFile("input.webm", await fetchFile(blob));

  // Convert webm to wav
  await ffmpeg.exec([
    "-i",
    "input.webm",
    "-ar",
    "44100",
    "-ac",
    "1",
    "output.wav",
  ]);

  // Read the output file
  const wavData = await ffmpeg.readFile("output.wav");
  return new Blob([wavData], { type: "audio/wav" });
}

export default function Home() {
  const [recording, setRecording] = useState(false);
  const [audioUrl, setAudioUrl] = useState(null);
  const [audioBlob, setAudioBlob] = useState(null);
  const mediaRecorderRef = useRef(null);
  const audioChunksRef = useRef([]);

  async function onAudioReady(blob) {
    try {
      // Convert blob directly to Uint8Array (no signed conversion needed)
      const bytes = new Uint8Array(await blob.arrayBuffer());

      // Get sample rate from audio file
      const audioContext = new (window.AudioContext ||
        window.webkitAudioContext)();
      const audioBuffer = await audioContext.decodeAudioData(
        await blob.arrayBuffer()
      );
      const sampleRate = audioBuffer.sampleRate;

      // Initialize WASM
      const wasmModule = await import(
        "../../public/wasm/fingerprinter_rust.js"
      );
      await wasmModule.default(); // Initialize the WASM module

      // Generate hashes using the unified function
      const result = await wasmModule.create_hashes_from_wav_wasm(bytes);

      // Extract hashes from result
      const hashes = result.hashes;
      console.log(`Generated ${hashes.length} hashes`);
      console.log("Sample hashes:", hashes.slice(0, 5));

      // Search with hashes
      const searchResults = await searchByHashes(hashes);
      console.log("Search results:", searchResults);
    } catch (error) {
      console.error("Audio processing failed:", error);
      // Handle error in UI
    }
  }

  const startRecording = async () => {
    const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
    const mediaRecorder = new window.MediaRecorder(stream);
    mediaRecorderRef.current = mediaRecorder;
    audioChunksRef.current = [];

    mediaRecorder.ondataavailable = (event) => {
      if (event.data.size > 0) {
        audioChunksRef.current.push(event.data);
      }
    };

    mediaRecorder.onstop = async () => {
      const webmBlob = new Blob(audioChunksRef.current, { type: "audio/webm" });
      const wavBlob = await convertWebmToWav(webmBlob);

      setAudioBlob(wavBlob);
      setAudioUrl(URL.createObjectURL(wavBlob));
      await onAudioReady(wavBlob);
    };

    mediaRecorder.start();
    setRecording(true);
  };

  const stopRecording = () => {
    if (mediaRecorderRef.current) {
      mediaRecorderRef.current.stop();
      setRecording(false);
    }
  };

  const handleUpload = async (e) => {
    const file = e.target.files[0];
    if (file && file.type === "audio/wav") {
      setAudioBlob(file);
      setAudioUrl(URL.createObjectURL(file));
      await onAudioReady(file);
    } else {
      alert("Please upload a .wav file.");
    }
  };

  async function searchByHashes(hashes) {
    const response = await fetch("http://localhost:8080/search", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ hashes }),
    });
    return await response.json();
  }

  return (
    <div>
      <h2>Audio Recorder</h2>
      <div>
        {!recording ? (
          <button onClick={startRecording}>Start Recording</button>
        ) : (
          <button onClick={stopRecording}>Stop Recording</button>
        )}
      </div>
      <div>
        <input
          type="file"
          accept="audio/wav"
          onChange={handleUpload}
          style={{ marginTop: "1em" }}
        />
      </div>
      {audioUrl && (
        <div style={{ marginTop: "1em" }}>
          <audio src={audioUrl} controls />
          <br />
          <a href={audioUrl} download="recording.wav">
            Download WAV
          </a>
        </div>
      )}
    </div>
  );
}
