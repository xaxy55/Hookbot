#ifndef NO_AUDIO

#include "audio.h"
#include "config.h"
#include <driver/i2s.h>

namespace Audio {

// Recording buffer (allocated on PSRAM if available)
static uint8_t* recordBuffer = nullptr;
static size_t recordedBytes = 0;
static bool recording = false;
static bool recorded = false;

// Playback state
static const uint8_t* playbackData = nullptr;
static size_t playbackSize = 0;
static size_t playbackPos = 0;
static bool playing = false;

// Wake word detection (energy-based VAD)
static bool wakeEnabled = false;
static bool wakeDetected = false;
static int silenceCount = 0;
static int speechCount = 0;

// Volume (0-100)
static uint8_t volume = 80;

// I2S port assignments
static const i2s_port_t MIC_PORT = I2S_NUM_0;
static const i2s_port_t SPK_PORT = I2S_NUM_1;

static void initMicrophone() {
    i2s_config_t mic_config = {};
    mic_config.mode = (i2s_mode_t)(I2S_MODE_MASTER | I2S_MODE_RX);
    mic_config.sample_rate = AUDIO_SAMPLE_RATE;
    mic_config.bits_per_sample = I2S_BITS_PER_SAMPLE_16BIT;
    mic_config.channel_format = I2S_CHANNEL_FMT_ONLY_LEFT;
    mic_config.communication_format = I2S_COMM_FORMAT_STAND_I2S;
    mic_config.intr_alloc_flags = ESP_INTR_FLAG_LEVEL1;
    mic_config.dma_buf_count = 4;
    mic_config.dma_buf_len = 1024;
    mic_config.use_apll = false;
    mic_config.tx_desc_auto_clear = false;
    mic_config.fixed_mclk = 0;

    i2s_pin_config_t mic_pins = {};
    mic_pins.bck_io_num = I2S_MIC_SCK;
    mic_pins.ws_io_num = I2S_MIC_WS;
    mic_pins.data_in_num = I2S_MIC_SD;
    mic_pins.data_out_num = I2S_PIN_NO_CHANGE;

    esp_err_t err = i2s_driver_install(MIC_PORT, &mic_config, 0, NULL);
    if (err != ESP_OK) {
        Serial.printf("[Audio] Mic I2S install failed: %d\n", err);
        return;
    }
    i2s_set_pin(MIC_PORT, &mic_pins);
    i2s_zero_dma_buffer(MIC_PORT);
    Serial.println("[Audio] Microphone initialized (INMP441)");
}

static void initSpeaker() {
    i2s_config_t spk_config = {};
    spk_config.mode = (i2s_mode_t)(I2S_MODE_MASTER | I2S_MODE_TX);
    spk_config.sample_rate = AUDIO_SAMPLE_RATE;
    spk_config.bits_per_sample = I2S_BITS_PER_SAMPLE_16BIT;
    spk_config.channel_format = I2S_CHANNEL_FMT_ONLY_LEFT;
    spk_config.communication_format = I2S_COMM_FORMAT_STAND_I2S;
    spk_config.intr_alloc_flags = ESP_INTR_FLAG_LEVEL1;
    spk_config.dma_buf_count = 4;
    spk_config.dma_buf_len = 1024;
    spk_config.use_apll = false;
    spk_config.tx_desc_auto_clear = true;
    spk_config.fixed_mclk = 0;

    i2s_pin_config_t spk_pins = {};
    spk_pins.bck_io_num = I2S_SPK_BCK;
    spk_pins.ws_io_num = I2S_SPK_WS;
    spk_pins.data_out_num = I2S_SPK_DOUT;
    spk_pins.data_in_num = I2S_PIN_NO_CHANGE;

    esp_err_t err = i2s_driver_install(SPK_PORT, &spk_config, 0, NULL);
    if (err != ESP_OK) {
        Serial.printf("[Audio] Speaker I2S install failed: %d\n", err);
        return;
    }
    i2s_set_pin(SPK_PORT, &spk_pins);
    i2s_zero_dma_buffer(SPK_PORT);
    Serial.println("[Audio] Speaker initialized (MAX98357A)");
}

void init() {
    // Allocate recording buffer (prefer PSRAM)
#if BOARD_HAS_PSRAM
    recordBuffer = (uint8_t*)ps_malloc(RECORD_BUFFER_SIZE);
#else
    recordBuffer = (uint8_t*)malloc(RECORD_BUFFER_SIZE);
#endif
    if (!recordBuffer) {
        Serial.println("[Audio] Failed to allocate recording buffer!");
        return;
    }

    initMicrophone();
    initSpeaker();
    Serial.printf("[Audio] Initialized (buffer=%dKB, rate=%dHz)\n",
        RECORD_BUFFER_SIZE / 1024, AUDIO_SAMPLE_RATE);
}

static uint16_t computeRMS(const int16_t* samples, size_t count) {
    if (count == 0) return 0;
    uint64_t sum = 0;
    for (size_t i = 0; i < count; i++) {
        int32_t s = samples[i];
        sum += s * s;
    }
    return (uint16_t)sqrt((double)sum / count);
}

void update(uint32_t deltaMs) {
    // Handle active recording
    if (recording && recordBuffer) {
        int16_t buf[512];
        size_t bytesRead = 0;
        esp_err_t err = i2s_read(MIC_PORT, buf, sizeof(buf), &bytesRead, 0);
        if (err == ESP_OK && bytesRead > 0) {
            size_t samples = bytesRead / sizeof(int16_t);
            uint16_t rms = computeRMS(buf, samples);

            if (rms > VAD_ENERGY_THRESHOLD) {
                speechCount += samples;
                silenceCount = 0;
            } else {
                silenceCount += samples;
            }

            // Auto-stop on extended silence (only after some speech)
            if (speechCount > VAD_MIN_SPEECH_FRAMES && silenceCount > VAD_SILENCE_FRAMES) {
                stopRecording();
                return;
            }

            // Copy to buffer
            size_t remaining = RECORD_BUFFER_SIZE - recordedBytes;
            size_t toCopy = (bytesRead < remaining) ? bytesRead : remaining;
            if (toCopy > 0) {
                memcpy(recordBuffer + recordedBytes, buf, toCopy);
                recordedBytes += toCopy;
            }

            // Stop if buffer full
            if (recordedBytes >= RECORD_BUFFER_SIZE) {
                stopRecording();
            }
        }
    }

    // Handle wake detection (passive listening when not recording)
    if (wakeEnabled && !recording && !playing) {
        int16_t buf[256];
        size_t bytesRead = 0;
        esp_err_t err = i2s_read(MIC_PORT, buf, sizeof(buf), &bytesRead, 0);
        if (err == ESP_OK && bytesRead > 0) {
            size_t samples = bytesRead / sizeof(int16_t);
            uint16_t rms = computeRMS(buf, samples);

            // Detect sustained loud audio as a "wake" trigger
            if (rms > VAD_ENERGY_THRESHOLD * 2) {
                speechCount += samples;
                if (speechCount > VAD_MIN_SPEECH_FRAMES) {
                    wakeDetected = true;
                    speechCount = 0;
                    Serial.println("[Audio] Wake detected!");
                }
            } else {
                speechCount = 0;
            }
        }
    }

    // Handle playback
    if (playing && playbackData && playbackPos < playbackSize) {
        size_t chunk = 1024;
        if (playbackPos + chunk > playbackSize) {
            chunk = playbackSize - playbackPos;
        }

        // Apply volume scaling
        int16_t scaled[512];
        size_t sampleCount = chunk / sizeof(int16_t);
        const int16_t* src = (const int16_t*)(playbackData + playbackPos);
        for (size_t i = 0; i < sampleCount; i++) {
            scaled[i] = (int16_t)((int32_t)src[i] * volume / 100);
        }

        size_t bytesWritten = 0;
        i2s_write(SPK_PORT, scaled, chunk, &bytesWritten, 0);
        playbackPos += bytesWritten;

        if (playbackPos >= playbackSize) {
            stopPlayback();
        }
    }
}

bool startRecording() {
    if (!recordBuffer || playing) return false;
    recordedBytes = 0;
    recording = true;
    recorded = false;
    silenceCount = 0;
    speechCount = 0;
    i2s_zero_dma_buffer(MIC_PORT);
    Serial.println("[Audio] Recording started");
    return true;
}

void stopRecording() {
    if (!recording) return;
    recording = false;
    recorded = (recordedBytes > 0 && speechCount > VAD_MIN_SPEECH_FRAMES);
    Serial.printf("[Audio] Recording stopped (%d bytes, valid=%s)\n",
        recordedBytes, recorded ? "yes" : "no");
}

bool isRecording() { return recording; }
bool hasRecordedAudio() { return recorded && recordedBytes > 0; }
const uint8_t* getRecordedData() { return recordBuffer; }
size_t getRecordedSize() { return recordedBytes; }

void clearRecording() {
    recordedBytes = 0;
    recorded = false;
}

bool startPlayback(const uint8_t* data, size_t size) {
    if (!data || size == 0 || recording) return false;
    playbackData = data;
    playbackSize = size;
    playbackPos = 0;
    playing = true;
    i2s_zero_dma_buffer(SPK_PORT);
    Serial.printf("[Audio] Playback started (%d bytes)\n", size);
    return true;
}

void stopPlayback() {
    playing = false;
    playbackData = nullptr;
    playbackSize = 0;
    playbackPos = 0;
    i2s_zero_dma_buffer(SPK_PORT);
    Serial.println("[Audio] Playback stopped");
}

bool isPlaying() { return playing; }

void enableWakeDetection(bool enabled) {
    wakeEnabled = enabled;
    speechCount = 0;
    Serial.printf("[Audio] Wake detection %s\n", enabled ? "enabled" : "disabled");
}

bool isWakeDetected() { return wakeDetected; }
void clearWakeFlag() { wakeDetected = false; }

void setVolume(uint8_t vol) {
    volume = (vol > 100) ? 100 : vol;
}

uint8_t getVolume() { return volume; }

} // namespace Audio

#endif // !NO_AUDIO
