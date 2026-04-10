/**
 * SolUPG QR checkout demo.
 *
 * A single file Node server that mimics what a real merchant checkout
 * page would do: take an amount from the user, ask SolUPG to generate
 * a Solana Pay URL plus QR code, and render the result so the customer
 * can scan it from their phone wallet.
 *
 * Stack:
 *   - Node built in http module (no Express, no framework)
 *   - @solupg/sdk for the API call
 *   - dotenv for env loading
 *
 * Run it with:
 *   cp .env.example .env       # fill in API_KEY and MERCHANT_WALLET
 *   npm install
 *   npm start
 *   open http://localhost:4000
 */

import "dotenv/config";
import { createServer, type IncomingMessage, type ServerResponse } from "node:http";
import { SolUPG, type GenerateSolanaPayResponse } from "@solupg/sdk";

const port = Number(process.env.PORT ?? 4000);
const baseUrl = process.env.SOLUPG_BASE_URL ?? "http://localhost:3002";
const apiKey = requireEnv("SOLUPG_API_KEY");
const merchantWallet = requireEnv("MERCHANT_WALLET");
const splToken = (process.env.SPL_TOKEN_MINT ?? "").trim() || undefined;

const client = new SolUPG({ baseUrl, apiKey });

const server = createServer(async (req, res) => {
  const url = req.url ?? "/";
  const method = req.method ?? "GET";

  try {
    if (method === "GET" && url === "/") {
      return serveIndex(res);
    }
    if (method === "POST" && url === "/api/checkout") {
      return await handleCheckout(req, res);
    }
    res.statusCode = 404;
    res.setHeader("Content-Type", "text/plain");
    res.end("not found");
  } catch (err) {
    console.error("request error:", err);
    res.statusCode = 500;
    res.setHeader("Content-Type", "application/json");
    res.end(JSON.stringify({ error: (err as Error).message }));
  }
});

server.listen(port, () => {
  console.log(`SolUPG QR checkout demo running at http://localhost:${port}`);
  console.log(`  using gateway: ${baseUrl}`);
  console.log(`  merchant wallet: ${merchantWallet}`);
  if (splToken) {
    console.log(`  spl token mint: ${splToken}`);
  } else {
    console.log(`  spl token mint: (none, native SOL)`);
  }
});

async function handleCheckout(req: IncomingMessage, res: ServerResponse): Promise<void> {
  const body = await readJsonBody(req);
  const amount = String(body.amount ?? "").trim();
  const label = String(body.label ?? "SolUPG Demo Checkout").trim();
  const message = String(body.message ?? "").trim();

  if (!amount) {
    return sendJson(res, 400, { error: "amount is required" });
  }

  const result: GenerateSolanaPayResponse = await client.solanaPay.generate({
    recipient: merchantWallet,
    amount,
    splToken,
    label,
    message: message || undefined,
  });

  return sendJson(res, 200, result);
}

function serveIndex(res: ServerResponse): void {
  res.statusCode = 200;
  res.setHeader("Content-Type", "text/html; charset=utf-8");
  res.end(INDEX_HTML);
}

function sendJson(res: ServerResponse, status: number, body: unknown): void {
  res.statusCode = status;
  res.setHeader("Content-Type", "application/json");
  res.end(JSON.stringify(body));
}

async function readJsonBody(req: IncomingMessage): Promise<Record<string, unknown>> {
  return new Promise((resolve, reject) => {
    let raw = "";
    req.on("data", (chunk: Buffer) => {
      raw += chunk.toString("utf8");
    });
    req.on("end", () => {
      if (!raw) return resolve({});
      try {
        resolve(JSON.parse(raw));
      } catch (err) {
        reject(new Error(`invalid JSON body: ${(err as Error).message}`));
      }
    });
    req.on("error", reject);
  });
}

function requireEnv(name: string): string {
  const value = process.env[name];
  if (!value || value.startsWith("REPLACE_WITH_")) {
    console.error(`Missing required env var: ${name}`);
    console.error(`Copy .env.example to .env, fill it in, then run again.`);
    process.exit(1);
  }
  return value;
}

const INDEX_HTML = `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  <title>SolUPG QR Checkout Demo</title>
  <style>
    :root {
      --bg: #0b0f1a;
      --panel: #131826;
      --border: #232a3d;
      --text: #e6e8ef;
      --muted: #8a91a5;
      --accent: #9945ff;
      --accent-2: #14f195;
    }
    * { box-sizing: border-box; }
    body {
      margin: 0;
      font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", system-ui, sans-serif;
      background: var(--bg);
      color: var(--text);
      min-height: 100vh;
      display: flex;
      align-items: center;
      justify-content: center;
      padding: 24px;
    }
    .container {
      width: 100%;
      max-width: 920px;
      background: var(--panel);
      border: 1px solid var(--border);
      border-radius: 20px;
      overflow: hidden;
      display: grid;
      grid-template-columns: 1fr 1fr;
      box-shadow: 0 24px 60px rgba(0,0,0,0.4);
    }
    @media (max-width: 720px) { .container { grid-template-columns: 1fr; } }
    .panel { padding: 36px; }
    .panel.left { border-right: 1px solid var(--border); }
    @media (max-width: 720px) { .panel.left { border-right: 0; border-bottom: 1px solid var(--border); } }
    h1 { margin: 0 0 8px 0; font-size: 22px; letter-spacing: -0.01em; }
    .sub { color: var(--muted); font-size: 14px; margin: 0 0 28px 0; }
    label { display: block; font-size: 12px; color: var(--muted); margin: 16px 0 6px 0; text-transform: uppercase; letter-spacing: 0.06em; }
    input, textarea {
      width: 100%;
      background: #0b0f1a;
      border: 1px solid var(--border);
      border-radius: 10px;
      padding: 12px 14px;
      color: var(--text);
      font-size: 15px;
      font-family: inherit;
    }
    input:focus, textarea:focus { outline: none; border-color: var(--accent); }
    button {
      margin-top: 24px;
      width: 100%;
      padding: 14px;
      background: linear-gradient(135deg, var(--accent), #6b2cd0);
      border: 0;
      border-radius: 10px;
      color: white;
      font-size: 15px;
      font-weight: 600;
      cursor: pointer;
      transition: transform 0.1s ease;
    }
    button:hover { transform: translateY(-1px); }
    button:disabled { opacity: 0.5; cursor: not-allowed; transform: none; }
    .right { display: flex; flex-direction: column; align-items: center; justify-content: center; text-align: center; }
    .placeholder { color: var(--muted); font-size: 14px; max-width: 240px; line-height: 1.5; }
    .qr-wrap {
      width: 280px;
      height: 280px;
      background: white;
      border-radius: 16px;
      padding: 16px;
      display: flex;
      align-items: center;
      justify-content: center;
      box-shadow: 0 12px 30px rgba(153, 69, 255, 0.25);
    }
    .qr-wrap img { width: 100%; height: 100%; }
    .url-box {
      margin-top: 18px;
      width: 100%;
      max-width: 320px;
      background: #0b0f1a;
      border: 1px solid var(--border);
      border-radius: 8px;
      padding: 10px;
      font-family: ui-monospace, "Cascadia Code", Menlo, monospace;
      font-size: 11px;
      word-break: break-all;
      color: var(--accent-2);
      max-height: 80px;
      overflow: auto;
    }
    .copy-btn {
      margin-top: 8px;
      background: transparent;
      border: 1px solid var(--border);
      color: var(--muted);
      font-size: 12px;
      padding: 8px 14px;
      border-radius: 8px;
      cursor: pointer;
      width: auto;
    }
    .error {
      color: #ff6b8a;
      font-size: 13px;
      margin-top: 12px;
    }
    .footer {
      grid-column: 1 / -1;
      padding: 16px 36px;
      border-top: 1px solid var(--border);
      font-size: 12px;
      color: var(--muted);
      display: flex;
      justify-content: space-between;
    }
    .badge { color: var(--accent-2); font-weight: 600; }
  </style>
</head>
<body>
  <div class="container">
    <div class="panel left">
      <h1>SolUPG Checkout</h1>
      <p class="sub">Generate a Solana Pay QR code that any wallet can scan.</p>

      <form id="checkout-form">
        <label for="amount">Amount</label>
        <input id="amount" name="amount" type="text" inputmode="decimal" placeholder="1.50" required />

        <label for="label">Label</label>
        <input id="label" name="label" type="text" placeholder="Warung Kopi Pak Budi" />

        <label for="message">Message</label>
        <input id="message" name="message" type="text" placeholder="Latte and croissant" />

        <button type="submit" id="submit-btn">Generate QR</button>
        <div id="error" class="error"></div>
      </form>
    </div>

    <div class="panel right" id="qr-panel">
      <div class="placeholder">
        Fill in an amount and click <strong>Generate QR</strong>. The QR code will appear here, ready to be scanned by any Solana Pay compatible wallet.
      </div>
    </div>

    <div class="footer">
      <span>Powered by <span class="badge">SolUPG</span></span>
      <span>Open source. Apache 2.0.</span>
    </div>
  </div>

  <script>
    const form = document.getElementById("checkout-form");
    const errorEl = document.getElementById("error");
    const submitBtn = document.getElementById("submit-btn");
    const qrPanel = document.getElementById("qr-panel");

    form.addEventListener("submit", async (event) => {
      event.preventDefault();
      errorEl.textContent = "";
      submitBtn.disabled = true;
      submitBtn.textContent = "Generating...";

      const formData = new FormData(form);
      const payload = {
        amount: formData.get("amount"),
        label: formData.get("label"),
        message: formData.get("message"),
      };

      try {
        const resp = await fetch("/api/checkout", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(payload),
        });
        const body = await resp.json();
        if (!resp.ok) {
          throw new Error(body.error || "request failed");
        }
        renderQr(body);
      } catch (err) {
        errorEl.textContent = err.message;
      } finally {
        submitBtn.disabled = false;
        submitBtn.textContent = "Generate QR";
      }
    });

    function renderQr(result) {
      qrPanel.innerHTML = "";

      const wrap = document.createElement("div");
      wrap.className = "qr-wrap";
      const img = document.createElement("img");
      img.alt = "Solana Pay QR";
      img.src = "data:image/png;base64," + result.qr_png_base64;
      wrap.appendChild(img);

      const urlBox = document.createElement("div");
      urlBox.className = "url-box";
      urlBox.textContent = result.url;

      const copyBtn = document.createElement("button");
      copyBtn.className = "copy-btn";
      copyBtn.type = "button";
      copyBtn.textContent = "Copy URL";
      copyBtn.addEventListener("click", async () => {
        try {
          await navigator.clipboard.writeText(result.url);
          copyBtn.textContent = "Copied";
          setTimeout(() => { copyBtn.textContent = "Copy URL"; }, 1500);
        } catch (e) {
          copyBtn.textContent = "Copy failed";
        }
      });

      qrPanel.appendChild(wrap);
      qrPanel.appendChild(urlBox);
      qrPanel.appendChild(copyBtn);
    }
  </script>
</body>
</html>`;
