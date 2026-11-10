import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(__dirname, "..");

test("[UNIT] cf_control_plane wrangler.toml and src/index.ts exist and are non-empty", () => {
  const wrangler = path.join(root, "wrangler.toml");
  const indexTs = path.join(root, "src", "index.ts");
  assert.ok(fs.existsSync(wrangler));
  assert.ok(fs.existsSync(indexTs));
  assert.ok(fs.statSync(wrangler).size > 0);
  assert.ok(fs.statSync(indexTs).size > 0);
});
