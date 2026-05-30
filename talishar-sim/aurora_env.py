"""
Aurora simulation environment for Talishar.

Wraps SamuelAnsel/Talishar-RL's `TalisharGymEnvironment` but fixes one blocker:
that repo's `_create_game()` **hardcodes a Fai sideboard** in `SubmitSideboard.php`
(see gymenv.py ~line 346), so no matter what `deck_link` you pass, you'd actually
play Fai. This subclass overrides `_create_game()` to submit a configurable
Aurora deck instead (loaded from a Talishar "sideboard" JSON).

Setup:
  1. Clone the RL env next to this folder so `gymenv.py` is importable:
         git clone https://github.com/SamuelAnsel/Talishar-RL
         cp Talishar-RL/gymenv.py .        # or add it to PYTHONPATH
  2. Run a local Talishar backend (Docker) — see README.md.
  3. pip install gymnasium numpy requests rich

Talishar card identifiers are the fabrary identifiers with hyphens replaced by
underscores (e.g. fabrary `stormshard-red` -> Talishar `stormshard_red`). The
deck JSON here already uses the Talishar form.
"""
from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path

from gymenv import TalisharGymEnvironment, GameConfig  # from Talishar-RL


@dataclass
class AuroraGameConfig(GameConfig):
    # Path to a Talishar sideboard JSON (hero/hands/head/chest/arms/legs/deck/inventory)
    deck_json_path: str = "aurora_deck.example.json"
    # Aurora is CC; openformatcc lets the deck-test sandbox accept the pool.
    format: str = "openformatcc"


class AuroraTalisharEnv(TalisharGymEnvironment):
    """Talishar env that plays a configurable Aurora deck vs the Combat Dummy AI."""

    def __init__(self, config: AuroraGameConfig | None = None):
        self.aurora_config = config or AuroraGameConfig()
        super().__init__(self.aurora_config)
        self._sideboard = self._load_sideboard(self.aurora_config.deck_json_path)

    @staticmethod
    def _load_sideboard(path: str) -> dict:
        data = json.loads(Path(path).read_text())
        required = {"hero", "deck"}
        missing = required - data.keys()
        if missing:
            raise ValueError(f"Deck JSON missing keys: {missing}")
        return data

    def _create_game(self):
        """Re-implementation of the parent's _create_game that submits OUR deck."""
        base = self.config.backend_url.rstrip("/")

        # 1. Create the game (deck_link is still passed but the sideboard below wins)
        payload = {
            "format": self.config.format,
            "visibility": self.config.visibility,
            "deckTestMode": self.config.deck_test_mode,
            "fabdb": self.config.deck_link,
            "deckTestDeck": self.config.ai_deck,  # "Dummy" = combat dummy opponent
        }
        resp = self._make_request("POST", f"{base}/api/APIs/CreateGame.php", json=payload)
        if "error" in resp:
            raise RuntimeError(f"Failed to create game: {resp['error']}")

        self.game_id = resp.get("gameName")
        self.player_id = resp.get("playerID", 1)
        self.auth_key = resp.get("authKey")
        if not all([self.game_id, self.player_id, self.auth_key]):
            raise RuntimeError(f"Invalid game creation response: {resp}")

        # 2. Submit the Aurora sideboard (this is the part the upstream repo hardcodes)
        submission = json.dumps(self._sideboard)
        sb_payload = {
            "gameName": self.game_id,
            "playerID": self.player_id,
            "authKey": self.auth_key,
            "submission": submission,
        }
        sb = self._make_request(
            "POST", f"{base}/api/APIs/SubmitSideboard.php", json=sb_payload
        )
        if sb.get("status") != "OK":
            raise RuntimeError(f"Failed to submit Aurora deck: {sb}")

        # 3. Wait for the game/AI to be ready (parent helper)
        self._wait_for_game_ready()
