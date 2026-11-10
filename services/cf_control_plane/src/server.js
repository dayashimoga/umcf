// Local development and integration testing server for UMCF Cloudflare Control Plane.
// Wraps handleControlPlaneRequest with Node.js node:http server.

import http from "node:http";
import { handleControlPlaneRequest, resetLocalCoordinators } from "./index.js";

/**
 * Starts a local HTTP server handling UMCF control plane requests.
 * @param {number} [port=0] Port to listen on (0 for dynamic free port)
 * @returns {Promise<{ server: http.Server; port: number; url: string; close: () => Promise<void> }>}
 */
export function startControlPlaneServer(port = 0) {
  return new Promise((resolve, reject) => {
    const server = http.createServer(async (req, res) => {
      try {
        const fullUrl = `http://${req.headers.host ?? "127.0.0.1"}${req.url ?? "/"}`;

        // Collect body chunks
        const chunks = [];
        for await (const chunk of req) {
          chunks.push(chunk);
        }
        const bodyBuffer = Buffer.concat(chunks);
        const bodyBytes = new Uint8Array(bodyBuffer);

        // Map Node headers to fetch Headers
        const headers = new Headers();
        for (const [key, val] of Object.entries(req.headers)) {
          if (Array.isArray(val)) {
            for (const v of val) headers.append(key, v);
          } else if (val !== undefined) {
            headers.set(key, val);
          }
        }

        // Construct Fetch Request
        const init = {
          method: req.method ?? "GET",
          headers
        };
        if (req.method !== "GET" && req.method !== "HEAD" && bodyBytes.length > 0) {
          init.body = bodyBytes;
        }

        const fetchReq = new Request(fullUrl, init);
        const fetchRes = await handleControlPlaneRequest(fetchReq);

        // Send response back
        res.statusCode = fetchRes.status;
        for (const [k, v] of fetchRes.headers.entries()) {
          res.setHeader(k, v);
        }

        const resArray = await fetchRes.arrayBuffer();
        res.end(Buffer.from(resArray));
      } catch (err) {
        res.statusCode = 500;
        res.setHeader("Content-Type", "application/json");
        res.end(JSON.stringify({ code: "E5000", error: String(err) }));
      }
    });

    server.listen(port, "127.0.0.1", () => {
      const addr = server.address();
      const actualPort = typeof addr === "object" && addr ? addr.port : port;
      resolve({
        server,
        port: actualPort,
        url: `http://127.0.0.1:${actualPort}`,
        close: () =>
          new Promise((resClose) => {
            resetLocalCoordinators();
            server.close(() => resClose());
          })
      });
    });

    server.on("error", reject);
  });
}

// Standalone execution: node services/cf_control_plane/src/server.js 8787
if (process.argv[1] && process.argv[1].endsWith("server.js")) {
  const argPort = parseInt(process.argv[2] ?? "8787", 10);
  startControlPlaneServer(argPort).then(({ port, url }) => {
    console.log(`UMCF Control Plane running on ${url} (port ${port})`);
  });
}
