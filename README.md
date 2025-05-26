# EXVS2 Audio Editor

A specialized audio editor for handling NUS3AUDIO format files from EXVS2 (Extreme VS 2) game.

![Version](https://img.shields.io/badge/version-0.4.0-blue)

## Overview

EXVS2 Audio Editor is a tool designed to help you work with NUS3AUDIO format audio files. It allows you to extract, play, replace, and export audio files contained within NUS3AUDIO containers, which are commonly used in Bandai Namco's Extreme VS 2 game.

![preview](/git_images/preview.png)

## Features

- **Audio File Management**: Open NUS3AUDIO files and view contained audio tracks
- **Audio Playback**: Built-in audio player for previewing tracks
- **Audio Extraction**: Export audio tracks to WAV format
- **Audio Replacement**: Replace audio tracks with your own audio files
- **Loop Point Processing**: Add loop points to audio files for seamless looping
- **Search & Filter**: Easily find specific audio tracks within large containers

## System Requirements

- **Windows**: Windows 10 or newer recommended

## Installation

1. Download the latest release from the [Releases](https://github.com/your-username/exvs2-audio-editor/releases) page
2. Extract the ZIP file to a location of your choice
3. Run the `exvs2_audio_editor.exe` file

## Usage

### Opening NUS3AUDIO Files

1. Click the "Add File" button in the file list panel
2. Select a NUS3AUDIO file from your computer
3. The file will be loaded and its contents displayed in the main area

### Playing Audio Tracks

1. Select an audio track from the list
2. Use the built-in audio player controls to play, pause, and adjust volume

### Exporting Audio Tracks

1. Select one or multiple audio tracks
2. Click the "Export" button
3. Choose an output directory
4. The selected tracks will be exported as WAV files

### Replacing Audio Tracks

1. Select an audio track you want to replace
2. Click the "Replace" button
3. Choose a replacement audio file from your computer
4. The audio track will be replaced in memory (changes aren't saved until you explicitly save the file)

### Saving Changes

1. After making your desired changes, click the "Save" button
2. Choose where to save the modified NUS3AUDIO file
3. All changes will be written to the new file (the original file is not modified)

## Tools Used

This application uses the following tools to process audio files:

- **vgmstream-cli**: For decoding and encoding various game audio formats
- **rodio**: For audio playback

## Development

### Building from Source

Prerequisites:
- Rust 1.81 or newer
- Cargo package manager

```bash
# Clone the repository
git clone https://github.com/your-username/exvs2-audio-editor.git
cd exvs2-audio-editor

# Build the application
cargo build --release

# Run the application
cargo run --release
```

## License

This project is licensed under both:
- MIT License
- Apache License 2.0

See [LICENSE-MIT](LICENSE-MIT) and [LICENSE-APACHE](LICENSE-APACHE) for details.

## Acknowledgements

- [egui](https://github.com/emilk/egui) - Immediate mode GUI library for Rust
- [nus3audio](https://crates.io/crates/nus3audio) - Library for handling NUS3AUDIO format files
- [vgmstream](https://github.com/vgmstream/vgmstream) - Audio stream player for video games

## AI Assistance

Parts of the code in this project were generated with the assistance of AI.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
