# Simulating Flesh and Blood (Talishar) with the extracted Aurora cards

**Short answer: yes — there is mature, ready-made open-source code.** Talishar
itself is open source, and it already implements the full rules engine *and* every
card we extracted (the Aurora / Omens of the Third Age cards included). On top of
it, two Python projects wrap it as a reinforcement-learning / simulation
environment. You do **not** need to model the rules from scratch.

## 1. The game engine — `Talishar/Talishar` (the real thing)

- Repo: https://github.com/Talishar/Talishar · PHP · 151★ · GPL-3.0
- This is the actual backend that powers talishar.net — a complete FaB rules
  engine, not a toy. Architecture:
  - **Decision-queue model**: card effects are expressed as sequences of
    `AddDecisionQueue(...)` steps (targeting, choices, triggers, layers). See
    `CardDictionaries/.../*Shared.php`, `PlayAbilities.php`, `HitEffects.php`,
    `Keywords.php`, `CurrentEffectAbilities.php`.
  - **Card data**: `GeneratedCode/GeneratedCardDictionaries.php` holds stats for
    every card keyed by set ID. I verified it contains **237 `OMN###`
    definitions** — e.g. `OMN190` = *Stormshard*, the same Aurora-pool cards in
    our `aurora-cards-with-tcgplayer-ids.json`. Set-specific mechanics live in
    `CardDictionaries/OmensOfTheThirdAge/OMNShared.php` (Duality, etc.).
  - **Built-in AI / no-opponent play**: `AI/CombatDummy.php`, `AI/EncounterAI.php`,
    `AI/CardBehaviors.php`. Lets you run games against a scripted opponent or a
    combat dummy — ideal for headless simulation.
  - Runs locally via Docker (`docker-compose` + `Makefile`); pairs with the
    `Talishar-FE` React frontend only if you want a UI.
- **License caveat**: GPL-3.0 (copyleft). Fine for research/personal simulation;
  anything you distribute that links its code must also be GPL-3.0.

### How it connects to our extracted data
Talishar keys cards by set identifier (`OMN190`, `ROS###`, …). Our extracted
cards carry the same `setIdentifiers`, so the two datasets map 1:1 — you can take
the 212-card Aurora pool and pull the matching engine entries directly by ID.

## 2. Ready-made simulation / RL wrappers (drive the engine for you)

- **`SamuelAnsel/Talishar-RL`** — https://github.com/SamuelAnsel/Talishar-RL · Python
  - A **Gymnasium**-compatible env. `GameConfig(backend_url, format, deck_test_mode=True)`
    creates a game vs the AI, exposes `reset()` / `step(action)` with normalized
    observations and win/loss rewards. Takes a fabrary deck link as the deck.
    This is the closest thing to "press play and simulate".
- **`egbicker/talishar_ml`** — https://github.com/egbicker/talishar_ml · Python
  - A **PettingZoo** multi-agent env where each player is an agent talking to a
    local Talishar server over HTTP. Currently models the Ira Welcome decks;
    the README explicitly lists **Aurora First Strike decks** as the next target.
  - Focused on parallelized training across many episodes.

## 3. Other Talishar org repos (supporting, not engines)

- `Talishar/Talishar-FE` (TypeScript/React, 40★) — the web UI.
- `Talishar/data-doll` (Go) — data service.
- `Talishar/CardImages`, `Talishar/FaB3D` — assets / a 3D client experiment.

## Recommendation

| Goal | Best path |
|---|---|
| Faithful, rules-accurate simulation incl. Aurora cards | Run **Talishar/Talishar** locally (Docker) and play vs its built-in AI |
| Train/evaluate an agent, run many games programmatically | **SamuelAnsel/Talishar-RL** (Gym) or **egbicker/talishar_ml** (PettingZoo) on top of a local Talishar |
| Lightweight custom model in TypeScript using *our* card data | Build a minimal engine seeded from `@flesh-and-blood/cards`; only worthwhile if you need full control and accept re-implementing rules |

Reverse-engineering the rules ourselves would be a large effort and would
duplicate years of work already in Talishar. Reusing Talishar (engine + an RL
wrapper) is by far the fastest route to simulating the Aurora pool.

> Note: I could not clone/run any of this here — this sandbox's network policy
> only allows GitHub, so Docker pulls and a live Talishar server aren't reachable.
> All of the above was verified by reading the repositories directly.
