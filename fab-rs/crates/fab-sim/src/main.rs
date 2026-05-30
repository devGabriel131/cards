//! Command-line simulator.
//!
//! Usage:
//!   fab-sim                       # 1000 Aurora-vs-dummy goldfish games, summary
//!   fab-sim --games 500 --seed 1
//!   fab-sim --verbose             # play one game with a full event log
//!   fab-sim --mirror              # Aurora (greedy) vs Aurora (random)

use fab_cards::CardDb;
use fab_engine::agents::{CombatDummyAgent, GreedyAttackAgent, RandomAgent};
use fab_engine::{Deck, Game, Outcome};

struct Args {
    games: u32,
    seed: u64,
    verbose: bool,
    mirror: bool,
}

fn parse_args() -> Args {
    let mut a = Args { games: 1000, seed: 1, verbose: false, mirror: false };
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--games" => a.games = it.next().and_then(|s| s.parse().ok()).unwrap_or(a.games),
            "--seed" => a.seed = it.next().and_then(|s| s.parse().ok()).unwrap_or(a.seed),
            "--verbose" => a.verbose = true,
            "--mirror" => a.mirror = true,
            "--help" | "-h" => {
                println!("fab-sim [--games N] [--seed S] [--verbose] [--mirror]");
                std::process::exit(0);
            }
            other => eprintln!("ignoring unknown arg: {other}"),
        }
    }
    a
}

fn main() {
    let args = parse_args();
    let db = CardDb::load();
    println!("Loaded {} cards (Aurora / Runeblade / Lightning pool).", db.len());

    if args.verbose {
        // One narrated game.
        let aurora = Deck::aurora_sample(&db);
        let dummy = Deck::combat_dummy(40);
        let mut g = Game::new(db, [aurora, dummy], args.seed);
        g.verbose = true;
        let mut a = GreedyAttackAgent::new("Aurora");
        let mut d = CombatDummyAgent::new();
        let o = g.run(&mut a, &mut d);
        println!("\nResult: {o:?} in {} turns", g.turn);
        return;
    }

    let mut wins0 = 0u32;
    let mut wins1 = 0u32;
    let mut draws = 0u32;
    let mut total_turns = 0u64;
    let mut fastest = u32::MAX;
    let mut slowest = 0u32;

    for n in 0..args.games {
        let db = CardDb::load();
        let aurora = Deck::aurora_sample(&db);
        let seed = args.seed.wrapping_add(n as u64).wrapping_mul(0x100_0001);

        let (outcome, turns) = if args.mirror {
            let other = Deck::aurora_sample(&db);
            let mut g = Game::new(db, [aurora, other], seed);
            let mut a = GreedyAttackAgent::new("Aurora-Greedy");
            let mut b = RandomAgent::new("Aurora-Random", seed ^ 0xABCD);
            let o = g.run(&mut a, &mut b);
            (o, g.turn)
        } else {
            let dummy = Deck::combat_dummy(40);
            let mut g = Game::new(db, [aurora, dummy], seed);
            let mut a = GreedyAttackAgent::new("Aurora");
            let mut d = CombatDummyAgent::new();
            let o = g.run(&mut a, &mut d);
            (o, g.turn)
        };

        match outcome {
            Outcome::Win(0) => wins0 += 1,
            Outcome::Win(_) => wins1 += 1,
            Outcome::Draw => draws += 1,
        }
        total_turns += turns as u64;
        fastest = fastest.min(turns);
        slowest = slowest.max(turns);
    }

    let g = args.games.max(1);
    let mode = if args.mirror { "Aurora(greedy) vs Aurora(random)" } else { "Aurora goldfish vs Combat Dummy" };
    println!("\n=== {mode} : {} games ===", args.games);
    println!("P0 (Aurora) wins: {wins0}  ({:.1}%)", 100.0 * wins0 as f64 / g as f64);
    println!("P1 wins:          {wins1}  ({:.1}%)", 100.0 * wins1 as f64 / g as f64);
    println!("Draws:            {draws}");
    println!("Turns: avg {:.2}, fastest {}, slowest {}", total_turns as f64 / g as f64, fastest, slowest);
}
