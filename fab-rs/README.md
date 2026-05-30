# fab-rs — Flesh and Blood engine + cards in Rust

A from-scratch Rust port of the pieces needed to **simulate** Flesh and Blood with
the Aurora (Runeblade / Lightning) card pool we extracted. Zero external
dependencies (std only), so it builds and tests fully offline.

```
fab-rs/
├── crates/
│   ├── fab-cards/    # card data model + the 1024-card Aurora pool (embedded)
│   ├── fab-engine/   # the rules engine (turns, pitch, combat) + agents
│   └── fab-sim/      # CLI simulator
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
