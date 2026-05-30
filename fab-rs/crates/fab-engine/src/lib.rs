//! `fab-engine`: a simplified, simulatable Flesh and Blood rules engine in Rust.
//!
//! See [`game`] for the rules subset that is modelled. Zero external dependencies.

pub mod agents;
pub mod deck;
pub mod game;
pub mod linear;
pub mod rng;

pub use deck::Deck;
pub use game::{Action, Agent, Game, Outcome, PlayerState};
pub use linear::{LinearAgent, Weights};
pub use rng::Rng;

pub use fab_cards::{Card, CardDb};
