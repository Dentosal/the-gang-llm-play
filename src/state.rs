use core::fmt;

use crate::card::Card;

/// Cards after all cards have been dealt.
pub struct RiverCards {
    pub community_cards: [Card; 5],
    pub hole_cards_per_player: Vec<[Card; 2]>,
}
impl RiverCards {
    pub fn random(players: usize) -> Self {
        let mut deck = crate::card::full_deck_shuffled();
        let community_cards: [Card; 5] = deck.drain(0..5).collect::<Vec<_>>().try_into().unwrap();
        let mut hole_cards_per_player = Vec::with_capacity(players);
        for _ in 0..players {
            let hole: [Card; 2] = deck.drain(0..2).collect::<Vec<_>>().try_into().unwrap();
            hole_cards_per_player.push(hole);
        }
        Self {
            community_cards,
            hole_cards_per_player,
        }
    }

    /// Returns a vector of hand ranks for each player, where a higher number is a better hand (0 = worst hand).
    /// Ties receive the same rank (competition ranking: rank = number of players with a strictly better hand).
    pub fn rank_hands(&self) -> Vec<usize> {
        let scores: Vec<crate::hand::Score> = self
            .hole_cards_per_player
            .iter()
            .map(|&hole| crate::hand::Score::best_score(hole, self.community_cards))
            .collect();
        scores
            .iter()
            .map(|s| scores.iter().filter(|o| *o < s).count())
            .collect()
    }
}

/// Token representing a claim that the player has Nth hand in the ordered list.
/// I.e. token 0 means that the player claims to have the worst hand.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Token(pub usize);

pub enum Action {
    /// No action, next player can act.
    Pass,
    /// Take a token, either from the middle or from another player.
    /// Any previously taken token is returned to the middle.
    Swap(Token),
    /// Return the token you're currently holding to the unclaimed pool in the middle.
    Return,
}

pub struct State {
    pub river_cards: RiverCards,
    pub action_log: Vec<Action>,
}

impl State {
    pub fn new(players: usize) -> Self {
        Self {
            river_cards: RiverCards::random(players),
            action_log: Vec::new(),
        }
    }

    pub fn players(&self) -> usize {
        self.river_cards.hole_cards_per_player.len()
    }

    pub fn tokens_held_by_players(&self) -> Vec<Option<Token>> {
        let mut tokens = vec![None; self.players()];
        for (i, action) in self.action_log.iter().enumerate() {
            let turn = i % self.players();
            match action {
                Action::Swap(takes) => {
                    for token in &mut tokens {
                        if *token == Some(*takes) {
                            *token = None;
                            break;
                        }
                    }
                    tokens[turn] = Some(*takes);
                }
                Action::Return => {
                    tokens[turn] = None;
                }
                Action::Pass => {}
            }
        }
        tokens
    }

    /// Returns None if the round is not complete,
    /// Some(true) the ranking is correct, Some(false) if the ranking is incorrect.
    pub fn is_victory(&self) -> Option<bool> {
        let tokens = self.tokens_held_by_players();
        let correct_ranking = self.river_cards.rank_hands();

        if tokens.iter().any(|t| t.is_none()) {
            return None;
        }
        let token_ranks: Vec<usize> = tokens.iter().map(|t| t.unwrap().0).collect();

        // Tied players at rank r are valid holders of any token in the contiguous block
        // [start, start+count-1], where start = number of players ranked below r.
        Some((0..self.players()).all(|i| {
            let r = correct_ranking[i];
            let start = correct_ranking.iter().filter(|&&o| o < r).count();
            let count = correct_ranking.iter().filter(|&&o| o == r).count();
            token_ranks[i] >= start && token_ranks[i] < start + count
        }))
    }

    pub fn view_for_player(&self, player_index: usize) -> StateView {
        StateView {
            community_cards: self.river_cards.community_cards,
            hand: self.river_cards.hole_cards_per_player[player_index],
        }
    }
}

/// View of [`State`] from the perspective of a single player.
pub struct StateView {
    community_cards: [Card; 5],
    hand: [Card; 2],
}

impl fmt::Display for StateView {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let hand_str = format!("{} {}", self.hand[0], self.hand[1]);
        let community_str = self
            .community_cards
            .iter()
            .map(|c| c.to_string())
            .collect::<Vec<_>>()
            .join(" ");
        write!(f, "Hand: {}\nCommunity: {}", hand_str, community_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{Rank, Suit::*};

    fn card(rank: u8, suit: crate::card::Suit) -> Card {
        Card {
            rank: Rank::new(rank),
            suit,
        }
    }

    fn river(community: [Card; 5], players: &[[Card; 2]]) -> RiverCards {
        RiverCards {
            community_cards: community,
            hole_cards_per_player: players.to_vec(),
        }
    }

    #[test]
    fn test_rank_hands_clear_winner() {
        // Community: 2♦ 3♣ 4♠ 7♥ J♦ — no board pairs, no flush
        // Player 0: A♠ K♠ → high card (A-K-J-7-4)
        // Player 1: 2♣ 2♥ → one pair of twos
        // Player 2: J♠ J♥ → one pair of jacks (best)
        let community = [
            card(2, Diamonds),
            card(3, Clubs),
            card(4, Spades),
            card(7, Hearts),
            card(11, Diamonds),
        ];
        let r = river(
            community,
            &[
                [card(1, Spades), card(13, Spades)],
                [card(2, Clubs), card(2, Hearts)],
                [card(11, Spades), card(11, Hearts)],
            ],
        );
        assert_eq!(r.rank_hands(), vec![0, 1, 2]);
    }

    #[test]
    fn test_rank_hands_all_play_the_board() {
        // Community alone forms the best possible 5-card hand for everyone (two pair A-K on board
        // with a Q kicker). Both players' hole cards are strictly worse, so everyone plays the board
        // and all ranks tie at 0.
        let community = [
            card(1, Spades),
            card(1, Clubs),
            card(13, Spades),
            card(13, Clubs),
            card(12, Spades),
        ];
        let r = river(
            community,
            &[
                [card(2, Hearts), card(3, Diamonds)],
                [card(4, Hearts), card(5, Diamonds)],
            ],
        );
        assert_eq!(r.rank_hands(), vec![0, 0]);
    }

    #[test]
    fn test_rank_hands_one_player_improves_with_hole_cards() {
        // Community: A♠ A♣ K♠ K♣ Q♠ → two pair (A-K) for anyone playing the board
        // Player 0: 2♥ 3♦ → plays the board, TwoPair(A,K,[Q])
        // Player 1: A♥ 2♣ → three aces + two kings = FullHouse(A,K), beats player 0
        let community = [
            card(1, Spades),
            card(1, Clubs),
            card(13, Spades),
            card(13, Clubs),
            card(12, Spades),
        ];
        let r = river(
            community,
            &[
                [card(2, Hearts), card(3, Diamonds)],
                [card(1, Hearts), card(2, Clubs)],
            ],
        );
        assert_eq!(r.rank_hands(), vec![0, 1]);
    }

    #[test]
    fn test_rank_hands_partial_tie() {
        // Player 2 has a full house; players 0 and 1 tie with two pair from the board.
        let community = [
            card(1, Spades),
            card(1, Clubs),
            card(13, Spades),
            card(13, Clubs),
            card(12, Spades),
        ];
        let r = river(
            community,
            &[
                [card(2, Hearts), card(3, Diamonds)], // plays board: TwoPair(A,K,[Q])
                [card(4, Hearts), card(5, Diamonds)], // plays board: TwoPair(A,K,[Q])
                [card(1, Hearts), card(2, Clubs)],    // FullHouse(A,K)
            ],
        );
        assert_eq!(r.rank_hands(), vec![0, 0, 2]);
    }

    // --- victory conditions ---

    #[test]
    fn test_victory_correct_ranking_simple() {
        let mut state = State::new(3);
        state.river_cards = river(
            [
                card(2, Diamonds),
                card(3, Clubs),
                card(4, Spades),
                card(7, Hearts),
                card(11, Diamonds),
            ],
            &[
                [card(1, Spades), card(13, Spades)],
                [card(2, Clubs), card(2, Hearts)],
                [card(11, Spades), card(11, Hearts)],
            ],
        );
        state.action_log = vec![
            Action::Swap(Token(0)), // Player 0 takes token 0 (worst hand)
            Action::Swap(Token(1)), // Player 1 takes token 1
            Action::Swap(Token(2)), // Player 2 takes token 2
        ];
        assert_eq!(state.is_victory(), Some(true));
    }

    #[test]
    fn test_victory_correct_ranking_3way_draw() {
        let mut state = State::new(3);
        state.river_cards = river(
            [
                card(2, Diamonds),
                card(3, Clubs),
                card(4, Spades),
                card(7, Hearts),
                card(11, Diamonds),
            ],
            &[
                [card(13, Spades), card(5, Spades)],
                [card(13, Clubs), card(5, Clubs)],
                [card(13, Diamonds), card(5, Diamonds)],
            ],
        );
        state.action_log = vec![
            Action::Swap(Token(0)), // Player 0 takes token 0 (worst hand)
            Action::Swap(Token(1)), // Player 1 takes token 1
            Action::Swap(Token(2)), // Player 2 takes token 2
        ];
        assert_eq!(state.is_victory(), Some(true));
    }

    #[test]
    fn test_victory_correct_ranking_2_tied() {
        let mut state = State::new(3);
        state.river_cards = river(
            [
                card(2, Diamonds),
                card(3, Clubs),
                card(4, Spades),
                card(7, Hearts),
                card(11, Diamonds),
            ],
            &[
                [card(13, Spades), card(5, Spades)],
                [card(13, Clubs), card(5, Clubs)],
                [card(11, Clubs), card(5, Diamonds)],
            ],
        );
        state.action_log = vec![
            Action::Swap(Token(0)), // Player 0 takes token 0 (worst hand)
            Action::Swap(Token(1)), // Player 1 takes token 1
            Action::Swap(Token(2)), // Player 2 takes token 2
        ];
        assert_eq!(state.is_victory(), Some(true));
    }

    #[test]
    fn test_victory_correct_ranking_2_tied_incorrect() {
        let mut state = State::new(3);
        state.river_cards = river(
            [
                card(2, Diamonds),
                card(3, Clubs),
                card(4, Spades),
                card(7, Hearts),
                card(11, Diamonds),
            ],
            &[
                [card(13, Spades), card(5, Spades)],
                [card(13, Clubs), card(5, Clubs)],
                [card(11, Clubs), card(5, Diamonds)],
            ],
        );
        state.action_log = vec![
            Action::Swap(Token(2)), // Player 0 takes token 2 (best hand) (wrong)
            Action::Swap(Token(1)), // Player 1 takes token 1
            Action::Swap(Token(0)), // Player 2 takes token 0 (worst hand) (wrong)
        ];
        assert_eq!(state.is_victory(), Some(false));
    }
}
