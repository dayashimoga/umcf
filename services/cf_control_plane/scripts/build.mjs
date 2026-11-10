import fs from "node:fs";
import path from "node:path";
import crypto from "node:crypto";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(__dirname, "..");
const srcDir = path.join(root, "src");
const distDir = path.join(root, "dist");

fs.mkdirSync(distDir, { recursive: true });

const filesToCopy = ["index.js", "crypto.js", "durable_object.js", "server.js"];
const checksumLines = [];

for (const file of filesToCopy) {
  const srcPath = path.join(srcDir, file);
  if (fs.existsSync(srcPath)) {
    const content = fs.readFileSync(srcPath);
    const outPath = path.join(distDir, file);
    fs.writeFileSync(outPath, content);
    const hash = crypto.createHash("sha256").update(content).digest("hex");
    checksumLines.push(`${hash}  ${file}`);
  }
}

fs.writeFileSync(path.join(distDir, "SHA256SUMS.txt"), checksumLines.join("\n") + "\n", "utf8");
console.log(`Built @umcf/cf_control_plane dist (${checksumLines.length} files hashed in dist/SHA256SUMS.txt)`);
