//! `fab-cards`: Flesh and Blood card data for the Aurora (Runeblade / Lightning) pool.
//!
//! The card data is embedded at compile time from `aurora_cards.tsv`, which was
//! generated from `@flesh-and-blood/cards` (every card legal for the new Aurora,
//! `Hero.Aurora2`). Zero external dependencies.

pub mod card;
pub mod types;

pub use card::{Card, Keywords};
pub use types::*;

use std::collections::HashMap;

const AURORA_TSV: &str = include_str!("aurora_cards.tsv");

/// In-memory card database, indexed by card identifier.
#[derive(Debug, Clone)]
pub struct CardDb {
    by_id: HashMap<String, Card>,
}

impl CardDb {
    /// Load the embedded Aurora pool.
    pub fn load() -> Self {
        let mut by_id = HashMap::new();
        for line in AURORA_TSV.lines() {
            if line.trim().is_empty() {
                continue;
            }
            if let Some(card) = Card::from_tsv(line) {
                by_id.insert(card.id.clone(), card);
            }
        }
        CardDb { by_id }
    }

    pub fn get(&self, id: &str) -> Option<&Card> {
        self.by_id.get(id)
    }

    pub fn len(&self) -> usize {
        self.by_id.len()
    }
    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Card> {
        self.by_id.values()
    }

    /// All identifiers whose card has the given type.
    pub fn ids_of_type(&self, t: CardType) -> Vec<&str> {
        let mut v: Vec<&str> = self
            .by_id
            .values()
            .filter(|c| c.is_type(t))
            .map(|c| c.id.as_str())
            .collect();
        v.sort_unstable();
        v
    }
}

impl Default for CardDb {
    fn default() -> Self {
        Self::load()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_full_pool() {
        let db = CardDb::load();
        assert!(db.len() > 900, "expected ~1024 cards, got {}", db.len());
    }

    #[test]
    fn aurora_hero_present_and_correct() {
        let db = CardDb::load();
        let hero = db.get("aurora-legacy-of-tempest").expect("Aurora hero");
        assert!(hero.is_hero());
        assert!(hero.has_class(Class::Runeblade));
        assert!(hero.has_talent(Talent::Lightning));
    }

    #[test]
    fn stormshard_is_lightning_instant() {
        let db = CardDb::load();
        let c = db.get("stormshard-red").expect("Stormshard");
        assert!(c.has_talent(Talent::Lightning));
        assert_eq!(c.pitch(), Pitch::Red);
    }

    #[test]
    fn attacks_have_power_and_pitch() {
        let db = CardDb::load();
        let c = db.get("adrenaline-rush-red").expect("Adrenaline Rush");
        assert!(c.is_attack());
        assert_eq!(c.power, Some(4));
        assert_eq!(c.pitch().value(), 1);
    }
}
