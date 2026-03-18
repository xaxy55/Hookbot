import { test, expect } from "@playwright/test";

const VALID_STATES = [
  "idle",
  "thinking",
  "waiting",
  "success",
  "taskcheck",
  "error",
] as const;

// Warm up: first request to ESP32 can be slow due to mDNS resolution
test("warm up connection", async ({ request }) => {
  const res = await request.get("/status");
  expect(res.ok()).toBe(true);
});

test.describe("POST /state", () => {
  for (const state of VALID_STATES) {
    test(`sets state to "${state}"`, async ({ request }) => {
      const res = await request.post("/state", {
        data: { state },
      });

      expect(res.ok()).toBe(true);
      const body = await res.json();
      expect(body.ok).toBe(true);
      expect(body.state).toBe(state);
    });
  }

  test("accepts optional tool info", async ({ request }) => {
    const res = await request.post("/state", {
      data: { state: "thinking", tool: "Bash", detail: "npm test" },
    });

    expect(res.ok()).toBe(true);
    const body = await res.json();
    expect(body.ok).toBe(true);
    expect(body.state).toBe("thinking");
    expect(body.tool).toBe("Bash");
  });

  test("defaults to idle for unknown state", async ({ request }) => {
    const res = await request.post("/state", {
      data: { state: "nonexistent" },
    });

    expect(res.ok()).toBe(true);
    const body = await res.json();
    expect(body.ok).toBe(true);
    expect(body.state).toBe("idle");
  });

  test("defaults to idle when state field is missing", async ({ request }) => {
    const res = await request.post("/state", {
      data: {},
    });

    expect(res.ok()).toBe(true);
    const body = await res.json();
    expect(body.ok).toBe(true);
    expect(body.state).toBe("idle");
  });
});

test.describe("GET /status", () => {
  test("returns device status", async ({ request }) => {
    const res = await request.get("/status");

    expect(res.ok()).toBe(true);
    const body = await res.json();
    expect(body).toHaveProperty("state");
    expect(body).toHaveProperty("uptime");
    expect(body).toHaveProperty("freeHeap");
    expect(body).toHaveProperty("ip");
    expect(VALID_STATES).toContain(body.state);
    expect(typeof body.uptime).toBe("number");
    expect(typeof body.freeHeap).toBe("number");
  });
});

test.describe("GET /", () => {
  test("returns the control page HTML", async ({ request }) => {
    const res = await request.get("/");

    expect(res.ok()).toBe(true);
    const html = await res.text();
    expect(html).toContain("CEO COMMAND CENTER");
    expect(html).toContain("Destroyer of Worlds");
  });
});

test.describe("state round-trip", () => {
  test("POST /state then GET /status reflects the change", async ({
    request,
  }) => {
    test.setTimeout(30000);
    await request.post("/state", { data: { state: "error" } });

    const res = await request.get("/status");
    const body = await res.json();
    expect(body.state).toBe("error");

    // Reset to idle
    await request.post("/state", { data: { state: "idle" } });
  });
});
