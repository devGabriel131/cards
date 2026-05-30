//! Deck-ratio optimizer for Aurora — the "Dr. Ruckus" approach for Flesh and Blood.
//!
//! Pipeline: engine (`fab-engine`) + a fast, decent AI (`GreedyAttackAgent`) +
//! **hill climbing over card quantities**. Each iteration it searches single
//! card-for-card swaps (remove one copy, add one copy) and applies the swap that
//! most improves the objective, until no swap helps — i.e. the local optimum the
//! video describes: "no matter which two cards you flipped, the win rate could
//! only go down."
//!
//! Because FaB's card value is numeric (pitch / cost / power), this naturally
//! tunes both *which* cards and their *pitch ratios* (how many red vs. blue).
//!
//! Objectives:
//!   --objective goldfish   minimise average turns to defeat the Combat Dummy
//!   --objective winrate    maximise win rate vs a fixed reference Aurora deck
//!
//! Evaluation is deterministic (fixed seed set per build), so the climb is
//! noise-free and terminates cleanly. More `--games` = more accurate (the video
//! used 10k; the engine here is simpler so fewer suffice).
//!
//! Usage:
//!   fab-optimize --objective goldfish --names 24 --games 80 --max-iters 12

use fab_cards::CardDb;
use fab_engine::agents::{CombatDummyAgent, GreedyAttackAgent};
use fab_engine::{Deck, Game, Outcome};
use std::collections::{BTreeMap, HashMap};
use std::rc::Rc;

const DECK_SIZE: usize = 60;
const MAX_PER_NAME: u32 = 3;

struct Cfg {
    objective: String,
    names: usize,
    games: u32,
    max_iters: u32,
    seed: u64,
}
fn parse() -> Cfg {
    let mut c = Cfg {
        objective: "goldfish".into(),
        names: 24,
        games: 80,
        max_iters: 15,
        seed: 12345,
    };
    let mut it = std::env::args().skip(1);
    while let Some(a) = it.next() {
        let mut next = || it.next().unwrap_or_default();
        match a.as_str() {
            "--objective" => c.objective = next(),
            "--names" => c.names = next().parse().unwrap_or(c.names),
            "--games" => c.games = next().parse().unwrap_or(c.games),
            "--max-iters" => c.max_iters = next().parse().unwrap_or(c.max_iters),
            "--seed" => c.seed = next().parse().unwrap_or(c.seed),
            "--help" | "-h" => {
                println!("fab-optimize [--objective goldfish|winrate] [--names N] [--games K] [--max-iters M] [--seed S]");
                std::process::exit(0);
            }
            _ => {}
        }
    }
    c
}

type Build = BTreeMap<String, u32>;

struct Pool {
    ids: Vec<String>,                  // candidate card identifiers (variants of chosen names)
    name_of: HashMap<String, String>,  // id -> card name
}

/// Choose candidate names: the strongest Aurora (Lightning/Runeblade) attacks by
/// power, then take every pitch variant of those names as the optimizer's palette.
fn build_pool(db: &CardDb, n_names: usize) -> Pool {
    let mut best_power: HashMap<String, i32> = HashMap::new();
    for c in db.iter() {
        if c.is_attack()
            && c.power.unwrap_or(0) > 0
            && (c.has_talent(fab_cards::Talent::Lightning) || c.has_class(fab_cards::Class::Runeblade))
        {
            let e = best_power.entry(c.name.clone()).or_insert(0);
            *e = (*e).max(c.power.unwrap_or(0));
        }
    }
    let mut names: Vec<(String, i32)> = best_power.into_iter().collect();
    names.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    let chosen: Vec<String> = names.into_iter().take(n_names).map(|(n, _)| n).collect();
    let chosen_set: std::collections::HashSet<&String> = chosen.iter().collect();

    let mut ids = Vec::new();
    let mut name_of = HashMap::new();
    for c in db.iter() {
        if c.is_attack() && chosen_set.contains(&c.name) {
            ids.push(c.id.clone());
            name_of.insert(c.id.clone(), c.name.clone());
        }
    }
    ids.sort();
    Pool { ids, name_of }
}

fn total(b: &Build) -> u32 {
    b.values().sum()
}
fn name_total(b: &Build, pool: &Pool, name: &str) -> u32 {
    b.iter()
        .filter(|(id, _)| pool.name_of.get(*id).map(|n| n == name).unwrap_or(false))
        .map(|(_, c)| *c)
        .sum()
}
fn to_cards(b: &Build) -> Vec<String> {
    let mut v = Vec::new();
    for (id, &c) in b {
        for _ in 0..c {
            v.push(id.clone());
        }
    }
    v
}

/// Seed a starting deck: 3 copies of each candidate name (highest-power variant)
/// until the deck is full.
fn initial_build(db: &CardDb, pool: &Pool) -> Build {
    // best variant per name = highest power
    let mut best_variant: HashMap<String, (String, i32)> = HashMap::new();
    for id in &pool.ids {
        let card = db.get(id).unwrap();
        let name = pool.name_of[id].clone();
        let p = card.power.unwrap_or(0);
        let e = best_variant.entry(name).or_insert((id.clone(), -1));
        if p > e.1 {
            *e = (id.clone(), p);
        }
    }
    let mut variants: Vec<(String, String)> =
        best_variant.into_iter().map(|(n, (id, _))| (n, id)).collect();
    variants.sort();
    let mut b: Build = BTreeMap::new();
    'outer: loop {
        let before = total(&b);
        for (_, id) in &variants {
            if total(&b) >= DECK_SIZE as u32 {
                break 'outer;
            }
            let e = b.entry(id.clone()).or_insert(0);
            if *e < MAX_PER_NAME {
                *e += 1;
            }
        }
        if total(&b) == before {
            break; // can't grow further
        }
    }
    b
}

/// Objective value (higher is better) plus a human-readable metric.
fn evaluate(
    db: &Rc<CardDb>,
    b: &Build,
    objective: &str,
    games: u32,
    seed0: u64,
) -> (f64, f64) {
    let cards = to_cards(b);
    match objective {
        "winrate" => {
            let mut wins = 0.0;
            for k in 0..games {
                let seed = seed0.wrapping_add(k as u64);
                let me = Deck::aurora_with_cards(db.as_ref(), cards.clone());
                let opp = Deck::aurora_sample(db.as_ref());
                // alternate seats to remove first-player bias
                let (d0, d1, my_seat) = if k % 2 == 0 { (me, opp, 0) } else { (opp, me, 1) };
                let mut g = Game::with_shared(db.clone(), [d0, d1], seed);
                let mut a0 = GreedyAttackAgent::new("a0");
                let mut a1 = GreedyAttackAgent::new("a1");
                if let Outcome::Win(w) = g.run(&mut a0, &mut a1) {
                    if w == my_seat {
                        wins += 1.0;
                    }
                }
            }
            let wr = wins / games as f64;
            (wr, wr * 100.0)
        }
        _ => {
            // goldfish: fewer turns to kill the dummy is better.
            let mut sum_turns = 0u64;
            for k in 0..games {
                let seed = seed0.wrapping_add(k as u64);
                let me = Deck::aurora_with_cards(db.as_ref(), cards.clone());
                let dummy = Deck::combat_dummy(40);
                let mut g = Game::with_shared(db.clone(), [me, dummy], seed);
                let mut a = GreedyAttackAgent::new("aurora");
                let mut d = CombatDummyAgent::new();
                g.run(&mut a, &mut d);
                sum_turns += g.turn as u64;
            }
            let avg = sum_turns as f64 / games as f64;
            (-avg, avg) // score = -avg_turns (maximised), metric = avg_turns
        }
    }
}

fn pitch_curve(db: &CardDb, b: &Build) -> (u32, u32, u32) {
    let (mut r, mut y, mut bl) = (0, 0, 0);
    for (id, &c) in b {
        match db.get(id).map(|x| x.pitch_value) {
            Some(Some(1)) => r += c,
            Some(Some(2)) => y += c,
            Some(Some(3)) => bl += c,
            _ => {}
        }
    }
    (r, y, bl)
}

fn print_deck(db: &CardDb, b: &Build, metric_label: &str, metric: f64) {
    let mut rows: Vec<(&String, u32)> = b.iter().map(|(id, c)| (id, *c)).collect();
    rows.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
    println!("\nOptimized Aurora deck ({} cards) — {metric_label} = {metric:.3}", total(b));
    for (id, c) in rows {
        if c == 0 {
            continue;
        }
        let card = db.get(id).unwrap();
        println!(
            "  {c} x {:<28} pitch {} cost {} power {}",
            card.name,
            card.pitch_value.map(|v| v.to_string()).unwrap_or("-".into()),
            card.cost,
            card.power.map(|p| p.to_string()).unwrap_or("-".into()),
        );
    }
    let (r, y, bl) = pitch_curve(db, b);
    println!("  pitch curve: {r} red(1) / {y} yellow(2) / {bl} blue(3)");
}

fn main() {
    let cfg = parse();
    let db = Rc::new(CardDb::load());
    let pool = build_pool(db.as_ref(), cfg.names);
    println!(
        "Optimizing Aurora deck | objective={} | {} candidate cards from {} names | {} games/eval",
        cfg.objective,
        pool.ids.len(),
        cfg.names,
        cfg.games
    );

    let mut build = initial_build(db.as_ref(), &pool);
    let (mut score, mut metric) = evaluate(&db, &build, &cfg.objective, cfg.games, cfg.seed);
    let label = if cfg.objective == "winrate" { "winrate%" } else { "avg turns to kill" };
    println!("start: {label} = {metric:.3}");

    for iter in 1..=cfg.max_iters {
        let remove_ids: Vec<String> =
            build.iter().filter(|(_, &c)| c > 0).map(|(id, _)| id.clone()).collect();

        let mut best: Option<(String, String, Build, f64, f64)> = None;
        for a in &remove_ids {
            for bcard in &pool.ids {
                if a == bcard {
                    continue;
                }
                // candidate = remove one a, add one b
                let mut cand = build.clone();
                *cand.get_mut(a).unwrap() -= 1;
                if cand[a] == 0 {
                    cand.remove(a);
                }
                // legality: per-name cap after the add
                let bname = &pool.name_of[bcard];
                if name_total(&cand, &pool, bname) + 1 > MAX_PER_NAME {
                    continue;
                }
                *cand.entry(bcard.clone()).or_insert(0) += 1;
                if total(&cand) != DECK_SIZE as u32 {
                    continue;
                }
                let (s, m) = evaluate(&db, &cand, &cfg.objective, cfg.games, cfg.seed);
                if s > score + 1e-9 && best.as_ref().map(|x| s > x.3).unwrap_or(true) {
                    best = Some((a.clone(), bcard.clone(), cand, s, m));
                }
            }
        }

        match best {
            Some((a, bcard, cand, s, m)) => {
                let an = db.get(&a).map(|c| c.name.clone()).unwrap_or(a.clone());
                let bn = db.get(&bcard).map(|c| c.name.clone()).unwrap_or(bcard.clone());
                println!(
                    "iter {iter:2}: -1 {an} [{a}]  +1 {bn} [{bcard}]  -> {label} {m:.3}"
                );
                build = cand;
                score = s;
                metric = m;
            }
            None => {
                println!("iter {iter:2}: no improving swap — local optimum reached.");
                break;
            }
        }
    }

    let _ = score;
    print_deck(db.as_ref(), &build, label, metric);
}
