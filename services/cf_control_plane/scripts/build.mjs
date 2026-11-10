import fs from "node:fs";
import path from "node:path";
import crypto from "node:crypto";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(__dirname, "..");
const distDir = path.join(root, "dist");

fs.mkdirSync(distDir, { recursive: true });
const src = fs.readFileSync(path.join(root, "src", "index.ts"), "utf8");
const outPath = path.join(distDir, "index.js");
fs.writeFileSync(outPath, src, "utf8");

const hash = crypto.createHash("sha256").update(src).digest("hex");
fs.writeFileSync(path.join(distDir, "SHA256SUMS.txt"), `${hash}  index.js\n`, "utf8");
console.log(`Built @umcf/cf_control_plane bundle at ${outPath} (sha256: ${hash})`);
