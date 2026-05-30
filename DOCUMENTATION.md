# Aurora project — full documentation and critical assessment

This document describes everything built in this working session and assesses it
honestly. It is deliberately **cautious and critical**: where something is
unverified, simplified, or potentially wrong, that is stated plainly. Read the
"Limitations" subsections before relying on any output.

> One-line summary: this is a **research/prototyping spike**, not a finished or
> validated product. The only components that have actually been *executed* are
> the Rust crates (`fab-rs`). Everything that touches an external service
> (TCGplayer, Fabrary/Algolia, a live Talishar server, YouTube) was written but
> **never run**, because the build environment only allows network access to
> GitHub. Treat all such code as untested against reality.

---

## 0. Environment constraints that shaped (and limit) this work

- The sandbox's network policy allowlists **GitHub only**. TCGplayer, RapidAPI,
  fabrary.net, its Algolia backend, `tcgcsv.com`, and YouTube all returned
  `403 Host not in allowlist`. Consequence: every claim about those services rests
  on **reading code/data**, not on observing a live response.
- No credentials were available for any paid/authenticated API (TCGplayer,
  RapidAPI). The pricing tools therefore could not be smoke-tested even partially.
- Because of the above, **none of the cross-service pipelines were validated
  end-to-end**. They may fail on first contact with the real services.

---

## 1. Card extraction — `aurora-report/`

### What it is
A list of the cards playable by "the new Aurora" plus supporting data files:
- `aurora-runeblade-lightning-cards.md` — 212 unique cards (26 Runeblade+Lightning,
  106 Runeblade, 80 Lightning), grouped and tabulated.
- `aurora-cards-with-tcgplayer-ids.json` — the same pool with TCGplayer product IDs.
- `card-identifier-lookup.json` — identifier → name/type/subtype/class/talent/pitch
  for the whole dataset (used by other tools).

### How it was produced
The upstream TypeScript card data (`packages/cards/src/index.ts`, ~478k lines) was
parsed by stripping type annotations and `eval`-ing it with `Proxy` objects standing
in for the enums. Cards legal for `Hero.Aurora2` were selected via each card's
`legalHeroes` field, then filtered to Runeblade-class / Lightning-talent.

### Limitations and criticisms
- **The parsing method is a hack.** The `Proxy` shims return enum *member names*,
  not the enums' real string values. For the fields used here they coincide, but
  this could silently diverge for other fields. It is not a robust parser.
- **"New Aurora" is an interpretation, not a fact.** There are four Aurora hero
  cards in the data. I mapped "new" to `Hero.Aurora2` (the *Omens of the Third Age*
  "Emissary of Lightning" / "Legacy of Tempest"). If you meant the *Rosetta* Aurora
  (also recent, and additionally **Elemental**), the legal pool is different and
  larger. This choice was never confirmed with you.
- **The 212 figure is sensitive to filter choices.** It includes NotClassed
  Lightning-talent cards and two Elemental "Lightning" tokens, and excludes all
  Generic cards. Reasonable alternative choices yield different totals. Counting is
  also done two ways in different files (unique *names* = 212; unique *identifiers*
  incl. pitch variants = 1024), which is easy to conflate.
- **Legality is only as correct as the upstream data.** It depends entirely on the
  community `legalHeroes` field being accurate and current. It was **not**
  cross-checked against official LSS deckbuilding rules, set legality, or any
  banlist. Community datasets lag and contain errors.
- **Licensing of the redistributed data was not verified** (see §6).

---

## 2. Fabrary deck scraper — `aurora-report/scrape-fabrary.mjs`

### What it is
A Node script that pages through Fabrary's public Algolia `public_decks` index for a
given hero/format and aggregates card-inclusion frequencies.

### Status: **written, never executed.**

### Limitations and criticisms
- **Never run.** The Algolia host is blocked here. It has not returned a single
  real response, so the output format, pagination behaviour, and field semantics are
  all *assumed*.
- **Reverse-engineered, not official.** The endpoint and app/search key were taken
  from a third-party repo (`Zugruul/fab-cli`). The key is a public client key, but it
  can change without notice, and **automated scraping may violate Fabrary's terms of
  service**. Use at your own risk.
- **The frequency math rests on an unverified assumption.** It assumes each deck
  record's `cards` array is the maindeck list. Whether it includes sideboard/"maybe"
  cards, or encodes quantities, is **not confirmed**. If that assumption is wrong,
  the "% of decks" and "total copies" numbers are wrong. This is flagged in the
  script but remains a genuine unknown.
- No handling for Algolia result caps, rate limits, or index changes has been tested.

---

## 3. TCGplayer price report generator — `aurora-report/fetch-prices.mjs`

### What it is
A script to turn the card pool into a price report, via either the official
TCGplayer API or the RapidAPI "Marketplace Price Tracker" service.

### Status: **written, never executed; requires an API key you must supply.**

### Limitations and criticisms
- **Never run; no key available.** Neither backend was reachable or authenticated
  here. The exact request/response shapes are based on documentation/examples, not
  observation, and may be wrong.
- The RapidAPI backend is a **third-party reseller**; its accuracy, coverage, and
  longevity are unknown. The official API requires an approved developer account.
- Only **196 of 212** cards carry a TCGplayer product ID in the source data; the
  rest would silently produce no price.
- "Cheapest printing" selection is simplistic, ignores condition/foiling/language,
  and prices are volatile — any report is a fragile snapshot, not a valuation.

---

## 4. Talishar simulation harness — `talishar-sim/`

### What it is
Python that drives the open-source Talishar server to play Aurora games, built on
the third-party `SamuelAnsel/Talishar-RL` Gym environment, plus a deck converter.

### Status: **written, never executed against a server.** Only offline pieces
(Python compiles; the deck converter round-trips) were checked.

### Limitations and criticisms
- **No game was ever played.** Running it needs a local Talishar stack (Docker +
  PHP backend + the RL repo). None of that ran here, so the create-game →
  submit-deck → play loop is unvalidated against a live server.
- **It depends on an upstream repo with a known bug** (the RL env hardcodes a Fai
  deck). My `aurora_env.py` overrides `_create_game` to inject an Aurora deck, but
  this reimplements upstream internals and will break if they change, or if my
  assumptions about the sideboard JSON / lobby readiness flow are wrong.
- **The sample deck may be illegal.** `aurora_deck.example.json` is auto-generated
  (equipment picked by first-match); it was not checked against deck-construction
  rules, and Talishar may reject it.
- **"Is it the real backend?" was answered with strong but incomplete evidence.**
  Endpoint names line up across the official frontend, the backend repo, and the RL
  env, which makes it very likely the open-source repo *is* the real engine. But I
  could **not** prove the public repo is byte-identical to the live production
  deployment, and I never executed it. Don't overstate this to "verified".
- The Combat Dummy is a passive target: it measures kill-speed ("goldfish"), not
  interactive play.

---

## 5. `fab-rs` — Rust engine, cards, simulator, trainer, optimizer

This is the **only part that actually runs.** It compiles with zero external
dependencies and **9 unit tests pass**. That is also the *full* extent of its
validation — see the heavy caveats below.

### Components
- `fab-cards` — card enums + a `Card` model + the 1024-identifier Aurora pool,
  embedded from the upstream data as a TSV.
- `fab-engine` — a simplified rules engine + agents + a learnable linear policy.
- `fab-sim` — batch/narrated game runner.
- `fab-train` — a (1+λ) evolution-strategy self-play trainer.
- `fab-optimize` — a deck-ratio hill-climber ("Dr. Ruckus" method).

### What is verified
- It builds offline; tests pass; the binaries produce the outputs quoted in the
  per-crate READMEs (e.g. goldfish ~6.9 turns; greedy beats random ~95%; the
  optimizer lowers goldfish turns ~6.58 → 5.88).

### Limitations and criticisms — read this before trusting any number
- **This is not Flesh and Blood.** It is a small, self-consistent *subset*. It
  models: a turn loop, one action point + `go again`, pitching to pay costs, a
  **single** attack-vs-block step, `dominate`, on-hit draw, hand refill, and
  life/fatigue loss. It **omits** essentially everything that gives FaB its depth:
  the trigger/layer stack, arcane damage, attack and defense **reactions**, instants
  beyond paying their cost, the combat chain beyond one link, weapon attacks,
  equipment and hero abilities, and — critically — **all per-card scripted text**
  (the hundreds of unique effects that *are* the game). Hero life/intellect are
  hardcoded (40/4), not real values. There is no banlist or deck-legality check.
- **Keyword detection is naive.** Keywords come from substring-matching rules text
  (e.g. `"go again"`), which will misfire on reminder text or negations
  ("doesn't gain go again"). Conditional stats ("gets +3 power if…") are ignored;
  every action with a power value is treated as a vanilla attack.
- **The simulation numbers are artifacts of the toy model and a weak AI, not
  insights about real FaB.** "Goldfish in ~6.9 turns" or any win rate here should
  **not** inform real deckbuilding, card evaluation, or purchases.
- **The AI is shallow.** The greedy/linear agents have no lookahead, no sequencing
  or arsenal planning, and pitch greedily (highest value first), which is not how the
  game is played well. Blocking is a simple heuristic.
- **The self-play result is weak evidence of "learning".** The champion reaches
  100% vs the baselines quickly *because* the strategy space is shallow and the
  baselines are poor. The policy is 9 scalar weights — this is **parameter tuning,
  not mastery**. "best-vs-champ > 50%" can partly reflect overfitting to the fixed
  evaluation seeds rather than genuine improvement. Describing this as "an AI that
  masters the game" would be inaccurate; it tunes a few knobs in a toy.
- **The optimizer optimizes a questionable objective.** "Goldfish kill-speed vs a
  passive dummy" has no opponent interaction, no blocking metagame, and no card
  effects, so its "optimal" deck (and the 46/12/2 pitch curve) is optimal *for the
  toy*, not a real recommendation. It is also: a **local** optimum of single swaps;
  dependent on the start deck and the top-power candidate-selection heuristic; and
  evaluated on **fixed seeds**, so determinism removes noise but **bakes in seed
  bias** (it can overfit those particular shuffles). `--games` defaults are far below
  the ~10k the source video used.
- **Determinism caveat.** Same seed ⇒ identical game. "Win rate over N games" samples
  N fixed shuffles, not a true expectation; small N is not statistically robust.

### Bottom line for `fab-rs`
A clean, tested **foundation and demonstration of the *method*** (engine →
agent → self-play / deck optimization), useful as scaffolding. It is **not** a
source of trustworthy FaB strategy or deck advice, and its outputs should not be
quoted as if they were.

---

## 6. Cross-cutting concerns

### Licensing (unresolved — treat as a risk)
- **The upstream card data's license was not checked** before redistributing a
  derived copy into `fab-rs/.../aurora_cards.tsv` and `aurora-report/*.json`. This
  should be verified before any public distribution.
- Flesh and Blood and all card text/names are **LSS intellectual property**. Card
  data, images, and names are theirs; this project is unaffiliated.
- `fab-rs` is labelled `GPL-3.0` because it follows the Talishar engine's rules, but
  it does **not** copy Talishar code. The label is conservative and may be
  unnecessary; conversely, if Talishar logic were ported in later, GPL obligations
  would attach. Licenses of the third-party Python/RL repos were not audited.

### Validation status (summary table)

| Component | Compiles/parses | Ran locally | Validated vs reality |
|---|---|---|---|
| Card extraction | n/a (data) | yes (offline) | depends on upstream data only |
| Fabrary scraper | yes | **no** | **no** |
| Price generator | yes | **no** | **no** |
| Talishar harness | yes (offline parts) | **no** (no server) | **no** |
| fab-rs (all crates) | yes | yes | **only** self-tests; not vs real FaB |

### Repo hygiene
- Python `__pycache__` bytecode was accidentally committed earlier and has now been
  removed; `.gitignore` files were added for Python and Rust `target/`.
- Large derived artifacts (the TSV and JSON pools) are committed into the repo;
  they are generated data, not hand-maintained source.

---

## 7. What would be needed to make any of this trustworthy

In rough priority order, and without optimism about effort:
1. **Verify the upstream data license** before relying on or sharing the derived sets.
2. **Confirm the "new Aurora" target** (Aurora2 vs the Rosetta Aurora) — the whole
   card pool hinges on it.
3. **Actually run** the scraper, price tool, and Talishar harness against the real
   services from an unrestricted network, and reconcile assumptions (especially the
   Fabrary `cards`-array semantics).
4. For `fab-rs` to produce meaningful strategy, it would need most of the rules it
   currently omits and per-card effect scripting — a large, multi-month effort
   comparable to what Talishar already represents. Until then, prefer driving the
   real Talishar engine over trusting the toy engine's numbers.
