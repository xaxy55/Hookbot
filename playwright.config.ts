import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: "./tests",
  timeout: 15000,
  use: {
    // These tests talk to a real device on the LAN. The mDNS name follows the
    // device's configured hostname (e.g. hookbot-0000.local), so point at your
    // own board with HOOKBOT_URL rather than relying on the default:
    //   HOOKBOT_URL=http://192.168.1.50 npm test
    baseURL: process.env.HOOKBOT_URL || "http://hookbot.local",
  },
});
