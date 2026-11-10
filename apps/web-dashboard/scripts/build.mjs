import fs from "node:fs";
import path from "node:path";
import crypto from "node:crypto";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(__dirname, "..");
const distDir = path.join(root, "dist");

fs.mkdirSync(distDir, { recursive: true });

const storeCode = fs.readFileSync(path.join(root, "src", "cluster-store.svelte.js"), "utf8");
fs.writeFileSync(path.join(distDir, "cluster-store.js"), storeCode, "utf8");

const html = `<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>UMCF Web & Admin Dashboard</title>
  <style>
    :root { color-scheme: dark; font-family: system-ui, sans-serif; background: #0b0f19; color: #f1f5f9; }
    body { max-width: 1100px; margin: 0 auto; padding: 2rem; }
    .card { background: #111827; border: 1px solid #1f2937; border-radius: 10px; padding: 1.25rem; margin-bottom: 1rem; }
    button { background: #2563eb; color: #fff; border: none; border-radius: 6px; padding: 0.6rem 1rem; cursor: pointer; margin-right: 0.5rem; }
    .badge { display: inline-block; padding: 0.25rem 0.6rem; border-radius: 999px; background: #065f46; color: #a7f3d0; font-size: 0.85rem; }
  </style>
</head>
<body>
  <main>
    <div class="card">
      <h1>Universal Mobile Compute Fabric (UMCF)</h1>
      <p><span class="badge">Protocol v1.0.0</span> Control-Plane Telemetry &amp; Workload Console</p>
    </div>
    <div class="card">
      <h2>Node Governance &amp; Resource Consent</h2>
      <button id="btn-role" type="button">Role: Hybrid</button>
      <button id="btn-consent" type="button">Sharing: Disabled (Safe Default)</button>
      <button id="btn-privacy" type="button">AI Mode: Private Local-Only</button>
    </div>
  </main>
  <script type="module" src="./cluster-store.js"></script>
</body>
</html>
`;

const indexPath = path.join(distDir, "index.html");
fs.writeFileSync(indexPath, html, "utf8");

const hash = crypto.createHash("sha256").update(html).digest("hex");
fs.writeFileSync(path.join(distDir, "SHA256SUMS.txt"), `${hash}  index.html\n`, "utf8");

console.log(`Built @umcf/web-dashboard production artifact at ${indexPath} (sha256: ${hash})`);

