#pragma once

#include <Arduino.h>

// I2S Audio subsystem for voice control and text-to-speech
// Supports INMP441 I2S microphone and MAX98357A I2S DAC amplifier

// I2S Microphone (INMP441) pin configuration
#define I2S_MIC_SCK    26   // Serial Clock (BCLK)
#define I2S_MIC_WS     32   // Word Select (LRCLK)
#define I2S_MIC_SD     33   // Serial Data (DOUT)

// I2S Speaker (MAX98357A) pin configuration
#define I2S_SPK_BCK    27   // Bit Clock
#define I2S_SPK_WS     14   // Word Select (LRCLK)
#define I2S_SPK_DOUT   12   // Data Out (DIN on amp)

// Audio parameters
#define AUDIO_SAMPLE_RATE     16000
#define AUDIO_BITS_PER_SAMPLE 16
#define AUDIO_CHANNELS        1

// Recording buffer: 4 seconds at 16kHz 16-bit = 128KB
#define MAX_RECORD_SECONDS    4
#define RECORD_BUFFER_SIZE    (AUDIO_SAMPLE_RATE * sizeof(int16_t) * MAX_RECORD_SECONDS)

// VAD (Voice Activity Detection) thresholds
#define VAD_ENERGY_THRESHOLD  500     // RMS energy threshold to detect speech
#define VAD_SILENCE_FRAMES    8000    // ~0.5s of silence to stop recording
#define VAD_MIN_SPEECH_FRAMES 4800    // ~0.3s minimum speech to be valid

namespace Audio {
    void init();
    void update(uint32_t deltaMs);

    // Recording
    bool startRecording();
    void stopRecording();
    bool isRecording();
    bool hasRecordedAudio();
    const uint8_t* getRecordedData();
    size_t getRecordedSize();
    void clearRecording();

    // Playback
    bool startPlayback(const uint8_t* data, size_t size);
    void stopPlayback();
    bool isPlaying();

    // Wake word detection (simple energy-based VAD)
    void enableWakeDetection(bool enabled);
    bool isWakeDetected();
    void clearWakeFlag();

    // Volume
    void setVolume(uint8_t vol);  // 0-100
    uint8_t getVolume();
}
