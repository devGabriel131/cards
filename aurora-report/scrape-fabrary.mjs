#!/usr/bin/env node
/**
 * Fabrary public-deck scraper + card-frequency analyzer.
 *
 * Mirrors this page:
 *   https://fabrary.net/decks?tab=latest&format=Classic+Constructed&hero=aurora-legacy-of-tempest
 *
 * How it works
 * ------------
 * Fabrary's public deck list is powered by a public, read-only Algolia index
 * ("public_decks") — the same one the website's deck browser queries. No login
 * or personal API key is required; the public search key below is the one the
 * site ships to browsers. Each indexed deck record already contains a `cards`
 * array (card identifiers), so we can compute card-inclusion frequencies across
 * every matching public deck without any authenticated GraphQL calls.
 *
 * It paginates through ALL matching decks (not just the first page the website
 * shows), aggregates how often each card appears, maps identifiers to readable
 * names via card-identifier-lookup.json, and writes:
 *   - aurora-deck-frequencies.csv
 *   - aurora-deck-frequencies.md
 *   - aurora-decks-raw.json   (every deck + its cards, for your own analysis)
 *
 * Usage
 * -----
 *   node scrape-fabrary.mjs
 *   node scrape-fabrary.mjs --hero=aurora-legacy-of-tempest --format="Classic Constructed"
 *   node scrape-fabrary.mjs --hero=briar --format="Classic Constructed"
 *
 * Requires Node 18+ (built-in fetch). No npm install needed.
 *
 * NOTE: built in a GitHub-only sandbox, so the live Algolia call could not be
 * exercised here. Run it on a machine with normal internet access.
 */
import { readFileSync, writeFileSync, existsSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const __dir = dirname(fileURLToPath(import.meta.url));

// ---- Public Algolia credentials (shipped client-side by fabrary.net) ----
const ALGOLIA_APP_ID = "4E2YSY5Y4I";
const ALGOLIA_API_KEY = "63c7b6aa56d38399d37df3c341b982c3";
const ALGOLIA_URL = `https://${ALGOLIA_APP_ID.toLowerCase()}-dsn.algolia.net/1/indexes/*/queries`;
const INDEX = "public_decks";

// ---- args ----
const args = Object.fromEntries(
  process.argv.slice(2).map((a) => {
    const [k, ...v] = a.replace(/^--/, "").split("=");
    return [k, v.length ? v.join("=") : true];
  })
);
const HERO = args.hero || "aurora-legacy-of-tempest";
const FORMAT = args.format || "Classic Constructed";
const HITS_PER_PAGE = Number(args.hitsPerPage || 200);

// ---- identifier -> name lookup (optional, generated from @flesh-and-blood/cards) ----
let lookup = {};
const lookupPath = join(__dir, "card-identifier-lookup.json");
if (existsSync(lookupPath)) lookup = JSON.parse(readFileSync(lookupPath, "utf8"));

function describe(id) {
  const m = lookup[id];
  return m
    ? { name: m.name, pitch: m.pitch, type: m.types, cls: m.classes, talents: m.talents }
    : { name: id, pitch: null, type: "", cls: "", talents: "" };
}

async function searchPage(page) {
  const params = new URLSearchParams({
    query: "",
    hitsPerPage: String(HITS_PER_PAGE),
    page: String(page),
    facetFilters: JSON.stringify([
      [`heroIdentifier:${HERO}`],
      [`format:${FORMAT}`],
    ]),
    facets: JSON.stringify(["heroIdentifier", "format"]),
  });
  const body = { requests: [{ indexName: INDEX, params: params.toString() }] };
  const url = `${ALGOLIA_URL}?x-algolia-api-key=${ALGOLIA_API_KEY}&x-algolia-application-id=${ALGOLIA_APP_ID}&x-algolia-agent=fabrary-scraper`;
  const res = await fetch(url, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
  });
  if (!res.ok) throw new Error(`Algolia ${res.status}: ${await res.text()}`);
  return (await res.json()).results[0];
}

async function main() {
  console.log(`Fetching public decks: hero=${HERO}, format=${FORMAT}`);
  const first = await searchPage(0);
  const nbPages = first.nbPages;
  console.log(`  ${first.nbHits} decks across ${nbPages} page(s)`);

  const decks = [...first.hits];
  for (let p = 1; p < nbPages; p++) {
    const r = await searchPage(p);
    decks.push(...r.hits);
    process.stdout.write(`  page ${p + 1}/${nbPages}\r`);
  }
  console.log(`\nCollected ${decks.length} decks.`);

  // Aggregate. `cards` may list an identifier once per copy or once total —
  // we track both deck-inclusion count and raw occurrence count to be safe.
  const inclDecks = new Map(); // id -> # decks containing it
  const occurrences = new Map(); // id -> total times it appears across all decks
  for (const d of decks) {
    const ids = Array.isArray(d.cards) ? d.cards : [];
    const unique = new Set(ids);
    for (const id of unique) inclDecks.set(id, (inclDecks.get(id) || 0) + 1);
    for (const id of ids) occurrences.set(id, (occurrences.get(id) || 0) + 1);
  }

  const N = decks.length || 1;
  const rows = [...inclDecks.keys()]
    .map((id) => {
      const d = describe(id);
      const inDecks = inclDecks.get(id);
      return {
        identifier: id,
        name: d.name,
        pitch: d.pitch ?? "",
        type: d.type,
        classes: d.cls,
        talents: d.talents,
        decksIncluding: inDecks,
        pctOfDecks: +((inDecks / N) * 100).toFixed(1),
        totalCopies: occurrences.get(id),
      };
    })
    .sort((a, b) => b.decksIncluding - a.decksIncluding || a.name.localeCompare(b.name));

  // raw dump
  writeFileSync(
    join(__dir, "aurora-decks-raw.json"),
    JSON.stringify(
      { hero: HERO, format: FORMAT, deckCount: decks.length, decks },
      null,
      2
    )
  );

  // CSV
  let csv =
    "identifier,name,pitch,type,classes,talents,decks_including,pct_of_decks,total_copies\n";
  for (const r of rows)
    csv += `"${r.identifier}","${r.name}","${r.pitch}","${r.type}","${r.classes}","${r.talents}",${r.decksIncluding},${r.pctOfDecks},${r.totalCopies}\n`;
  writeFileSync(join(__dir, "aurora-deck-frequencies.csv"), csv);

  // Markdown
  let md = `# Fabrary card frequencies — ${HERO} (${FORMAT})\n\n`;
  md += `Based on **${decks.length}** public decks scraped from fabrary.net's Algolia index.\n\n`;
  md += `Ranked by how many decks include each card. "% of decks" = inclusion rate.\n\n`;
  md += `| # | Card | Pitch | Type | Decks | % of decks | Total copies |\n|--:|---|--:|---|--:|--:|--:|\n`;
  rows.forEach((r, i) => {
    md += `| ${i + 1} | ${r.name} | ${r.pitch} | ${r.type} | ${r.decksIncluding} | ${r.pctOfDecks}% | ${r.totalCopies} |\n`;
  });
  writeFileSync(join(__dir, "aurora-deck-frequencies.md"), md);

  console.log(
    `Wrote aurora-deck-frequencies.csv / .md (${rows.length} distinct cards) and aurora-decks-raw.json`
  );
}

main().catch((e) => {
  console.error("ERROR:", e.message);
  process.exit(1);
});
