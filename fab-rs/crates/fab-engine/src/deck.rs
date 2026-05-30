//! Deck model + a sample Aurora deck and the combat-dummy target.

use fab_cards::{CardDb, CardType, EquipSlot};

/// A complete registered deck: hero, equipment, weapon, and the shuffled main deck.
#[derive(Debug, Clone)]
pub struct Deck {
    pub hero: String,
    pub life: i32,
    pub intellect: u8,
    pub weapon: Option<String>,
    pub equipment: Vec<String>,
    pub cards: Vec<String>,
}

impl Deck {
    /// Build a runnable (not optimised) Aurora, Legacy of Tempest deck from the pool:
    /// hero + one weapon + one equipment per slot + 60 attack cards.
    pub fn aurora_sample(db: &CardDb) -> Self {
        let weapon = db.ids_of_type(CardType::Weapon).first().map(|s| s.to_string());

        let mut equipment = Vec::new();
        for slot in [EquipSlot::Head, EquipSlot::Chest, EquipSlot::Arms, EquipSlot::Legs] {
            if let Some(id) = db
                .iter()
                .filter(|c| c.is_equipment() && c.equip_slot() == Some(slot))
                .map(|c| c.id.clone())
                .min()
            {
                equipment.push(id);
            }
        }

        // 60 attack cards, deterministic order, cycled if the pool is short.
        let mut attacks: Vec<String> = db
            .iter()
            .filter(|c| c.is_attack() && c.power.unwrap_or(0) > 0)
            .map(|c| c.id.clone())
            .collect();
        attacks.sort();
        let mut cards = Vec::with_capacity(60);
        if !attacks.is_empty() {
            for i in 0..60 {
                cards.push(attacks[i % attacks.len()].clone());
            }
        }

        Deck {
            hero: "aurora-legacy-of-tempest".to_string(),
            life: 40,
            intellect: 4,
            weapon,
            equipment,
            cards,
        }
    }

    /// The passive "Combat Dummy" — a fixed-life target that never acts, mirroring
    /// Talishar's deck-test dummy. Good for measuring goldfish (kill-speed) only.
    pub fn combat_dummy(life: i32) -> Self {
        Deck {
            hero: "combat-dummy".to_string(),
            life,
            intellect: 0,
            weapon: None,
            equipment: vec![],
            cards: vec![],
        }
    }
}
