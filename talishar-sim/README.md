# Aurora simulation harness (Talishar)

Runs simulated games for **Aurora, Legacy of Tempest** against Talishar's built-in
Combat Dummy AI and logs win/loss, turns, and reward — built on top of
[`SamuelAnsel/Talishar-RL`](https://github.com/SamuelAnsel/Talishar-RL) (a
Gymnasium env that talks to a Talishar server).

## Why this wrapper exists

The upstream `gymenv.py` has the right Gym interface, but its `_create_game()`
**hardcodes a Fai sideboard** (gymenv.py ~line 346) — so whatever `deck_link` you
pass, you'd actually play Fai. `aurora_env.py` subclasses the env and overrides the
deck submission to send a configurable **Aurora** deck (a Talishar "sideboard"
JSON). Everything else (state polling, action extraction, rewards) is reused.

> Card identifiers: Talishar uses the fabrary identifier with hyphens replaced by
> underscores (`stormshard-red` → `stormshard_red`). The deck JSON and the
> converter already handle this.

## Files

| File | Purpose |
|---|---|
| `aurora_env.py` | `AuroraTalisharEnv` — Talishar Gym env that plays a configurable Aurora deck vs the AI |
| `run_simulation.py` | Run N games with a random-legal-action baseline; write `results.csv` + summary |
| `build_aurora_deck.py` | Convert a Fabrary deck (or a deck from the scraper's `aurora-decks-raw.json`) into a Talishar sideboard JSON |
| `aurora_deck.example.json` | Auto-generated legal-ish Aurora deck (hero + equipment + 60 cards) so it runs out of the box. **Not optimized** — swap in a real list. |

## Setup

```sh
# 1. Get the upstream Gym env so `gymenv.py` is importable
git clone https://github.com/SamuelAnsel/Talishar-RL
cp Talishar-RL/gymenv.py .        # or put it on PYTHONPATH

# 2. Python deps
pip install gymnasium numpy requests rich

# 3. Run a local Talishar backend (the actual rules engine) via Docker
git clone https://github.com/Talishar/Talishar
git clone https://github.com/Talishar/Talishar-FE      # FE is mounted by compose
cd Talishar && docker compose up        # serves the API the env calls
```

The default backend URL is `http://host.docker.internal:5173/` (what the upstream
repo expects). Adjust `--backend` if your Talishar instance is elsewhere.

## Run a batch of games

```sh
python run_simulation.py --games 20 --deck aurora_deck.example.json \
    --backend http://host.docker.internal:5173/ --out results.csv
```

Example output:
```
Game 1/20: win in 142 steps (you 7 - 0 opp), reward=98.3, 11.4s
...
=== Summary ===
Games: 20  Wins: 13  Win rate: 65.0%
Avg steps/game: 130.5  Avg reward: 41.20
```

## Use a real deck

Pull real Aurora CC lists with `../aurora-report/scrape-fabrary.mjs`, then convert
one into a Talishar deck:

```sh
python build_aurora_deck.py --from-scraper ../aurora-report/aurora-decks-raw.json \
    --index 0 --out my_aurora_deck.json
python run_simulation.py --deck my_aurora_deck.json --games 50
```

Or pass identifiers directly:
```sh
python build_aurora_deck.py --identifiers "aurora-legacy-of-tempest,stormshard-red,..." \
    --out my_aurora_deck.json
```

## Plug in your own agent

`run_simulation.py`'s `choose_action(legal_actions)` is a uniform-random baseline.
Replace it with a trained policy (it receives the list of legal action indices;
`env.step(action)` returns the standard Gym 5-tuple, and `info['game_state']` holds
the parsed Talishar state for richer features).

## Caveats

- The Combat Dummy is a passive opponent — good for measuring whether a deck can
  "goldfish" / close out games, not for evaluating interactive play. For a real
  opponent, run two agents (see `egbicker/talishar_ml`'s PettingZoo approach) or
  set `ai_deck` to a non-Dummy deck if your Talishar build supports it.
- A random policy will misplay badly; win rates are only meaningful once you add a
  real policy. The harness is the scaffolding, not a strong player.
- `aurora_deck.example.json` is auto-generated from the legal Aurora pool for
  runnability; it is not a competitive list. Use `build_aurora_deck.py` with a real
  fabrary deck for meaningful results.
- Talishar is GPL-3.0; the RL env is its own license — check before redistributing.
- Could not be executed in this repo's environment (network is GitHub-only, so no
  local Talishar server / Docker). Validated offline: Python compiles and the deck
  converter round-trips correctly.
