//! Baseline agents. Swap in your own policy by implementing [`Agent`].

use crate::game::{Action, Agent, Game};
use crate::rng::Rng;

/// Passive target: never attacks, never blocks (Talishar combat-dummy behaviour).
pub struct CombatDummyAgent;

impl CombatDummyAgent {
    pub fn new() -> Self {
        CombatDummyAgent
    }
}
impl Default for CombatDummyAgent {
    fn default() -> Self {
        Self::new()
    }
}

impl Agent for CombatDummyAgent {
    fn name(&self) -> &str {
        "CombatDummy"
    }
    fn choose_action(&mut self, _g: &Game, _me: usize, _legal: &[Action]) -> Action {
        Action::EndTurn
    }
    fn choose_blocks(&mut self, _g: &Game, _me: usize, _power: i32, _dom: bool) -> Vec<usize> {
        vec![]
    }
}

/// Uniform-random legal action; blocks randomly.
pub struct RandomAgent {
    name: String,
    rng: Rng,
}
impl RandomAgent {
    pub fn new(name: &str, seed: u64) -> Self {
        RandomAgent { name: name.into(), rng: Rng::new(seed) }
    }
}
impl Agent for RandomAgent {
    fn name(&self) -> &str {
        &self.name
    }
    fn choose_action(&mut self, _g: &Game, _me: usize, legal: &[Action]) -> Action {
        legal[self.rng.below(legal.len())]
    }
    fn choose_blocks(&mut self, g: &Game, me: usize, _power: i32, dom: bool) -> Vec<usize> {
        let n = g.me(me).hand.len();
        if n == 0 {
            return vec![];
        }
        // Block with ~half the hand at random.
        let mut picks = Vec::new();
        for i in 0..n {
            if self.rng.below(2) == 0 {
                picks.push(i);
            }
        }
        if dom {
            picks.truncate(1);
        }
        picks
    }
}

/// Greedy aggression: play the highest-power affordable attack, preferring `go again`;
/// block only what's needed to survive lethal, using the lowest-value cards.
pub struct GreedyAttackAgent {
    name: String,
}
impl GreedyAttackAgent {
    pub fn new(name: &str) -> Self {
        GreedyAttackAgent { name: name.into() }
    }
}
impl Agent for GreedyAttackAgent {
    fn name(&self) -> &str {
        &self.name
    }

    fn choose_action(&mut self, g: &Game, me: usize, legal: &[Action]) -> Action {
        let mut best: Option<(Action, i64)> = None;
        for &a in legal {
            let Action::Play(i) = a else { continue };
            let Some(card) = g.me(me).hand.get(i).and_then(|id| g.card(id)) else {
                continue;
            };
            // Score: prefer attacks, higher power, and go again (keeps the turn alive).
            let power = card.power.unwrap_or(0) as i64;
            let go = if card.keywords.go_again { 100 } else { 0 };
            let attack_bonus = if card.is_attack() { 50 } else { 0 };
            let score = attack_bonus + power + go;
            if best.map(|(_, s)| score > s).unwrap_or(true) {
                best = Some((a, score));
            }
        }
        best.map(|(a, _)| a).unwrap_or(Action::EndTurn)
    }

    fn choose_blocks(&mut self, g: &Game, me: usize, power: i32, dom: bool) -> Vec<usize> {
        // Only block to avoid taking damage we can't afford; block from cheapest cards.
        if power <= 0 {
            return vec![];
        }
        let mut by_def: Vec<(usize, i32)> = g
            .me(me)
            .hand
            .iter()
            .enumerate()
            .map(|(i, id)| (i, g.card(id).map(|c| c.defense).unwrap_or(0)))
            .filter(|(_, d)| *d > 0)
            .collect();
        // Use smallest blockers first to soak the hit while keeping big threats.
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
