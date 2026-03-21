#pragma once

#include <Arduino.h>

// Mini-games playable on the device screen.
// Snake game for OLED (button/touch controlled).
namespace MiniGames {
    enum class Game : uint8_t {
        NONE,
        SNAKE
    };

    void init();

    /// Start a game. Call from touch UI or command handler.
    void startGame(Game game);

    /// Stop the current game and return to normal avatar mode.
    void stopGame();

    /// Whether a game is currently active.
    bool isActive();

    /// Get current score.
    uint16_t getScore();

    /// Update game logic (call every frame).
    void update(uint32_t deltaMs);

    /// Draw game state to display (call every frame when active).
    void draw();

    /// Input: change direction (0=up, 1=right, 2=down, 3=left)
    void input(uint8_t direction);
}
