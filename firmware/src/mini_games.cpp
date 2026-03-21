#include "mini_games.h"
#include "display.h"
#include "config.h"

namespace MiniGames {

// ─── Snake Game ────────────────────────────────────────────────

// Grid: each cell is 4x4 pixels
static const uint8_t CELL = 4;
static const uint8_t GRID_W = SCREEN_WIDTH / CELL;
static const uint8_t GRID_H = (SCREEN_HEIGHT - 10) / CELL;  // Reserve top 10px for score
static const uint8_t GRID_Y_OFF = 10;  // Y offset for score bar
static const uint16_t MAX_SNAKE = 128;

static Game currentGame = Game::NONE;

// Snake state
static uint8_t snakeX[MAX_SNAKE];
static uint8_t snakeY[MAX_SNAKE];
static uint16_t snakeLen = 3;
static uint8_t snakeDir = 1;     // 0=up, 1=right, 2=down, 3=left
static uint8_t nextDir = 1;
static uint8_t foodX, foodY;
static uint16_t score = 0;
static bool gameOver = false;
static uint32_t moveTimer = 0;
static const uint32_t MOVE_INTERVAL_MS = 150;  // Snake speed

static void spawnFood() {
    bool valid;
    do {
        foodX = esp_random() % GRID_W;
        foodY = esp_random() % GRID_H;
        valid = true;
        for (uint16_t i = 0; i < snakeLen; i++) {
            if (snakeX[i] == foodX && snakeY[i] == foodY) {
                valid = false;
                break;
            }
        }
    } while (!valid);
}

static void initSnake() {
    snakeLen = 3;
    snakeDir = 1;
    nextDir = 1;
    score = 0;
    gameOver = false;
    moveTimer = 0;

    // Start in center
    uint8_t cx = GRID_W / 2;
    uint8_t cy = GRID_H / 2;
    for (uint16_t i = 0; i < snakeLen; i++) {
        snakeX[i] = cx - i;
        snakeY[i] = cy;
    }

    spawnFood();
    Serial.println("[Games] Snake started!");
}

static void updateSnake(uint32_t deltaMs) {
    if (gameOver) return;

    moveTimer += deltaMs;
    if (moveTimer < MOVE_INTERVAL_MS) return;
    moveTimer = 0;

    // Apply direction change (prevent 180-degree turns)
    if ((nextDir + 2) % 4 != snakeDir) {
        snakeDir = nextDir;
    }

    // Calculate new head position
    int8_t newX = snakeX[0];
    int8_t newY = snakeY[0];
    switch (snakeDir) {
        case 0: newY--; break;  // up
        case 1: newX++; break;  // right
        case 2: newY++; break;  // down
        case 3: newX--; break;  // left
    }

    // Wall collision → wrap around
    if (newX < 0) newX = GRID_W - 1;
    if (newX >= GRID_W) newX = 0;
    if (newY < 0) newY = GRID_H - 1;
    if (newY >= GRID_H) newY = 0;

    // Self collision
    for (uint16_t i = 0; i < snakeLen; i++) {
        if (snakeX[i] == (uint8_t)newX && snakeY[i] == (uint8_t)newY) {
            gameOver = true;
            Serial.printf("[Games] Snake game over! Score: %d\n", score);
            return;
        }
    }

    // Check food
    bool ate = ((uint8_t)newX == foodX && (uint8_t)newY == foodY);

    // Move body: shift everything down
    if (!ate) {
        // Normal move: shift tail
        for (uint16_t i = snakeLen - 1; i > 0; i--) {
            snakeX[i] = snakeX[i - 1];
            snakeY[i] = snakeY[i - 1];
        }
    } else {
        // Grow: shift everything, keep tail
        if (snakeLen < MAX_SNAKE) {
            for (uint16_t i = snakeLen; i > 0; i--) {
                snakeX[i] = snakeX[i - 1];
                snakeY[i] = snakeY[i - 1];
            }
            snakeLen++;
        }
        score += 10;
        spawnFood();
    }

    snakeX[0] = (uint8_t)newX;
    snakeY[0] = (uint8_t)newY;
}

static void drawSnake(DisplayCanvas* d) {
    // Score bar at top
    d->setTextSize(1);
    d->setTextColor(COLOR_WHITE);
    d->setCursor(0, 1);
    d->print("SNAKE ");
    d->print(score);

    // Grid border
    d->drawRect(0, GRID_Y_OFF, GRID_W * CELL, GRID_H * CELL, COLOR_WHITE);

    // Food (blinking dot)
    uint32_t t = millis();
    if ((t / 300) % 2 == 0) {
        d->fillRect(
            foodX * CELL + 1, foodY * CELL + GRID_Y_OFF + 1,
            CELL - 1, CELL - 1, COLOR_WHITE
        );
    } else {
        d->drawRect(
            foodX * CELL, foodY * CELL + GRID_Y_OFF,
            CELL, CELL, COLOR_WHITE
        );
    }

    // Snake body
    for (uint16_t i = 0; i < snakeLen; i++) {
        int16_t px = snakeX[i] * CELL;
        int16_t py = snakeY[i] * CELL + GRID_Y_OFF;
        if (i == 0) {
            // Head: filled
            d->fillRect(px, py, CELL, CELL, COLOR_WHITE);
        } else {
            // Body: filled with 1px gap for segmented look
            d->fillRect(px + 1, py + 1, CELL - 1, CELL - 1, COLOR_WHITE);
        }
    }

    // Game over overlay
    if (gameOver) {
        int16_t cx = SCREEN_WIDTH / 2;
        int16_t cy = SCREEN_HEIGHT / 2;

        // Dark background box
        d->fillRect(cx - 30, cy - 12, 60, 24, COLOR_BLACK);
        d->drawRect(cx - 30, cy - 12, 60, 24, COLOR_WHITE);

        d->setTextSize(1);
        d->setTextColor(COLOR_WHITE);
        d->setCursor(cx - 24, cy - 8);
        d->print("GAME OVER");
        d->setCursor(cx - 22, cy + 2);
        d->print("Score:");
        d->print(score);
    }
}

// ─── Public API ─────────────────────────────────────────────────

void init() {
    currentGame = Game::NONE;
}

void startGame(Game game) {
    currentGame = game;
    switch (game) {
        case Game::SNAKE: initSnake(); break;
        default: break;
    }
}

void stopGame() {
    currentGame = Game::NONE;
}

bool isActive() {
    return currentGame != Game::NONE;
}

uint16_t getScore() {
    return score;
}

void update(uint32_t deltaMs) {
    switch (currentGame) {
        case Game::SNAKE: updateSnake(deltaMs); break;
        default: break;
    }
}

void draw() {
    switch (currentGame) {
        case Game::SNAKE: drawSnake(Display::getCanvas()); break;
        default: break;
    }
}

void input(uint8_t direction) {
    if (currentGame == Game::SNAKE) {
        if (gameOver) {
            // Any input restarts
            initSnake();
        } else {
            nextDir = direction % 4;
        }
    }
}

} // namespace MiniGames
