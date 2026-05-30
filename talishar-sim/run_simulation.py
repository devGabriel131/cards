"""
Run N simulated Aurora games against Talishar's Combat Dummy AI and log results.

Baseline policy = uniformly random legal action each step. Swap in your own
policy (e.g. a trained agent) by replacing `choose_action`.

Usage:
    python run_simulation.py --games 20 --deck aurora_deck.example.json \
        --backend http://host.docker.internal:5173/ --out results.csv

Outputs a per-game CSV and prints a summary (win rate, avg turns, avg reward).
"""
from __future__ import annotations

import argparse
import csv
import random
import time

from aurora_env import AuroraTalisharEnv, AuroraGameConfig


def choose_action(legal_actions: list[int]) -> int:
    """Baseline: pick a uniformly random legal action."""
    return random.choice(legal_actions)


def play_one_game(env: AuroraTalisharEnv, max_steps: int) -> dict:
    obs, info = env.reset()
    total_reward = 0.0
    steps = 0
    final = None
    for steps in range(1, max_steps + 1):
        action = choose_action(info["legal_actions"])
        obs, reward, terminated, truncated, info = env.step(action)
        total_reward += reward
        final = info.get("game_state")
        if terminated or truncated:
            break

    player_hp = getattr(final, "player_health", None)
    opp_hp = getattr(final, "opponent_health", None)
    won = opp_hp is not None and opp_hp <= 0 and (player_hp is None or player_hp > 0)
    return {
        "result": "win" if won else ("loss" if (player_hp is not None and player_hp <= 0) else "unfinished"),
        "steps": steps,
        "player_health": player_hp,
        "opponent_health": opp_hp,
        "total_reward": round(total_reward, 2),
    }


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--games", type=int, default=10)
    ap.add_argument("--deck", default="aurora_deck.example.json")
    ap.add_argument("--backend", default="http://host.docker.internal:5173/")
    ap.add_argument("--max-steps", type=int, default=1000)
    ap.add_argument("--out", default="results.csv")
    ap.add_argument("--seed", type=int, default=0)
    args = ap.parse_args()

    random.seed(args.seed)
    config = AuroraGameConfig(
        backend_url=args.backend,
        deck_json_path=args.deck,
        deck_test_mode=True,
        ai_deck="Dummy",
        max_game_length=args.max_steps,
    )
    env = AuroraTalisharEnv(config)

    rows = []
    wins = 0
    try:
        for g in range(1, args.games + 1):
            t0 = time.time()
            r = play_one_game(env, args.max_steps)
            r["game"] = g
            r["seconds"] = round(time.time() - t0, 1)
            rows.append(r)
            wins += r["result"] == "win"
            print(f"Game {g}/{args.games}: {r['result']} in {r['steps']} steps "
                  f"(you {r['player_health']} - {r['opponent_health']} opp), "
                  f"reward={r['total_reward']}, {r['seconds']}s")
    finally:
        env.close()

    fields = ["game", "result", "steps", "player_health", "opponent_health",
              "total_reward", "seconds"]
    with open(args.out, "w", newline="") as f:
        w = csv.DictWriter(f, fieldnames=fields)
        w.writeheader()
        w.writerows(rows)

    n = len(rows) or 1
    avg_steps = sum(r["steps"] for r in rows) / n
    avg_reward = sum(r["total_reward"] for r in rows) / n
    print("\n=== Summary ===")
    print(f"Games: {len(rows)}  Wins: {wins}  Win rate: {wins / n:.1%}")
    print(f"Avg steps/game: {avg_steps:.1f}  Avg reward: {avg_reward:.2f}")
    print(f"Per-game results written to {args.out}")


if __name__ == "__main__":
    main()
