//! A linear (feature-weighted) policy agent whose weights can be learned by
//! self-play. This is the "policy" that the evolution-strategy trainer optimises.
//!
//! It is deliberately simple and transparent: every decision is a dot product of a
//! small feature vector with a learned weight vector, so you can read exactly what
//! strategy emerged. Swap this for a neural policy later behind the same `Agent`.

use crate::game::{Action, Agent, Game};
use std::io::{self, Write};

/// Number of learnable weights. See indices below.
pub const NW: usize = 9;

// Weight indices.
pub const W_ATTACK: usize = 0; // bias for playing an attack
pub const W_POWER: usize = 1; // per point of power
pub const W_GO_AGAIN: usize = 2; // bias for go again
pub const W_COST: usize = 3; // per point of cost (usually negative)
pub const W_PITCH: usize = 4; // per point of the card's own pitch value
pub const W_DRAW: usize = 5; // bias for on-hit draw
pub const W_LETHAL: usize = 6; // bias when power >= opponent life
pub const W_END: usize = 7; // score of ending the turn (the "pass" threshold)
pub const W_BLOCK_THRESHOLD: usize = 8; // block only if incoming power >= this

#[derive(Clone, Debug, PartialEq)]
pub struct Weights {
    pub w: [f64; NW],
}

impl Weights {
    /// A sensible hand-set starting point (roughly the greedy heuristic).
    pub fn baseline() -> Self {
        let mut w = [0.0; NW];
        w[W_ATTACK] = 5.0;
        w[W_POWER] = 1.0;
        w[W_GO_AGAIN] = 3.0;
        w[W_COST] = -0.5;
        w[W_PITCH] = -0.2;
        w[W_DRAW] = 1.0;
        w[W_LETHAL] = 25.0;
        w[W_END] = 0.0;
        w[W_BLOCK_THRESHOLD] = 3.0;
        Weights { w }
    }

    pub fn zeros() -> Self {
        Weights { w: [0.0; NW] }
    }

    /// Gaussian-perturbed copy (one ES mutation step).
    pub fn perturb(&self, rng: &mut crate::rng::Rng, sigma: f64) -> Self {
        let mut n = self.clone();
        for x in n.w.iter_mut() {
            *x += rng.gaussian() * sigma;
        }
        n
    }

    pub fn save(&self, path: &str) -> io::Result<()> {
        let mut f = std::fs::File::create(path)?;
        let line = self
            .w
            .iter()
            .map(|x| format!("{x:.6}"))
            .collect::<Vec<_>>()
            .join(",");
        writeln!(f, "{line}")
    }

    pub fn load(path: &str) -> io::Result<Self> {
        let s = std::fs::read_to_string(path)?;
        let mut w = [0.0; NW];
        for (i, tok) in s.trim().split(',').take(NW).enumerate() {
            w[i] = tok.trim().parse().unwrap_or(0.0);
        }
        Ok(Weights { w })
    }
}

pub struct LinearAgent {
    name: String,
    weights: Weights,
}

impl LinearAgent {
    pub fn new(name: &str, weights: Weights) -> Self {
        LinearAgent { name: name.into(), weights }
    }
}

impl Agent for LinearAgent {
    fn name(&self) -> &str {
        &self.name
    }

    fn choose_action(&mut self, g: &Game, me: usize, legal: &[Action]) -> Action {
        let w = &self.weights.w;
        let opp_life = g.opponent(me).life;
        let mut best: Option<(Action, f64)> = None;

        for &a in legal {
            let Action::Play(i) = a else { continue };
            let Some(card) = g.me(me).hand.get(i).and_then(|id| g.card(id)) else {
                continue;
            };
            let power = card.power.unwrap_or(0);
            let mut s = 0.0;
            if card.is_attack() {
                s += w[W_ATTACK];
                s += w[W_POWER] * power as f64;
                if power >= opp_life {
                    s += w[W_LETHAL];
                }
            }
            if card.keywords.go_again {
                s += w[W_GO_AGAIN];
            }
            s += w[W_COST] * card.cost as f64;
            s += w[W_PITCH] * card.pitch().value() as f64;
            if card.keywords.draws_on_hit {
                s += w[W_DRAW];
            }
            if best.map(|(_, bs)| s > bs).unwrap_or(true) {
                best = Some((a, s));
            }
        }

        match best {
            Some((a, s)) if s > w[W_END] => a,
            _ => Action::EndTurn,
        }
    }

    fn choose_blocks(&mut self, g: &Game, me: usize, power: i32, dom: bool) -> Vec<usize> {
        if (power as f64) < self.weights.w[W_BLOCK_THRESHOLD] {
            return vec![];
        }
        // Soak the hit with the fewest, lowest-value blockers.
        let mut by_def: Vec<(usize, i32)> = g
            .me(me)
            .hand
            .iter()
            .enumerate()
            .map(|(i, id)| (i, g.card(id).map(|c| c.defense).unwrap_or(0)))
            .filter(|(_, d)| *d > 0)
            .collect();
        by_def.sort_by_key(|(_, d)| *d);
        let mut chosen = Vec::new();
        let mut soaked = 0;
        for (i, d) in by_def {
            if soaked >= power {
                break;
            }
            chosen.push(i);
            soaked += d;
            if dom {
                break;
            }
        }
        chosen
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::RandomAgent;
    use crate::{Deck, Game, Outcome};
    use fab_cards::CardDb;

    #[test]
    fn baseline_policy_beats_random() {
        let db = CardDb::load();
        let mut wins = 0;
        let n = 80;
        for k in 0..n {
            let decks = [Deck::aurora_sample(&db), Deck::aurora_sample(&db)];
            let mut g = Game::new(db.clone(), decks, 1000 + k);
            let mut lin = LinearAgent::new("linear", Weights::baseline());
            let mut rnd = RandomAgent::new("random", 7 * k + 1);
            if let Outcome::Win(0) = g.run(&mut lin, &mut rnd) {
                wins += 1;
            }
        }
        assert!(wins * 100 / n >= 70, "linear should beat random badly, won {wins}/{n}");
    }

    #[test]
    fn weights_roundtrip() {
        let w = Weights::baseline();
        let path = std::env::temp_dir().join("fab_w_test.csv");
        let p = path.to_str().unwrap();
        w.save(p).unwrap();
        assert_eq!(Weights::load(p).unwrap(), w);
    }
}
