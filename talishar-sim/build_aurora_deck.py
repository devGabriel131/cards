"""
Convert a Fabrary deck into a Talishar "sideboard" JSON that aurora_env.py can play.

Input options:
  --identifiers "stormshard-red,lightning-press-red,..."   (comma list, fabrary form)
  --from-scraper ../aurora-report/aurora-decks-raw.json --index 0   (a deck the
        scraper collected; uses that deck's `cards` array)

It classifies cards by type/subtype using the card lookup generated from
@flesh-and-blood/cards, assigns equipment to slots, and writes the JSON.
Fabrary identifiers (hyphens) are converted to Talishar identifiers (underscores).

Usage:
    python build_aurora_deck.py --from-scraper ../aurora-report/aurora-decks-raw.json \
        --index 0 --out my_aurora_deck.json
"""
from __future__ import annotations

import argparse
import json
from pathlib import Path

LOOKUP_PATH = Path(__file__).parent.parent / "aurora-report" / "card-identifier-lookup.json"
SLOT_SUBTYPES = {"Head": "head", "Chest": "chest", "Arms": "arms", "Legs": "legs"}


def to_talishar(identifier: str) -> str:
    return identifier.replace("-", "_")


def build(identifiers: list[str], lookup: dict) -> dict:
    sb = {"hero": "", "hands": [], "head": "", "chest": "", "arms": "",
          "legs": "", "deck": [], "inventory": []}
    unknown = []
    for ident in identifiers:
        meta = lookup.get(ident)
        tid = to_talishar(ident)
        if not meta:
            unknown.append(ident)
            sb["deck"].append(tid)  # assume a deck card if unknown
            continue
        types = set(meta["types"].split("/"))
        subtypes = set(meta["subtypes"].split("/"))
        if "Hero" in types:
            sb["hero"] = tid
        elif "Weapon" in types:
            sb["hands"].append(tid)
            sb["inventory"].append(tid)
        elif "Equipment" in types:
            placed = False
            for sub, slot in SLOT_SUBTYPES.items():
                if sub in subtypes:
                    sb[slot] = tid
                    sb["inventory"].append(tid)
                    placed = True
                    break
            if not placed:
                sb["inventory"].append(tid)
        else:
            sb["deck"].append(tid)
    if unknown:
        print(f"WARNING: {len(unknown)} identifiers not in lookup (added to deck as-is): "
              f"{unknown[:10]}{'...' if len(unknown) > 10 else ''}")
    if not sb["hero"]:
        print("WARNING: no Hero card found in the deck list — set sb['hero'] manually.")
    return sb


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--identifiers", help="comma-separated fabrary card identifiers")
    ap.add_argument("--from-scraper", help="path to aurora-decks-raw.json")
    ap.add_argument("--index", type=int, default=0, help="which deck from the scraper file")
    ap.add_argument("--out", default="my_aurora_deck.json")
    args = ap.parse_args()

    lookup = json.loads(LOOKUP_PATH.read_text())

    if args.identifiers:
        ids = [s.strip() for s in args.identifiers.split(",") if s.strip()]
    elif args.from_scraper:
        data = json.loads(Path(args.from_scraper).read_text())
        decks = data["decks"]
        ids = decks[args.index]["cards"]
        print(f"Using deck '{decks[args.index].get('name')}' "
              f"by {decks[args.index].get('author')} ({len(ids)} cards)")
    else:
        ap.error("provide --identifiers or --from-scraper")

    sb = build(ids, lookup)
    Path(args.out).write_text(json.dumps(sb, indent=2))
    print(f"Wrote {args.out}: hero={sb['hero']}, deck={len(sb['deck'])} cards, "
          f"weapon={sb['hands']}, equipment slots set: "
          f"{[s for s in ('head','chest','arms','legs') if sb[s]]}")


if __name__ == "__main__":
    main()
