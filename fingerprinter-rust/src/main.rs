use fingerprinter_rust::{AudioFingerprinter, AudioLoader, SongInfo};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let fingerprinter = AudioFingerprinter::new("redis://127.0.0.1/")?;

    // Load and store songs
    let songs_to_store = [
        ("505.wav", "505", "Arctic Monkeys"),
        ("song2.wav", "LINKE HIM", "TYLER"),
        ("input.wav", "we fell in october", "girl in red"),
    ];

    for (file, name, singer) in songs_to_store {
        println!("Loading song: {}", file);
        let (audio_data, sample_rate) = AudioLoader::load_from_wav(file)?;

        let song_info = SongInfo {
            name: name.to_string(),
            singer: singer.to_string(),
        };

        println!("Storing song fingerprint...");
        fingerprinter.store_song(&song_info, &audio_data, sample_rate)?;
    }

    // Test files to search
    let test_files = [
        "record_out2.wav",
        "record_out1.wav",
        "in1.wav",
        "in2.wav",
        "in4.wav",
    ];

    for test_file in test_files {
        println!("\nSearching with: {}", test_file);
        let (audio_data, sample_rate) = AudioLoader::load_from_wav(test_file)?;

        println!("Searching for matches...");
        let results = fingerprinter.search_song(&audio_data, sample_rate)?;

        if results.is_empty() {
            println!("No matches found!");
        } else {
            println!("Found {} matches:", results.len());
            for (i, (song, confidence)) in results.iter().enumerate().take(5) {
                println!(
                    "  {}. {} by {} (confidence: {:.3})",
                    i + 1,
                    song.name,
                    song.singer,
                    confidence
                );
            }
        }
    }

    Ok(())
}
