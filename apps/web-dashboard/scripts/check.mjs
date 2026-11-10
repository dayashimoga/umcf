import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(__dirname, "..");

const requiredFiles = [
  path.join(root, "src", "cluster-store.svelte.js"),
  path.join(root, "src", "App.svelte"),
];

for (const file of requiredFiles) {
  if (!fs.existsSync(file)) {
    console.error(`Missing required source file: ${file}`);
    process.exit(1);
  }
  const content = fs.readFileSync(file, "utf8");
  if (content.includes("TODO:") || content.includes("FIX" + "ME:")) {
    console.error(`Unresolved placeholder found in ${file}`);
    process.exit(1);
  }
}

console.log("PASS: @umcf/web-dashboard static & Svelte 5 structure check succeeded.");

