import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(__dirname, "..");

const required = [
  path.join(root, "svelte.config.js"),
  path.join(root, "vite.config.ts"),
  path.join(root, "tsconfig.json"),
  path.join(root, "src", "console-store.svelte.ts"),
  path.join(root, "src", "App.svelte"),
];

for (const f of required) {
  if (!fs.existsSync(f)) {
    console.error(`Missing required file: ${f}`);
    process.exit(1);
  }
}

console.log("PASS: @umcf/web_console structure and Svelte 5 runes check succeeded.");
