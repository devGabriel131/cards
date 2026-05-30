# New Aurora (Runeblade / Lightning) — card pool + price report

## What "new Aurora" means here

Two heroes share the identity **`Hero.Aurora2`** (cardIdentifier `aurora2`), both
from **Omens of the Third Age**:

- **Aurora, Emissary of Lightning** — young hero (Blitz / Draft / Sealed)
- **Aurora, Legacy of Tempest** — adult hero (Classic Constructed / Golden Age / Living Legend)

Both are **Runeblade** with the single **Lightning** talent. (The older
`Hero.Aurora` from Rosetta / 1st Strike is also Elemental — that one is *not*
what this report covers.)

## The card pool

`aurora-runeblade-lightning-cards.md` lists **every card legal for `Hero.Aurora2`**,
filtered to **Runeblade-class** and **Lightning-talent** cards (Generic cards
excluded, as requested):

| Category            | Count |
| ------------------- | ----: |
| Runeblade + Lightning | 26 |
| Runeblade only      |   106 |
| Lightning only      |    80 |
| **Total**           | **212** |

Legality comes directly from each card's `legalHeroes` array in
`@flesh-and-blood/cards`, so it already accounts for class + talent + format rules.

`aurora-cards-with-tcgplayer-ids.json` is the machine-readable version, including
the embedded **TCGplayer product IDs** and purchase URLs for every printing.

## The price report

`fetch-prices.mjs` turns the card pool into a price report (`price-report.md` +
`price-report.csv`). It supports two backends:

1. **TCGplayer official API** (exact pricing by product ID):
   ```sh
   TCGPLAYER_CLIENT_ID=... TCGPLAYER_CLIENT_SECRET=... node fetch-prices.mjs
   ```
2. **RapidAPI "Marketplace Price Tracker"** (the service used by
   [`lulzasaur9192/tcgplayer-price-api-examples`](https://github.com/lulzasaur9192/tcgplayer-price-api-examples),
   searches by card name):
   ```sh
   RAPIDAPI_KEY=... node fetch-prices.mjs --backend=rapidapi
   ```

> ⚠️ This was generated in a sandbox whose network policy only allows GitHub, so
> live prices could **not** be fetched here (TCGplayer / RapidAPI hosts returned
> "Host not in allowlist", and no API key was available). Run the script in an
> environment with network access and a key to produce the actual price numbers.

## Fabrary deck scraper + card-frequency analysis

`scrape-fabrary.mjs` reproduces this page and aggregates card usage across **all**
matching public decks (not just the first page the site shows):

> https://fabrary.net/decks?tab=latest&format=Classic+Constructed&hero=aurora-legacy-of-tempest

It uses fabrary's **public Algolia `public_decks` index** — the same read-only
index the website's deck browser queries (the public search key is shipped
client-side by the site, no login needed). Each indexed deck record already
includes a `cards` array, so frequencies are computed without any authenticated
GraphQL calls.

```sh
node scrape-fabrary.mjs
# or for any other hero/format:
node scrape-fabrary.mjs --hero=aurora-legacy-of-tempest --format="Classic Constructed"
```

Outputs:

- `aurora-deck-frequencies.md` / `.csv` — every card ranked by how many decks
  include it (inclusion count, % of decks, total copies). Card identifiers are
  mapped to readable names via `card-identifier-lookup.json` (generated from
  `@flesh-and-blood/cards`).
- `aurora-decks-raw.json` — every scraped deck with its card list, for your own
  comparisons.

Requires Node 18+ (built-in `fetch`); no `npm install`.

> ⚠️ Same sandbox limitation: the Algolia host is not on this environment's
> allowlist, so the scrape could not be run here. Run it where fabrary.net /
> Algolia is reachable.

### How I found fabrary's API

fabrary.net has no official public API doc, so the endpoint was reverse-engineered
from the open-source [`Zugruul/fab-cli`](https://github.com/Zugruul/fab-cli)
project, which queries the same `public_decks` Algolia index. Full deck card
data (with sideboard/quantities) is also available via fabrary's AppSync GraphQL
`getDeck` query, but that path needs an auth token — the Algolia `cards` field is
enough for frequency analysis and stays fully unauthenticated.
