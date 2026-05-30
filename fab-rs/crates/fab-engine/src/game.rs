//! A simplified but self-consistent Flesh and Blood rules engine.
//!
//! Scope (intentionally a faithful *subset*, not a full Talishar reimplementation):
//! turn structure, the single action point + `go again`, pitching cards to pay
//! costs, attacks vs. blocks on a one-link combat step, `dominate`, on-hit draw,
//! hand refill to intellect, and life/fatigue loss conditions. Arcane damage,
//! triggered layers, reactions, and most card-specific text are not modelled —
//! see README for the boundary.

use crate::deck::Deck;
use crate::rng::Rng;
use fab_cards::{Card, CardDb};
use std::collections::VecDeque;

/// An action a player can take during their action phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Play the hand card at this index.
    Play(usize),
    EndTurn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Win(usize),
    Draw,
}

/// One player's board state.
#[derive(Debug, Clone)]
pub struct PlayerState {
    pub hero: String,
    pub life: i32,
    pub intellect: u8,
    pub deck: VecDeque<String>,
    pub hand: Vec<String>,
    pub arsenal: Option<String>,
    pub pitch: Vec<String>,
    pub graveyard: Vec<String>,
    pub equipment: Vec<String>,
    pub weapon: Option<String>,
    pub action_points: u8,
}

impl PlayerState {
    fn from_deck(d: &Deck, rng: &mut Rng) -> Self {
        let mut cards = d.cards.clone();
        rng.shuffle(&mut cards);
        PlayerState {
            hero: d.hero.clone(),
            life: d.life,
            intellect: d.intellect,
            deck: cards.into_iter().collect(),
            hand: Vec::new(),
            arsenal: None,
            pitch: Vec::new(),
            graveyard: Vec::new(),
            equipment: d.equipment.clone(),
            weapon: d.weapon.clone(),
            action_points: 0,
        }
    }

    fn exhausted(&self) -> bool {
        self.deck.is_empty() && self.hand.is_empty() && self.arsenal.is_none()
    }
}

/// Agents decide actions and blocks. They get a read-only `&Game` plus their seat.
pub trait Agent {
    fn name(&self) -> &str;
    fn choose_action(&mut self, game: &Game, me: usize, legal: &[Action]) -> Action;
    /// Return hand indices (of `me`) to commit as blocks against `incoming_power`.
    fn choose_blocks(
        &mut self,
        game: &Game,
        me: usize,
        incoming_power: i32,
        dominate: bool,
    ) -> Vec<usize>;
}

pub struct Game {
    db: CardDb,
    pub players: [PlayerState; 2],
    pub active: usize,
    pub turn: u32,
    pub max_turns: u32,
    pub log: Vec<String>,
    pub verbose: bool,
}

impl Game {
    pub fn new(db: CardDb, decks: [Deck; 2], seed: u64) -> Self {
        let mut rng = Rng::new(seed);
        let mut players = [
            PlayerState::from_deck(&decks[0], &mut rng),
            PlayerState::from_deck(&decks[1], &mut rng),
        ];
        // Opening hands: draw to intellect.
        for p in players.iter_mut() {
            for _ in 0..p.intellect {
                if let Some(c) = p.deck.pop_front() {
                    p.hand.push(c);
                }
            }
        }
        Game {
            db,
            players,
            active: 0,
            turn: 0,
            max_turns: 200,
            log: Vec::new(),
            verbose: false,
        }
    }

    // ---- read helpers for agents ----
    pub fn db(&self) -> &CardDb {
        &self.db
    }
    pub fn card(&self, id: &str) -> Option<&Card> {
        self.db.get(id)
    }
    pub fn me(&self, p: usize) -> &PlayerState {
        &self.players[p]
    }
    pub fn opponent(&self, p: usize) -> &PlayerState {
        &self.players[1 - p]
    }

    fn logln(&mut self, s: String) {
        if self.verbose {
            println!("{s}");
        }
        self.log.push(s);
    }

    /// Total pitch available in a hand, excluding one index (the card being played).
    fn pitchable_excluding(&self, p: usize, exclude: usize) -> u8 {
        self.players[p]
            .hand
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != exclude)
            .map(|(_, id)| self.db.get(id).map(|c| c.pitch().value()).unwrap_or(0))
            .sum()
    }

    /// Legal actions for the active player.
    pub fn legal_actions(&self) -> Vec<Action> {
        let p = self.active;
        let mut acts = vec![Action::EndTurn];
        for (i, id) in self.players[p].hand.iter().enumerate() {
            let Some(card) = self.db.get(id) else { continue };
            let first_type = card.types.first().copied().unwrap_or(fab_cards::CardType::Other);
            let playable_type = matches!(
                first_type,
                fab_cards::CardType::Action | fab_cards::CardType::Instant
            ) || card.is_attack();
            if !playable_type {
                continue;
            }
            // Actions/attacks need an action point; instants do not.
            let is_action = card.is_type(fab_cards::CardType::Action) || card.is_attack();
            if is_action && self.players[p].action_points == 0 {
                continue;
            }
            if card.cost as u8 > self.pitchable_excluding(p, i) {
                continue;
            }
            acts.push(Action::Play(i));
        }
        acts
    }

    fn draw(&mut self, p: usize) -> Option<String> {
        if let Some(c) = self.players[p].deck.pop_front() {
            self.players[p].hand.push(c.clone());
            Some(c)
        } else {
            None
        }
    }

    /// Pitch cards from `p`'s hand (already excluding the played card) to cover `cost`.
    fn pay_cost(&mut self, p: usize, cost: u8) -> bool {
        if cost == 0 {
            return true;
        }
        let mut cand: Vec<(usize, u8)> = self.players[p]
            .hand
            .iter()
            .enumerate()
            .filter_map(|(i, id)| {
                let v = self.db.get(id).map(|c| c.pitch().value()).unwrap_or(0);
                (v > 0).then_some((i, v))
            })
            .collect();
        // Fewest cards first: pitch highest value.
        cand.sort_by(|a, b| b.1.cmp(&a.1));
        let mut got = 0u8;
        let mut chosen = Vec::new();
        for (i, v) in cand {
            if got >= cost {
                break;
            }
            chosen.push(i);
            got += v;
        }
        if got < cost {
            return false;
        }
        chosen.sort_unstable_by(|a, b| b.cmp(a)); // remove high indices first
        for i in chosen {
            let id = self.players[p].hand.remove(i);
            self.players[p].pitch.push(id);
        }
        true
    }

    fn play_from_hand(&mut self, idx: usize, me: usize, a0: &mut dyn Agent, a1: &mut dyn Agent) {
        if idx >= self.players[me].hand.len() {
            return;
        }
        let id = self.players[me].hand.remove(idx);
        let card = match self.db.get(&id) {
            Some(c) => c.clone(),
            None => return,
        };

        if !self.pay_cost(me, card.cost) {
            // Should not happen (legality checked); refund and bail.
            self.players[me].hand.insert(idx, id);
            return;
        }

        let go_again = card.keywords.go_again;

        if card.is_attack() {
            let power = card.power.unwrap_or(0);
            let defender = 1 - me;
            let dominate = card.keywords.dominate;
            let block_idxs = {
                let agent: &mut dyn Agent = if defender == 0 { a0 } else { a1 };
                agent.choose_blocks(self, defender, power, dominate)
            };
            // Validate, dedup, clamp for dominate.
            let mut seen = std::collections::HashSet::new();
            let mut valid: Vec<usize> = block_idxs
                .into_iter()
                .filter(|i| *i < self.players[defender].hand.len() && seen.insert(*i))
                .collect();
            if dominate {
                valid.truncate(1);
            }
            let total_block: i32 = valid
                .iter()
                .map(|i| {
                    let bid = &self.players[defender].hand[*i];
                    self.db.get(bid).map(|c| c.defense).unwrap_or(0)
                })
                .sum();
            // Move blocking cards to graveyard (high indices first).
            valid.sort_unstable_by(|a, b| b.cmp(a));
            for i in valid {
                let bid = self.players[defender].hand.remove(i);
                self.players[defender].graveyard.push(bid);
            }

            let damage = (power - total_block).max(0);
            let hit = damage > 0;
            if hit {
                self.players[defender].life -= damage;
            }
            self.logln(format!(
                "  P{me} plays {} ({}p) vs {total_block} block -> {damage} dmg{} [P{defender} life {}]",
                card.name,
                power,
                if hit { " HIT" } else { "" },
                self.players[defender].life
            ));
            if hit && card.keywords.draws_on_hit {
                self.draw(me);
            }
        } else {
            self.logln(format!("  P{me} plays {} (non-attack)", card.name));
        }

        self.players[me].graveyard.push(id);

        if !go_again {
            self.players[me].action_points = self.players[me].action_points.saturating_sub(1);
        }
    }

    fn end_turn(&mut self) {
        let p = self.active;
        // Pitched cards go to the bottom of the deck.
        let pitched: Vec<String> = self.players[p].pitch.drain(..).collect();
        for c in pitched {
            self.players[p].deck.push_back(c);
        }
        // Refill hand to intellect.
        while self.players[p].hand.len() < self.players[p].intellect as usize {
            if self.draw(p).is_none() {
                break;
            }
        }
        self.active = 1 - self.active;
    }

    pub fn check_winner(&self) -> Option<Outcome> {
        match (self.players[0].life <= 0, self.players[1].life <= 0) {
            (true, true) => Some(Outcome::Draw),
            (false, true) => Some(Outcome::Win(0)),
            (true, false) => Some(Outcome::Win(1)),
            (false, false) => None,
        }
    }

    fn take_turn(&mut self, a0: &mut dyn Agent, a1: &mut dyn Agent) {
        let p = self.active;
        self.turn += 1;
        self.players[p].action_points = 1;
        self.logln(format!(
            "Turn {} - P{p} ({}) [life {} vs {}]",
            self.turn, self.players[p].hero, self.players[p].life, self.players[1 - p].life
        ));

        let mut guard = 0;
        loop {
            guard += 1;
            if guard > 100 {
                break; // safety against pathological loops
            }
            let legal = self.legal_actions();
            let action = {
                let agent: &mut dyn Agent = if p == 0 { a0 } else { a1 };
                agent.choose_action(self, p, &legal)
            };
            let action = if legal.contains(&action) {
                action
            } else {
                Action::EndTurn
            };
            match action {
                Action::EndTurn => break,
                Action::Play(i) => self.play_from_hand(i, p, a0, a1),
            }
            if self.check_winner().is_some() {
                return;
            }
            if self.players[p].action_points == 0 {
                break;
            }
        }
        self.end_turn();
    }

    /// Run a full game between two agents; returns the outcome.
    pub fn run(&mut self, a0: &mut dyn Agent, a1: &mut dyn Agent) -> Outcome {
        loop {
            if let Some(o) = self.check_winner() {
                return o;
            }
            if self.turn >= self.max_turns {
                return Outcome::Draw;
            }
            // A passive target (intellect 0, no cards) is a pure damage dummy: it
            // never takes a turn and never fatigues — it only loses at 0 life.
            if self.players[self.active].intellect == 0 && self.players[self.active].exhausted() {
                self.active = 1 - self.active;
                continue;
            }
            // A real player who cannot present a turn loses to fatigue.
            if self.players[self.active].exhausted() {
                return Outcome::Win(1 - self.active);
            }
            self.take_turn(a0, a1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::{CombatDummyAgent, GreedyAttackAgent};

    fn aurora_vs_dummy(seed: u64) -> (Outcome, u32) {
        let db = CardDb::load();
        let aurora = Deck::aurora_sample(&db);
        let dummy = Deck::combat_dummy(40);
        let mut g = Game::new(db, [aurora, dummy], seed);
        let mut a = GreedyAttackAgent::new("Aurora");
        let mut d = CombatDummyAgent::new();
        let o = g.run(&mut a, &mut d);
        (o, g.turn)
    }

    #[test]
    fn aurora_kills_the_dummy() {
        let (o, turns) = aurora_vs_dummy(42);
        assert_eq!(o, Outcome::Win(0), "Aurora should always beat a passive dummy");
        assert!(turns > 0 && turns < 60, "unreasonable turn count: {turns}");
    }

    #[test]
    fn deterministic_for_same_seed() {
        assert_eq!(aurora_vs_dummy(7), aurora_vs_dummy(7));
    }

    #[test]
    fn legal_actions_always_include_end_turn() {
        let db = CardDb::load();
        let g = Game::new(db, [Deck::aurora_sample(&CardDb::load()), Deck::combat_dummy(40)], 1);
        assert!(g.legal_actions().contains(&Action::EndTurn));
    }
}
