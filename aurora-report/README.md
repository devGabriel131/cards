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
