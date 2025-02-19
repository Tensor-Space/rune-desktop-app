use std::path::PathBuf;

pub fn get_temp_recording_path() -> PathBuf {
    std::env::temp_dir().join("rune_recording.wav")
}
