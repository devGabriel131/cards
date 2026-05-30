//! Self-play trainer: a (1 + λ) evolution strategy that improves the linear
//! policy by having candidates play **against the current champion** (self-play).
//! Fitness is win rate vs. the champion; the champion is replaced when beaten.
//!
//! No ML dependencies — this is the dependency-free path to "an agent that learns
//! to master the game from self-play". The same `Agent` seam accepts a neural
//! policy later (needs ML crates / network).
//!
//! Usage:
//!   fab-train                                  # default run
//!   fab-train --generations 60 --lambda 8 --games 40 --sigma 0.25 --seed 1
//!   fab-train --out learned_weights.csv
//!
//! Prints a learning curve: champion win rate vs the fixed Greedy baseline.

use fab_cards::CardDb;
use fab_engine::agents::{GreedyAttackAgent, RandomAgent};
use fab_engine::{Deck, Game, LinearAgent, Outcome, Rng, Weights};

struct Cfg {
    generations: u32,
    lambda: u32,
    games: u32,    // games per candidate vs champion
    eval_games: u32,
    sigma: f64,
    seed: u64,
    out: String,
}

fn parse() -> Cfg {
    let mut c = Cfg {
        generations: 40,
        lambda: 8,
        games: 30,
        eval_games: 200,
        sigma: 0.3,
        seed: 1,
        out: "learned_weights.csv".to_string(),
    };
    let mut it = std::env::args().skip(1);
    while let Some(a) = it.next() {
        let mut next = || it.next().unwrap_or_default();
        match a.as_str() {
            "--generations" => c.generations = next().parse().unwrap_or(c.generations),
            "--lambda" => c.lambda = next().parse().unwrap_or(c.lambda),
            "--games" => c.games = next().parse().unwrap_or(c.games),
            "--eval-games" => c.eval_games = next().parse().unwrap_or(c.eval_games),
            "--sigma" => c.sigma = next().parse().unwrap_or(c.sigma),
            "--seed" => c.seed = next().parse().unwrap_or(c.seed),
            "--out" => c.out = next(),
            "--help" | "-h" => {
                println!("fab-train [--generations N] [--lambda L] [--games K] [--eval-games E] [--sigma S] [--seed X] [--out FILE]");
                std::process::exit(0);
            }
            _ => {}
        }
    }
    c
}

/// Play one game between two weight sets (seat-determined), return true if P0 won.
fn play_game(db: &CardDb, p0: &Weights, p1: &Weights, seed: u64) -> Option<usize> {
    let decks = [Deck::aurora_sample(db), Deck::aurora_sample(db)];
    let mut g = Game::new(db.clone(), decks, seed);
    let mut a0 = LinearAgent::new("p0", p0.clone());
    let mut a1 = LinearAgent::new("p1", p1.clone());
    match g.run(&mut a0, &mut a1) {
        Outcome::Win(w) => Some(w),
        Outcome::Draw => None,
    }
}

/// Win rate of `cand` vs `opp` over `n` games, alternating seats to remove
/// first-player bias.
fn match_winrate(db: &CardDb, cand: &Weights, opp: &Weights, n: u32, rng: &mut Rng) -> f64 {
    let mut wins = 0.0;
    let mut counted = 0.0;
    for k in 0..n {
        let seed = rng.next_u64();
        let cand_first = k % 2 == 0;
        let (a, b) = if cand_first { (cand, opp) } else { (opp, cand) };
        if let Some(winner) = play_game(db, a, b, seed) {
            let cand_seat = if cand_first { 0 } else { 1 };
            if winner == cand_seat {
                wins += 1.0;
            }
            counted += 1.0;
        }
    }
    if counted == 0.0 {
        0.5
    } else {
        wins / counted
    }
}

/// Champion win rate vs a fixed baseline agent (the learning-curve metric).
fn winrate_vs_baseline(db: &CardDb, champ: &Weights, n: u32, rng: &mut Rng, random: bool) -> f64 {
    let mut wins = 0.0;
    for k in 0..n {
        let seed = rng.next_u64();
        let champ_first = k % 2 == 0;
        let decks = [Deck::aurora_sample(db), Deck::aurora_sample(db)];
        let mut g = Game::new(db.clone(), decks, seed);
        let mut champ_agent = LinearAgent::new("champ", champ.clone());
        let mut greedy = GreedyAttackAgent::new("greedy");
        let mut rnd = RandomAgent::new("random", seed ^ 0xDEAD);
        let base: &mut dyn fab_engine::Agent = if random { &mut rnd } else { &mut greedy };
        let champ_seat = if champ_first { 0 } else { 1 };
        let outcome = if champ_first {
            g.run(&mut champ_agent, base)
        } else {
            g.run(base, &mut champ_agent)
        };
        if let Outcome::Win(w) = outcome {
            if w == champ_seat {
                wins += 1.0;
            }
        }
    }
    wins / n as f64
}

fn main() {
    let cfg = parse();
    let db = CardDb::load();
    let mut rng = Rng::new(cfg.seed);

    let mut champion = Weights::baseline();
    println!(
        "Self-play ES training: {} generations, λ={}, {} games/candidate, σ={}",
        cfg.generations, cfg.lambda, cfg.games, cfg.sigma
    );
    let start_vs_greedy = winrate_vs_baseline(&db, &champion, cfg.eval_games, &mut rng, false);
    let start_vs_random = winrate_vs_baseline(&db, &champion, cfg.eval_games, &mut rng, true);
    println!(
        "gen   0 | champion vs greedy {:5.1}% | vs random {:5.1}% | (baseline weights)",
        100.0 * start_vs_greedy,
        100.0 * start_vs_random
    );

    let mut sigma = cfg.sigma;
    for gen in 1..=cfg.generations {
        // Generate λ offspring; keep the one that best beats the champion (>50%).
        let mut best_child: Option<Weights> = None;
        let mut best_wr = 0.5_f64;
        for _ in 0..cfg.lambda {
            let child = champion.perturb(&mut rng, sigma);
            let wr = match_winrate(&db, &child, &champion, cfg.games, &mut rng);
            if wr > best_wr {
                best_wr = wr;
                best_child = Some(child);
            }
        }
        if let Some(c) = best_child {
            champion = c; // self-play improvement step
        } else {
            sigma *= 0.9; // no improvement: anneal exploration
        }

        if gen % 5 == 0 || gen == cfg.generations {
            let g_wr = winrate_vs_baseline(&db, &champion, cfg.eval_games, &mut rng, false);
            let r_wr = winrate_vs_baseline(&db, &champion, cfg.eval_games, &mut rng, true);
            println!(
                "gen {gen:3} | champion vs greedy {:5.1}% | vs random {:5.1}% | best-vs-champ {:4.1}% σ={:.3}",
                100.0 * g_wr,
                100.0 * r_wr,
                100.0 * best_wr,
                sigma
            );
        }
    }

    match champion.save(&cfg.out) {
        Ok(_) => println!("\nSaved learned weights to {}", cfg.out),
        Err(e) => eprintln!("Failed to save weights: {e}"),
    }
    println!("Final weights: {:?}", champion.w);
}
