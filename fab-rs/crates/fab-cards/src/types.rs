//! Core card enums, ported from `@flesh-and-blood/types`.

use std::str::FromStr;

/// Card super-type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CardType {
    Action,
    AttackReaction,
    Block,
    Companion,
    DefenseReaction,
    DemiHero,
    Equipment,
    Hero,
    Instant,
    Macro,
    Mentor,
    Resource,
    Token,
    Weapon,
    Other,
}

impl FromStr for CardType {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, ()> {
        Ok(match s {
            "Action" => Self::Action,
            "Attack Reaction" | "AttackReaction" => Self::AttackReaction,
            "Block" => Self::Block,
            "Companion" => Self::Companion,
            "Defense Reaction" | "DefenseReaction" => Self::DefenseReaction,
            "Demi-Hero" | "DemiHero" => Self::DemiHero,
            "Equipment" => Self::Equipment,
            "Hero" => Self::Hero,
            "Instant" => Self::Instant,
            "Macro" => Self::Macro,
            "Mentor" => Self::Mentor,
            "Resource" => Self::Resource,
            "Token" => Self::Token,
            "Weapon" => Self::Weapon,
            _ => Self::Other,
        })
    }
}

/// Hero / card class.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Class {
    Generic,
    NotClassed,
    Runeblade,
    Wizard,
    Warrior,
    Brute,
    Guardian,
    Ninja,
    Other,
}

impl FromStr for Class {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, ()> {
        Ok(match s {
            "Generic" => Self::Generic,
            "NotClassed" => Self::NotClassed,
            "Runeblade" => Self::Runeblade,
            "Wizard" => Self::Wizard,
            "Warrior" => Self::Warrior,
            "Brute" => Self::Brute,
            "Guardian" => Self::Guardian,
            "Ninja" => Self::Ninja,
            _ => Self::Other,
        })
    }
}

/// Card talent (e.g. Aurora is Lightning).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Talent {
    Lightning,
    Elemental,
    Ice,
    Earth,
    Light,
    Shadow,
    Other,
}

impl FromStr for Talent {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, ()> {
        Ok(match s {
            "Lightning" => Self::Lightning,
            "Elemental" => Self::Elemental,
            "Ice" => Self::Ice,
            "Earth" => Self::Earth,
            "Light" => Self::Light,
            "Shadow" => Self::Shadow,
            _ => Self::Other,
        })
    }
}

/// Equipment slot, derived from subtypes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EquipSlot {
    Head,
    Chest,
    Arms,
    Legs,
}

impl EquipSlot {
    pub fn from_subtype(s: &str) -> Option<Self> {
        Some(match s {
            "Head" => Self::Head,
            "Chest" => Self::Chest,
            "Arms" => Self::Arms,
            "Legs" => Self::Legs,
            _ => return None,
        })
    }
}

/// Pitch colour, by the dotted value on the card.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Pitch {
    Red,    // 1
    Yellow, // 2
    Blue,   // 3
    None,
}

impl Pitch {
    pub fn from_value(v: Option<u8>) -> Self {
        match v {
            Some(1) => Self::Red,
            Some(2) => Self::Yellow,
            Some(3) => Self::Blue,
            _ => Self::None,
        }
    }
    /// Resources generated when pitched.
    pub fn value(self) -> u8 {
        match self {
            Self::Red => 1,
            Self::Yellow => 2,
            Self::Blue => 3,
            Self::None => 0,
        }
    }
}
