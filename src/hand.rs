use core::fmt;

use crate::card::{Card, Rank};

/// Poker hand types, ordered from worst to best
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Score {
    HighCard([Rank; 5]),
    OnePair(Rank, [Rank; 3]),
    TwoPair(Rank, Rank, [Rank; 1]),
    ThreeOfAKind(Rank, [Rank; 2]),
    Straight(Rank),
    Flush([Rank; 5]),
    FullHouse(Rank, Rank),
    FourOfAKind(Rank, Rank),
    StraightFlush(Rank),
}
impl Score {
    pub fn from_hand(mut cards: [Card; 5]) -> Self {
        // Sort cards by rank in descending order, so that the highest card is first
        cards.sort_by(|a, b| b.rank.cmp(&a.rank));

        let is_flush = cards.iter().all(|card| card.suit == cards[0].suit);
        let is_straight = cards
            .array_windows()
            .all(|[a, b]| a.rank.index() == b.rank.index() + 1);
        // A-2-3-4-5: after descending sort the ace lands at cards[0] (index 12) and
        // the remaining four cards form the 5-4-3-2 run (indices 3-2-1-0).
        let is_ace_low_straight = cards[0].rank.index() == 12
            && cards[1].rank.index() == 3
            && cards[1..]
                .array_windows::<2>()
                .all(|[a, b]| a.rank.index() == b.rank.index() + 1);

        let mut rank_counts = [0; 13];
        for card in &cards {
            rank_counts[card.rank.index()] += 1;
        }

        // Straight flush
        if (is_straight || is_ace_low_straight) && is_flush {
            let top = if is_ace_low_straight {
                cards[1].rank
            } else {
                cards[0].rank
            };
            return Self::StraightFlush(top);
        }

        // Four of a kind
        if let Some((rank, _)) = rank_counts
            .iter()
            .enumerate()
            .find(|&(_, &count)| count == 4)
        {
            let kicker = cards
                .iter()
                .find(|card| card.rank.index() != rank)
                .unwrap()
                .rank;
            return Self::FourOfAKind(Rank::from_index(rank), kicker);
        }

        // Flush
        if is_flush {
            return Self::Flush(cards.map(|card| card.rank));
        }

        // Full house
        if let Some((three_rank, _)) = rank_counts
            .iter()
            .enumerate()
            .find(|&(_, &count)| count == 3)
        {
            if let Some((two_rank, _)) = rank_counts
                .iter()
                .enumerate()
                .find(|&(_, &count)| count == 2)
            {
                return Self::FullHouse(Rank::from_index(three_rank), Rank::from_index(two_rank));
            }
        }

        // Straight
        if is_straight || is_ace_low_straight {
            let top = if is_ace_low_straight {
                cards[1].rank
            } else {
                cards[0].rank
            };
            return Self::Straight(top);
        }

        // Three of a kind
        if let Some((three_rank, _)) = rank_counts
            .iter()
            .enumerate()
            .find(|&(_, &count)| count == 3)
        {
            let kickers: [Rank; 2] = cards
                .iter()
                .filter(|card| card.rank.index() != three_rank)
                .map(|card| card.rank)
                .collect::<Vec<_>>()
                .try_into()
                .unwrap();
            return Self::ThreeOfAKind(Rank::from_index(three_rank), kickers);
        }

        // Two pair
        let pairs: Vec<usize> = rank_counts
            .iter()
            .enumerate()
            .filter(|&(_, &count)| count == 2)
            .map(|(index, _)| index)
            .collect();
        if pairs.len() == 2 {
            let kicker = cards
                .iter()
                .find(|card| card.rank.index() != pairs[0] && card.rank.index() != pairs[1])
                .unwrap()
                .rank;
            // pairs is collected in ascending index order; store higher pair first so
            // derived Ord compares the more significant pair first.
            return Self::TwoPair(
                Rank::from_index(pairs[1]),
                Rank::from_index(pairs[0]),
                [kicker],
            );
        }

        // One pair
        if let Some((pair_rank, _)) = rank_counts
            .iter()
            .enumerate()
            .find(|&(_, &count)| count == 2)
        {
            let kickers: [Rank; 3] = cards
                .iter()
                .filter(|card| card.rank.index() != pair_rank)
                .map(|card| card.rank)
                .collect::<Vec<_>>()
                .try_into()
                .unwrap();
            return Self::OnePair(Rank::from_index(pair_rank), kickers);
        }

        Self::HighCard(cards.map(|card| card.rank))
    }

    /// Pick the best 5-card combination out of the 7 cards available (2 hole + 5 community)
    /// and return the corresponding hand score.
    pub fn best_score(hole_cards: [Card; 2], community_cards: [Card; 5]) -> Self {
        let all: [Card; 7] = [
            hole_cards[0],
            hole_cards[1],
            community_cards[0],
            community_cards[1],
            community_cards[2],
            community_cards[3],
            community_cards[4],
        ];
        // Iterate over all C(7,2)=21 pairs of cards to drop; the remaining 5 form a hand.
        let mut best = None::<Self>;
        for i in 0..7 {
            for j in (i + 1)..7 {
                let hand: [Card; 5] = (0..7)
                    .filter(|&k| k != i && k != j)
                    .map(|k| all[k])
                    .collect::<Vec<_>>()
                    .try_into()
                    .unwrap();
                let score = Self::from_hand(hand);
                best = Some(best.map_or(score, |b: Self| b.max(score)));
            }
        }
        best.unwrap()
    }
}

impl fmt::Display for Score {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Score::HighCard(r) => write!(f, "High Card, {}-high", r[0]),
            Score::OnePair(pair, k) => write!(
                f,
                "One Pair of {} (kickers: {} {} {})",
                plural(*pair),
                k[0],
                k[1],
                k[2]
            ),
            Score::TwoPair(hi, lo, k) => write!(
                f,
                "Two Pair, {} and {} (kicker: {})",
                plural(*hi),
                plural(*lo),
                k[0]
            ),
            Score::ThreeOfAKind(rank, k) => write!(
                f,
                "Three of a Kind, {} (kickers: {} {})",
                plural(*rank),
                k[0],
                k[1]
            ),
            Score::Straight(top) => write!(f, "Straight, {}-high", top),
            Score::Flush(r) => write!(f, "Flush, {}-high", r[0]),
            Score::FullHouse(three, two) => {
                write!(f, "Full House, {} full of {}", plural(*three), plural(*two))
            }
            Score::FourOfAKind(rank, k) => {
                write!(f, "Four of a Kind, {} (kicker: {})", plural(*rank), k)
            }
            Score::StraightFlush(top) if *top == Rank::new(14) => write!(f, "Royal Flush"),
            Score::StraightFlush(top) => write!(f, "Straight Flush, {}-high", top),
        }
    }
}

fn plural(r: Rank) -> &'static str {
    match r.index() {
        0 => "Twos",
        1 => "Threes",
        2 => "Fours",
        3 => "Fives",
        4 => "Sixes",
        5 => "Sevens",
        6 => "Eights",
        7 => "Nines",
        8 => "Tens",
        9 => "Jacks",
        10 => "Queens",
        11 => "Kings",
        12 => "Aces",
        _ => unreachable!(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::Suit::{Clubs, Diamonds, Hearts, Spades};

    fn card(rank: u8, suit: crate::card::Suit) -> Card {
        Card {
            rank: Rank::new(rank),
            suit,
        }
    }

    // --- classification ---

    #[test]
    fn test_high_card() {
        let hand = [
            card(2, Hearts),
            card(5, Diamonds),
            card(7, Clubs),
            card(9, Spades),
            card(11, Hearts),
        ];
        assert_eq!(
            Score::from_hand(hand),
            Score::HighCard([
                Rank::new(11),
                Rank::new(9),
                Rank::new(7),
                Rank::new(5),
                Rank::new(2)
            ])
        );
    }

    #[test]
    fn test_one_pair() {
        let hand = [
            card(7, Hearts),
            card(7, Spades),
            card(2, Clubs),
            card(5, Diamonds),
            card(9, Hearts),
        ];
        assert_eq!(
            Score::from_hand(hand),
            Score::OnePair(Rank::new(7), [Rank::new(9), Rank::new(5), Rank::new(2)])
        );
    }

    #[test]
    fn test_two_pair() {
        let hand = [
            card(1, Hearts),
            card(1, Spades),
            card(2, Clubs),
            card(2, Diamonds),
            card(9, Hearts),
        ];
        assert_eq!(
            Score::from_hand(hand),
            Score::TwoPair(Rank::new(14), Rank::new(2), [Rank::new(9)])
        );
    }

    #[test]
    fn test_three_of_a_kind() {
        let hand = [
            card(8, Hearts),
            card(8, Spades),
            card(8, Clubs),
            card(3, Diamonds),
            card(6, Hearts),
        ];
        assert_eq!(
            Score::from_hand(hand),
            Score::ThreeOfAKind(Rank::new(8), [Rank::new(6), Rank::new(3)])
        );
    }

    #[test]
    fn test_straight() {
        let hand = [
            card(5, Hearts),
            card(6, Spades),
            card(7, Clubs),
            card(8, Diamonds),
            card(9, Hearts),
        ];
        assert_eq!(Score::from_hand(hand), Score::Straight(Rank::new(9)));
    }

    #[test]
    fn test_flush() {
        let hand = [
            card(2, Hearts),
            card(5, Hearts),
            card(7, Hearts),
            card(9, Hearts),
            card(11, Hearts),
        ];
        assert_eq!(
            Score::from_hand(hand),
            Score::Flush([
                Rank::new(11),
                Rank::new(9),
                Rank::new(7),
                Rank::new(5),
                Rank::new(2)
            ])
        );
    }

    #[test]
    fn test_full_house() {
        let hand = [
            card(10, Hearts),
            card(10, Spades),
            card(10, Clubs),
            card(6, Diamonds),
            card(6, Hearts),
        ];
        assert_eq!(
            Score::from_hand(hand),
            Score::FullHouse(Rank::new(10), Rank::new(6))
        );
    }

    #[test]
    fn test_four_of_a_kind() {
        let hand = [
            card(9, Hearts),
            card(9, Spades),
            card(9, Clubs),
            card(9, Diamonds),
            card(3, Hearts),
        ];
        assert_eq!(
            Score::from_hand(hand),
            Score::FourOfAKind(Rank::new(9), Rank::new(3))
        );
    }

    #[test]
    fn test_straight_flush() {
        let hand = [
            card(10, Hearts),
            card(11, Hearts),
            card(12, Hearts),
            card(13, Hearts),
            card(1, Hearts),
        ];
        assert_eq!(Score::from_hand(hand), Score::StraightFlush(Rank::new(14)));
    }

    #[test]
    fn test_ace_low_straight() {
        let hand = [
            card(1, Hearts),
            card(2, Spades),
            card(3, Diamonds),
            card(4, Clubs),
            card(5, Hearts),
        ];
        assert_eq!(Score::from_hand(hand), Score::Straight(Rank::new(5)));
    }

    #[test]
    fn test_ace_low_straight_flush() {
        let hand = [
            card(1, Spades),
            card(2, Spades),
            card(3, Spades),
            card(4, Spades),
            card(5, Spades),
        ];
        assert_eq!(Score::from_hand(hand), Score::StraightFlush(Rank::new(5)));
    }

    // --- hand-type ordering (the enum variant order) ---

    #[test]
    fn test_hand_type_ranking() {
        let high_card = Score::HighCard([Rank::new(14); 5]);
        let one_pair = Score::OnePair(Rank::new(14), [Rank::new(13); 3]);
        let two_pair = Score::TwoPair(Rank::new(14), Rank::new(13), [Rank::new(12)]);
        let three_oak = Score::ThreeOfAKind(Rank::new(14), [Rank::new(13); 2]);
        let straight = Score::Straight(Rank::new(14));
        let flush = Score::Flush([Rank::new(14); 5]);
        let full_house = Score::FullHouse(Rank::new(14), Rank::new(13));
        let four_oak = Score::FourOfAKind(Rank::new(14), Rank::new(13));
        let str_flush = Score::StraightFlush(Rank::new(14));

        assert!(high_card < one_pair);
        assert!(one_pair < two_pair);
        assert!(two_pair < three_oak);
        assert!(three_oak < straight);
        assert!(straight < flush);
        assert!(flush < full_house);
        assert!(full_house < four_oak);
        assert!(four_oak < str_flush);
    }

    // --- tiebreaker ordering ---

    #[test]
    fn test_high_card_tiebreak() {
        let a = Score::from_hand([
            card(14, Hearts),
            card(10, Spades),
            card(7, Clubs),
            card(4, Diamonds),
            card(2, Hearts),
        ]);
        let b = Score::from_hand([
            card(14, Spades),
            card(10, Clubs),
            card(7, Hearts),
            card(4, Spades),
            card(3, Diamonds),
        ]);
        assert!(b > a);
    }

    #[test]
    fn test_one_pair_kicker_tiebreak() {
        let a = Score::from_hand([
            card(7, Hearts),
            card(7, Spades),
            card(2, Clubs),
            card(4, Diamonds),
            card(9, Hearts),
        ]);
        let b = Score::from_hand([
            card(7, Clubs),
            card(7, Diamonds),
            card(2, Hearts),
            card(4, Spades),
            card(10, Clubs),
        ]);
        assert!(b > a);
    }

    #[test]
    fn test_two_pair_high_pair_wins() {
        // AA22 vs KKQQ: Aces-up beats Kings-up
        let aces_up = Score::from_hand([
            card(1, Hearts),
            card(1, Spades),
            card(2, Clubs),
            card(2, Diamonds),
            card(9, Hearts),
        ]);
        let kings_up = Score::from_hand([
            card(13, Hearts),
            card(13, Spades),
            card(12, Clubs),
            card(12, Diamonds),
            card(9, Clubs),
        ]);
        assert!(aces_up > kings_up);
    }

    #[test]
    fn test_two_pair_low_pair_tiebreak() {
        // AAKK vs AAQQ
        let aakk = Score::from_hand([
            card(1, Hearts),
            card(1, Spades),
            card(13, Clubs),
            card(13, Diamonds),
            card(2, Hearts),
        ]);
        let aaqq = Score::from_hand([
            card(1, Clubs),
            card(1, Diamonds),
            card(12, Hearts),
            card(12, Spades),
            card(2, Clubs),
        ]);
        assert!(aakk > aaqq);
    }

    #[test]
    fn test_two_pair_kicker_tiebreak() {
        // AAKK Q-kicker vs AAKK J-kicker
        let with_q = Score::from_hand([
            card(1, Hearts),
            card(1, Spades),
            card(13, Clubs),
            card(13, Diamonds),
            card(12, Hearts),
        ]);
        let with_j = Score::from_hand([
            card(1, Clubs),
            card(1, Diamonds),
            card(13, Hearts),
            card(13, Spades),
            card(11, Clubs),
        ]);
        assert!(with_q > with_j);
    }

    #[test]
    fn test_wheel_ranks_below_six_high_straight() {
        let wheel = Score::Straight(Rank::new(5));
        let six_high = Score::Straight(Rank::new(6));
        assert!(wheel < six_high);
    }

    // --- best_score ---

    #[test]
    fn test_best_score_straight_flush() {
        // Hole: A♠ K♠  Community: Q♠ J♠ 10♠ 2♥ 7♦ → royal flush
        let hole = [card(1, Spades), card(13, Spades)];
        let community = [
            card(12, Spades),
            card(11, Spades),
            card(10, Spades),
            card(2, Hearts),
            card(7, Diamonds),
        ];
        assert_eq!(
            Score::best_score(hole, community),
            Score::StraightFlush(Rank::new(14))
        );
    }

    #[test]
    fn test_best_score_flush_beats_straight() {
        // Hole: A♥ 2♥  Community: 5♥ 7♥ 9♥ 3♦ 4♣
        // Options include wheel straight (A-2-3-4-5) and flush (A-9-7-5-2 of hearts).
        // Flush must win.
        let hole = [card(1, Hearts), card(2, Hearts)];
        let community = [
            card(5, Hearts),
            card(7, Hearts),
            card(9, Hearts),
            card(3, Diamonds),
            card(4, Clubs),
        ];
        assert_eq!(
            Score::best_score(hole, community),
            Score::Flush([
                Rank::new(14),
                Rank::new(9),
                Rank::new(7),
                Rank::new(5),
                Rank::new(2)
            ]),
        );
    }

    #[test]
    fn test_best_score_uses_best_kickers() {
        // Hole: K♥ 2♦  Community: A♣ A♦ Q♠ J♦ 3♣
        // One pair of Aces; best kickers are K, Q, J (not 3 or 2).
        let hole = [card(13, Hearts), card(2, Diamonds)];
        let community = [
            card(1, Clubs),
            card(1, Diamonds),
            card(12, Spades),
            card(11, Diamonds),
            card(3, Clubs),
        ];
        assert_eq!(
            Score::best_score(hole, community),
            Score::OnePair(Rank::new(14), [Rank::new(13), Rank::new(12), Rank::new(11)]),
        );
    }

    #[test]
    fn test_best_score_upgrade_to_full_house() {
        // Hole: Q♥ Q♦  Community: Q♠ K♣ K♦ 2♥ 3♠
        // Three queens + two kings = full house QQQ-KK (beats trip queens with worse kickers).
        let hole = [card(12, Hearts), card(12, Diamonds)];
        let community = [
            card(12, Spades),
            card(13, Clubs),
            card(13, Diamonds),
            card(2, Hearts),
            card(3, Spades),
        ];
        assert_eq!(
            Score::best_score(hole, community),
            Score::FullHouse(Rank::new(12), Rank::new(13))
        );
    }

    #[test]
    fn test_best_score_community_only() {
        // Hole: 2♣ 3♦ (both low, useless)  Community: A♥ K♥ Q♥ J♥ 10♥ (royal flush on board)
        // Best 5 are all community cards.
        let hole = [card(2, Clubs), card(3, Diamonds)];
        let community = [
            card(1, Hearts),
            card(13, Hearts),
            card(12, Hearts),
            card(11, Hearts),
            card(10, Hearts),
        ];
        assert_eq!(
            Score::best_score(hole, community),
            Score::StraightFlush(Rank::new(14))
        );
    }

    #[test]
    fn test_best_score_four_of_a_kind_best_kicker() {
        // Hole: A♥ K♦  Community: A♠ A♣ A♦ 2♥ 3♣
        // Four aces; best kicker is K (not 3 or 2).
        let hole = [card(1, Hearts), card(13, Diamonds)];
        let community = [
            card(1, Spades),
            card(1, Clubs),
            card(1, Diamonds),
            card(2, Hearts),
            card(3, Clubs),
        ];
        assert_eq!(
            Score::best_score(hole, community),
            Score::FourOfAKind(Rank::new(14), Rank::new(13))
        );
    }
}
