"use client";
import React, { useRef, useState } from "react";

export default function Home() {
  const [recording, setRecording] = useState(false);
  const [audioUrl, setAudioUrl] = useState(null);
  const [audioBlob, setAudioBlob] = useState(null);
  const mediaRecorderRef = useRef(null);
  const audioChunksRef = useRef([]);

  async function onAudioReady(blob) {
    const arrayBuffer = await blob.arrayBuffer();
    const audioCtx = new AudioContext();
    const audioBuffer = await audioCtx.decodeAudioData(arrayBuffer);

    const float32 = audioBuffer.getChannelData(0); // mono channel
    const sampleRate = audioBuffer.sampleRate;

    // Load wasm + generate hashes
    const init = (await import("../../public/wasm/fingerprinter_rust.js"))
      .default;
    await init(); // initializes wasm
    const { generate_query_fingerprint_wasm } = await import(
      "../../public/wasm/fingerprinter_rust.js"
    );

    const audioBytes = await blobToBytes(blob);
    const hashes = await generate_query_fingerprint_wasm(
      audioBytes,
      sampleRate
    );

    console.log("Hashes:", hashes);

    searchByHashes(hashes)
      .then((results) => {
        console.log("Search results:", results);
        // Optionally, update state to display results in your UI
        // setSearchResults(results);
      })
      .catch((err) => {
        console.error("Error searching by hashes:", err);
      });
  }

  async function blobToBytes(blob) {
    const arrayBuffer = await blob.arrayBuffer();
    // Convert to Java-style signed bytes
    const int8Array = new Int8Array(arrayBuffer);
    return new Uint8Array(int8Array.map((b) => (b < 0 ? b + 256 : b)));
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
      const blob = new Blob(audioChunksRef.current, { type: "audio/webm" });
      setAudioBlob(blob);
      setAudioUrl(URL.createObjectURL(blob));
      await onAudioReady(blob);
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
