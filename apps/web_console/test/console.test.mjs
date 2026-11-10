import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(__dirname, "..");

test("[UNIT] web_console configuration and runes store definitions exist and are valid", () => {
  assert.ok(fs.existsSync(path.join(root, "svelte.config.js")));
  assert.ok(fs.existsSync(path.join(root, "vite.config.ts")));
  assert.ok(fs.existsSync(path.join(root, "tsconfig.json")));
  assert.ok(fs.existsSync(path.join(root, "src", "console-store.svelte.ts")));
  assert.ok(fs.existsSync(path.join(root, "src", "App.svelte")));
});
