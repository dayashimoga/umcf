import fs from "node:fs";
import path from "node:path";
import crypto from "node:crypto";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(__dirname, "..");
const dist = path.join(root, "dist");

fs.mkdirSync(dist, { recursive: true });

const html = `<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8"/>
  <meta name="viewport" content="width=device-width, initial-scale=1"/>
  <title>UMCF Web Console</title>
</head>
<body>
  <div id="app"><h1>UMCF Web Console v0.1.0</h1></div>
</body>
</html>`;

const outIndex = path.join(dist, "index.html");
fs.writeFileSync(outIndex, html, "utf8");
const hash = crypto.createHash("sha256").update(html).digest("hex");
fs.writeFileSync(path.join(dist, "SHA256SUMS.txt"), `${hash}  index.html\n`, "utf8");

console.log(`Built @umcf/web_console production artifact at ${outIndex}`);
