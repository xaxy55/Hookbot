#!/usr/bin/env node
// Hookbot Claude Code Hook
// Sends state updates to ESP32 (direct) or management server (routed).
// Fire-and-forget, never blocks Claude.

const http = require("http");
const https = require("https");
const path = require("path");
const fs = require("fs");

const LOG_FILE = "/tmp/hookbot-hook.log";
const TIMEOUT_MS = 500;

function log(msg) {
  try {
    fs.appendFileSync(LOG_FILE, `${new Date().toISOString()} ${msg}\n`);
  } catch (_) {}
}

function loadConfig() {
  // Check for per-project .hookbot config first
  const projectConfig = path.join(process.cwd(), ".hookbot");
  if (fs.existsSync(projectConfig)) {
    try {
      return JSON.parse(fs.readFileSync(projectConfig, "utf8"));
    } catch (e) {
      log(`Project config error: ${e.message}`);
    }
  }

  const configPath = path.join(__dirname, "hookbot-config.json");
  try {
    return JSON.parse(fs.readFileSync(configPath, "utf8"));
  } catch (e) {
    log(`Config error: ${e.message}`);
    return { host: "http://hookbot.local" };
  }
}

function sendDirect(host, state, toolName) {
  const url = new URL("/state", host);
  const body = JSON.stringify({ state, tool: toolName || "" });
  const mod = url.protocol === "https:" ? https : http;

  return new Promise((resolve) => {
    const req = mod.request(
      url,
      {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          "Content-Length": Buffer.byteLength(body),
        },
        timeout: TIMEOUT_MS,
      },
      (res) => {
        res.resume();
        resolve();
      }
    );

    req.on("error", (e) => {
      log(`Request error: ${e.message}`);
      resolve();
    });

    req.on("timeout", () => {
      req.destroy();
      log("Request timeout");
      resolve();
    });

    req.write(body);
    req.end();
  });
}

function sendToServer(host, event, input, deviceId) {
  const url = new URL("/api/hook", host);
  const body = JSON.stringify({
    event,
    tool_name: input.tool_name || "",
    tool_output: input.tool_output || "",
    project: process.cwd(),
    device_id: deviceId || input.device_id || undefined,
  });
  const mod = url.protocol === "https:" ? https : http;

  return new Promise((resolve) => {
    const req = mod.request(
      url,
      {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          "Content-Length": Buffer.byteLength(body),
          ...(config.api_key ? { "X-API-Key": config.api_key } : {}),
        },
        timeout: TIMEOUT_MS,
      },
      (res) => {
        res.resume();
        resolve();
      }
    );

    req.on("error", (e) => {
      log(`Server request error: ${e.message}`);
      resolve();
    });

    req.on("timeout", () => {
      req.destroy();
      log("Server request timeout");
      resolve();
    });

    req.write(body);
    req.end();
  });
}

function readStdin() {
  return new Promise((resolve) => {
    let data = "";
    process.stdin.setEncoding("utf8");
    process.stdin.on("data", (chunk) => (data += chunk));
    process.stdin.on("end", () => {
      try {
        resolve(JSON.parse(data));
      } catch (_) {
        resolve({});
      }
    });
    setTimeout(() => resolve({}), 200);
  });
}

async function main() {
  const hookEvent = process.argv[2];

  if (!hookEvent) {
    log("No hook event provided");
    process.exit(0);
  }

  const config = loadConfig();
  const input = await readStdin();

  // Server mode: route through management server
  if (config.mode === "server") {
    log(`Event: ${hookEvent} -> server at ${config.host} (device: ${config.device_id || "auto"})`);
    await sendToServer(config.host, hookEvent, input, config.device_id);
    process.exit(0);
  }

  // Direct mode: send straight to device
  let state = "idle";

  switch (hookEvent) {
    case "PreToolUse":
      state = "thinking";
      break;

    case "PostToolUse": {
      const toolName = (input.tool_name || "").toLowerCase();
      const output = (input.tool_output || "").toLowerCase();
      const isBuildOrTest =
        toolName.includes("bash") &&
        (output.includes("passed") ||
          output.includes("success") ||
          output.includes("build succeeded"));
      state = isBuildOrTest ? "success" : "idle";
      break;
    }

    case "UserPromptSubmit":
      state = "thinking";
      break;

    case "Stop":
      state = "idle";
      break;

    case "TaskCompleted":
      state = "success";
      break;

    default:
      log(`Unknown event: ${hookEvent}`);
      process.exit(0);
  }

  log(`Event: ${hookEvent} -> State: ${state}`);
  await sendDirect(config.host, state, input.tool_name);
  process.exit(0);
}

main().catch((e) => {
  log(`Fatal: ${e.message}`);
  process.exit(0);
});
