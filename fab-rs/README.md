# fab-rs — Flesh and Blood engine + cards in Rust

A from-scratch Rust port of the pieces needed to **simulate** Flesh and Blood with
the Aurora (Runeblade / Lightning) card pool we extracted. Zero external
dependencies (std only), so it builds and tests fully offline.

```
fab-rs/
├── crates/
│   ├── fab-cards/    # card data model + the 1024-card Aurora pool (embedded)
│   ├── fab-engine/   # rules engine (turns, pitch, combat) + agents + learnable policy
│   ├── fab-sim/      # CLI simulator
│   └── fab-train/    # self-play trainer (evolution strategy)
```

## Quick start

```sh
cd fab-rs
cargo test                                   # 7 tests, all green
cargo run --release -p fab-sim -- --games 2000        # goldfish batch
cargo run --release -p fab-sim -- --verbose --seed 3  # one narrated game
cargo run --release -p fab-sim -- --mirror --games 2000   # greedy vs random
```

Example output:

```
=== Aurora goldfish vs Combat Dummy : 2000 games ===
P0 (Aurora) wins: 2000  (100.0%)
Turns: avg 6.87, fastest 5, slowest 10

=== Aurora(greedy) vs Aurora(random) : 2000 games ===
P0 (Aurora) wins: 1898  (94.9%)     # the heuristic clearly beats random
```

## The crates

### `fab-cards`
- `types.rs` — `CardType`, `Class`, `Talent`, `EquipSlot`, `Pitch` enums (ported
  from `@flesh-and-blood/types`).
- `card.rs` — the `Card` struct + `Keywords` parsed from rules text
  (`go again`, `dominate`, `intimidate`, `overpower`, on-hit draw).
- `aurora_cards.tsv` — **every card legal for the new Aurora** (`Hero.Aurora2`),
  generated from `@flesh-and-blood/cards` and embedded with `include_str!`.
  `CardDb::load()` parses it into a `HashMap<id, Card>`.

### `fab-engine`
A faithful **subset** of the real rules, enough to simulate games:
- Turn structure with a single **action point** and **`go again`**.
- **Pitching** cards from hand to pay costs (auto, fewest-cards-first).
- **Attacks vs. blocks** on a one-link combat step, **`dominate`**, on-hit draw.
- Hand **refill to intellect**, pitched cards to bottom of deck.
- Loss by **life ≤ 0** or **fatigue** (a real player who can't present a turn).
- An `Agent` trait + three baseline agents: `CombatDummyAgent` (passive target),
  `RandomAgent`, and `GreedyAttackAgent` (max-power, prefers go-again, blocks lean).

### `fab-sim`
CLI driver: runs N games, tallies win rate and turn stats, or narrates one game.

### `fab-train` — self-play AI
An agent that **learns to play by self-play**, with no ML dependencies. The policy
([`LinearAgent`](crates/fab-engine/src/linear.rs)) scores every decision as a dot
product of a small feature vector (power, go-again, cost, pitch, lethal, on-hit
draw, pass threshold, block threshold) with a learned weight vector — so the
strategy it discovers is fully readable.

Training is a **(1 + λ) evolution strategy**: each generation perturbs the
champion's weights into λ candidates, each candidate plays K games *against the
current champion*, and the best candidate that beats it (>50%) becomes the new
champion. Fitness comes purely from self-play; the only fixed yardstick is a
learning-curve readout vs. the Greedy/Random baselines.

```sh
cargo run --release -p fab-train -- --generations 40 --lambda 8 --games 30 --seed 1
```

```
gen   0 | champion vs greedy  86.0% | vs random  97.5% | (baseline weights)
gen   5 | champion vs greedy 100.0% | vs random  99.5% | best-vs-champ 70.0%
gen  40 | champion vs greedy 100.0% | vs random 100.0% | best-vs-champ 63.3%
Saved learned weights to learned_weights.csv
```

`best-vs-champ > 50%` every generation = the agent keeps finding ways to beat its
former self. Learned weights are saved to CSV and load back via `Weights::load`.

**Toward AlphaZero:** the `Agent` trait is the seam. The next steps are (1) a
determinized-MCTS agent (pure Rust, doable offline) for look-ahead, and (2) a
neural policy/value net trained by self-play — which needs an ML crate
(`burn`/`candle`/`tch`) and therefore network access this sandbox doesn't have.
The evolution-strategy trainer here is the dependency-free stand-in that already
demonstrates self-play mastery over the baselines.

## What is and isn't modelled

**In:** turn flow, action points, go again, pitch economy, attack/block combat,
dominate, on-hit draw, hand size/intellect, fatigue, deterministic seeded RNG.

**Out (the honest boundary):** arcane damage, the triggered-ability / layer stack,
attack & defense **reactions**, instants beyond paying their cost, equipment
abilities, per-card scripted text (the ~hundreds of unique effects Talishar
implements one by one), and multi-version deck legality. The engine is a clean,
well-tested **foundation** with a data-driven default behaviour for every card —
not a complete reimplementation of Talishar's years of card logic.

## How card text becomes behaviour

Most cards act through their data (power, cost, pitch, and keywords detected in the
text). To script a specific card precisely, extend `play_from_hand` in
`fab-engine/src/game.rs` with a per-id match arm, or add a `CardEffect` trait
dispatched by `card.id` — the architecture leaves a clear seam for this.

> License: GPL-3.0-or-later (matching the Talishar engine whose rules it follows).
