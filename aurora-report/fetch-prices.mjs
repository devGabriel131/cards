#!/usr/bin/env node
/**
 * Price report generator for the "new Aurora" (Runeblade / Lightning) card pool.
 *
 * Reads `aurora-cards-with-tcgplayer-ids.json` (every card's embedded TCGplayer
 * product IDs come straight from @flesh-and-blood/cards) and produces a price
 * report as Markdown + CSV.
 *
 * It supports two pricing backends — pick whichever you have access to:
 *
 *  1) TCGplayer official API (recommended, exact data)
 *     Requires a TCGplayer API account (https://docs.tcgplayer.com/).
 *     Set env vars:  TCGPLAYER_CLIENT_ID, TCGPLAYER_CLIENT_SECRET
 *     Endpoint used: POST /app/token  then  GET /pricing/product/{ids}
 *
 *  2) RapidAPI "Marketplace Price Tracker" (the backend used by the
 *     lulzasaur9192/tcgplayer-price-api-examples repo). Searches by name.
 *     Set env vars:  RAPIDAPI_KEY  (and optionally RAPIDAPI_HOST)
 *
 * Usage:
 *   TCGPLAYER_CLIENT_ID=... TCGPLAYER_CLIENT_SECRET=... node fetch-prices.mjs
 *   RAPIDAPI_KEY=... node fetch-prices.mjs --backend=rapidapi
 *
 * NOTE: This was written in an environment whose network policy only allows
 * GitHub, so the live fetch could not be exercised here. Run it where the
 * pricing host is reachable.
 */
import { readFileSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const __dir = dirname(fileURLToPath(import.meta.url));
const cards = JSON.parse(
  readFileSync(join(__dir, "aurora-cards-with-tcgplayer-ids.json"), "utf8")
);

const args = Object.fromEntries(
  process.argv.slice(2).map((a) => {
    const [k, v] = a.replace(/^--/, "").split("=");
    return [k, v ?? true];
  })
);
const backend = args.backend || (process.env.RAPIDAPI_KEY ? "rapidapi" : "tcgplayer");

async function getTcgplayerToken() {
  const id = process.env.TCGPLAYER_CLIENT_ID;
  const secret = process.env.TCGPLAYER_CLIENT_SECRET;
  if (!id || !secret)
    throw new Error("Set TCGPLAYER_CLIENT_ID and TCGPLAYER_CLIENT_SECRET");
  const res = await fetch("https://api.tcgplayer.com/token", {
    method: "POST",
    headers: { "Content-Type": "application/x-www-form-urlencoded" },
    body: `grant_type=client_credentials&client_id=${id}&client_secret=${secret}`,
  });
  if (!res.ok) throw new Error(`token failed: ${res.status}`);
  return (await res.json()).access_token;
}

// TCGplayer allows comma-separated product IDs, batched (max ~250 per call).
async function pricesFromTcgplayer(allIds) {
  const token = await getTcgplayerToken();
  const map = new Map();
  for (let i = 0; i < allIds.length; i += 200) {
    const batch = allIds.slice(i, i + 200);
    const res = await fetch(
      `https://api.tcgplayer.com/pricing/product/${batch.join(",")}`,
      { headers: { Authorization: `Bearer ${token}` } }
    );
    if (!res.ok) throw new Error(`pricing failed: ${res.status}`);
    for (const r of (await res.json()).results || []) {
      // keep the "Normal" sub-type market/mid price
      const prev = map.get(String(r.productId));
      if (!prev || r.subTypeName === "Normal")
        map.set(String(r.productId), {
          market: r.marketPrice,
          low: r.lowPrice,
          mid: r.midPrice,
          subType: r.subTypeName,
        });
    }
  }
  return map;
}

async function priceFromRapidApi(name) {
  const host = process.env.RAPIDAPI_HOST || "marketplace-price-tracker.p.rapidapi.com";
  const url = `https://${host}/search?query=${encodeURIComponent(name)}&marketplace=tcgplayer`;
  const res = await fetch(url, {
    headers: {
      "X-RapidAPI-Key": process.env.RAPIDAPI_KEY,
      "X-RapidAPI-Host": host,
    },
  });
  if (!res.ok) return null;
  const data = await res.json();
  // shape varies; grab the first reasonable price field
  const hit = Array.isArray(data) ? data[0] : data.results?.[0] || data;
  return hit ? { market: hit.price ?? hit.marketPrice ?? hit.market } : null;
}

const fmt = (n) => (n == null ? "" : `$${Number(n).toFixed(2)}`);

async function main() {
  const out = [];
  if (backend === "tcgplayer") {
    const allIds = [...new Set(cards.flatMap((c) => c.tcgplayerProductIds))];
    const priceMap = await pricesFromTcgplayer(allIds);
    for (const c of cards) {
      const ids = c.tcgplayerProductIds;
      const priced = ids.map((id) => priceMap.get(id)).filter(Boolean);
      const market = priced.map((p) => p.market).filter((x) => x != null);
      out.push({
        ...c,
        market: market.length ? Math.min(...market) : null,
      });
    }
  } else {
    for (const c of cards) {
      const p = await priceFromRapidApi(c.name);
      out.push({ ...c, market: p?.market ?? null });
      await new Promise((r) => setTimeout(r, 250)); // be polite to rate limits
    }
  }

  out.sort((a, b) => (b.market ?? -1) - (a.market ?? -1));
  const total = out.reduce((s, c) => s + (c.market || 0), 0);

  let md = `# TCGplayer price report — new Aurora (Runeblade / Lightning)\n\n`;
  md += `Backend: \`${backend}\` · Cards: ${out.length} · Approx. total (cheapest printing each): ${fmt(total)}\n\n`;
  md += `| Card | Category | Type | Lowest market price | Product IDs |\n|---|---|---|--:|---|\n`;
  for (const c of out)
    md += `| ${c.name} | ${c.category} | ${c.type} | ${fmt(c.market)} | ${c.tcgplayerProductIds.join(", ")} |\n`;
  writeFileSync(join(__dir, "price-report.md"), md);

  let csv = "name,category,type,pitch,cost,market_price,product_ids\n";
  for (const c of out)
    csv += `"${c.name}","${c.category}","${c.type}","${c.pitch}","${c.cost}",${c.market ?? ""},"${c.tcgplayerProductIds.join(" ")}"\n`;
  writeFileSync(join(__dir, "price-report.csv"), csv);

  console.log(`Wrote price-report.md and price-report.csv (${out.length} cards, total ${fmt(total)})`);
}

main().catch((e) => {
  console.error(e.message);
  process.exit(1);
});
