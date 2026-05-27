use rand::prelude::*;

use core::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Suit {
    Hearts,
    Diamonds,
    Clubs,
    Spades,
}

impl fmt::Display for Suit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let suit_str = match self {
            Suit::Hearts => "♥",
            Suit::Diamonds => "♦",
            Suit::Clubs => "♣",
            Suit::Spades => "♠",
        };
        write!(f, "{}", suit_str)
    }
}

/// Rank, stored internally as a number from 2 (Two) to 14 (Ace),
/// to make comparisons easier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Rank(u8);

impl Rank {
    pub fn new(value: u8) -> Self {
        if value < 1 || value > 14 {
            panic!("Rank must be between 1 and 14 (inclusive)");
        }
        if value == 1 {
            return Rank(14); // Ace is treated as the highest rank
        }
        Rank(value)
    }

    pub fn index(&self) -> usize {
        (self.0 - 2) as usize
    }

    pub fn from_index(index: usize) -> Self {
        if index >= 13 {
            panic!("Index must be between 0 and 12");
        }
        Rank((index + 2) as u8)
    }
}

impl fmt::Display for Rank {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = self.0.to_string();
        let rank_str = match self.0 {
            11 => "J",
            12 => "Q",
            13 => "K",
            14 => "A",
            _ => s.as_str(),
        };
        write!(f, "{}", rank_str)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Card {
    pub rank: Rank,
    pub suit: Suit,
}

impl fmt::Display for Card {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.rank, self.suit)
    }
}

/// Full standard 52-card deck, not shuffled.
pub fn full_deck() -> Vec<Card> {
    let mut deck = Vec::with_capacity(52);
    for suit in &[Suit::Hearts, Suit::Diamonds, Suit::Clubs, Suit::Spades] {
        for rank_value in 1..=13 {
            deck.push(Card {
                rank: Rank::new(rank_value),
                suit: *suit,
            });
        }
    }
    deck
}

pub fn full_deck_shuffled() -> Vec<Card> {
    let mut deck = full_deck();
    let mut rng = rand::rng();
    deck.shuffle(&mut rng);
    deck
}
