//! The card model and keyword detection.

use crate::types::*;
use std::str::FromStr;

/// Keywords relevant to simulation, parsed from the rules text.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Keywords {
    pub go_again: bool,
    pub dominate: bool,
    pub intimidate: bool,
    pub overpower: bool,
    pub draws_on_hit: bool,
}

impl Keywords {
    fn parse(text: &str) -> Self {
        let t = text.to_ascii_lowercase();
        Keywords {
            go_again: t.contains("go again"),
            dominate: t.contains("dominate"),
            intimidate: t.contains("intimidate"),
            overpower: t.contains("overpower"),
            // "if this hits ... draw a card" style on-hit draw
            draws_on_hit: t.contains("hits")
                && t.contains("draw a card")
                || t.contains("if this hits a hero, draw a card"),
        }
    }
}

/// A single card definition (one pitch variant = one card).
#[derive(Debug, Clone)]
pub struct Card {
    pub id: String,
    pub name: String,
    pub pitch_value: Option<u8>,
    pub cost: u8,
    pub power: Option<i32>,
    pub defense: i32,
    pub types: Vec<CardType>,
    pub subtypes: Vec<String>,
    pub classes: Vec<Class>,
    pub talents: Vec<Talent>,
    pub keywords: Keywords,
    pub text: String,
}

impl Card {
    pub fn pitch(&self) -> Pitch {
        Pitch::from_value(self.pitch_value)
    }
    pub fn is_type(&self, t: CardType) -> bool {
        self.types.contains(&t)
    }
    pub fn is_attack(&self) -> bool {
        self.subtypes.iter().any(|s| s == "Attack")
            || (self.is_type(CardType::Action) && self.power.is_some())
    }
    pub fn is_hero(&self) -> bool {
        self.is_type(CardType::Hero)
    }
    pub fn is_equipment(&self) -> bool {
        self.is_type(CardType::Equipment)
    }
    pub fn is_weapon(&self) -> bool {
        self.is_type(CardType::Weapon)
    }
    pub fn equip_slot(&self) -> Option<EquipSlot> {
        self.subtypes.iter().find_map(|s| EquipSlot::from_subtype(s))
    }
    pub fn has_talent(&self, t: Talent) -> bool {
        self.talents.contains(&t)
    }
    pub fn has_class(&self, c: Class) -> bool {
        self.classes.contains(&c)
    }

    /// Parse one TSV record (see `aurora_cards.tsv`).
    pub(crate) fn from_tsv(line: &str) -> Option<Self> {
        let f: Vec<&str> = line.split('\t').collect();
        if f.len() < 11 {
            return None;
        }
        let list = |s: &str| -> Vec<String> {
            if s.is_empty() {
                vec![]
            } else {
                s.split('|').map(|x| x.to_string()).collect()
            }
        };
        let num = |s: &str| -> Option<i32> { s.parse().ok() };

        let text = f[10].to_string();
        Some(Card {
            id: f[0].to_string(),
            name: f[1].to_string(),
            pitch_value: f[2].parse().ok(),
            cost: num(f[3]).unwrap_or(0).max(0) as u8,
            power: num(f[4]),
            defense: num(f[5]).unwrap_or(0),
            types: list(f[6])
                .iter()
                .filter_map(|s| CardType::from_str(s).ok())
                .collect(),
            subtypes: list(f[7]),
            classes: list(f[8])
                .iter()
                .filter_map(|s| Class::from_str(s).ok())
                .collect(),
            talents: list(f[9])
                .iter()
                .filter_map(|s| Talent::from_str(s).ok())
                .collect(),
            keywords: Keywords::parse(&text),
            text,
        })
    }
}
