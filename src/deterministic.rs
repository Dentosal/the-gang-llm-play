use core::range::RangeInclusive;

use crate::{
    card::{Card, full_deck},
    runner::Player,
    state::{Action, RiverCards, StateView, Token, tokens_held_by_players},
};

/// Distributes full information deterministically, just fails to end the game until all info is out.
pub struct DeterministicPlayer {
    player_index: usize,
    player_count: usize,
    _max_rounds: usize,
}

impl DeterministicPlayer {
    pub fn new(player_index: usize, player_count: usize, _max_rounds: usize) -> Self {
        Self {
            player_index,
            player_count,
            _max_rounds,
        }
    }
}

impl Player for DeterministicPlayer {
    fn name(&self) -> &str {
        "deterministic"
    }

    fn choose_action(
        &mut self,
        state: &StateView<'_>,
        _round: usize,
        _max_rounds: usize,
    ) -> Result<Action, String> {
        let actions =
            non_game_ending_actions(state.action_log, self.player_index, self.player_count);

        // Space of still-possible cards; we signal to reduce this space until only one possibility remains.
        let player_limits =
            compute_player_limits(state.community_cards, state.action_log, self.player_count);

        // Check if all required info has been communicated already, and if so just pick a token.
        // Note that we must not pick a token that would end the game incorrectly here.
        let mut all_known = true;
        for limit in &player_limits {
            if limit.start != limit.last {
                // Still some uncertainty, need to communicate more.
                all_known = false;
                break;
            }
        }
        if all_known {
            let player_hands: Vec<[Card; 2]> = player_limits
                .iter()
                .map(|r| possible_holes(state.community_cards)[r.start])
                .collect();

            let ranking = (RiverCards {
                community_cards: state.community_cards,
                hole_cards_per_player: player_hands,
            })
            .rank_hands();
            let ranking = ranking_incr_duplicates(ranking);

            let our_ranking = ranking[self.player_index];

            // If we can grab our own token without ending the game, do it.
            let proposed = Action::Take(Token(our_ranking));
            if actions.contains(&proposed) {
                return Ok(proposed);
            }

            // If everyone else has the correct token already, we can end and win.
            let mut others_hold_correct_token = true;
            let ot = tokens_held_by_players(state.action_log, self.player_count);
            for i in 0..self.player_count {
                if i == self.player_index {
                    if ot[i] == Some(Token(our_ranking)) {
                        // If we already hold the correct token, pass
                        return Ok(Action::Pass);
                    } else if ot[i].is_some() {
                        // If we hold a wrong token, return it so someone else can grab theirs
                        return Ok(Action::Return);
                    }
                    continue;
                }
                if ot[i] != Some(Token(ranking[i])) {
                    others_hold_correct_token = false;
                }
            }
            if others_hold_correct_token {
                // We can win immediately so do that
                return Ok(proposed);
            }

            // Otherwise just pass, since we can't end the game correctly yet
            return Ok(Action::Pass);
        }

        // Otherwise communicate more
        let space = player_limits[self.player_index];
        Ok(actions[pick_action(state.community_cards, state.hand, actions.len(), space)?])
    }
}

/// Given a valid ranking, deterministically creates one without duplicates by incrementing.
fn ranking_incr_duplicates(ranking: Vec<usize>) -> Vec<usize> {
    let mut seen = std::collections::HashSet::new();
    let mut result = Vec::new();
    for r in ranking {
        let mut new_r = r;
        while seen.contains(&new_r) {
            new_r += 1;
        }
        seen.insert(new_r);
        result.push(new_r);
    }
    result
}

/// All possible hole card combinations, in deterministic order.
fn possible_holes(community_cards: [Card; 5]) -> Vec<[Card; 2]> {
    let mut pairs = Vec::new();
    let deck = full_deck();
    for i in 0..deck.len() {
        if community_cards.contains(&deck[i]) {
            continue;
        }
        for j in (i + 1)..deck.len() {
            if community_cards.contains(&deck[j]) {
                continue;
            }
            let mut pair = [deck[i], deck[j]];
            pair.sort();
            pairs.push(pair);
        }
    }
    pairs
}

/// The publicly known knowledge about which hole cards each player could still be holding.
/// Event-sourcing-style computed from actions.
fn compute_player_limits(
    community_cards: [Card; 5],
    action_log: &[Action],
    player_count: usize,
) -> Vec<RangeInclusive<usize>> {
    let mut limits: Vec<RangeInclusive<usize>> =
        vec![(0..=(possible_holes(community_cards).len() - 1)).into(); player_count];

    for (i, action) in action_log.iter().enumerate() {
        let actor = i % player_count;

        let actions = non_game_ending_actions(&action_log[..i], actor, player_count);

        let space = &mut limits[actor];
        if space.start == space.last {
            // The player's hand is already fully determined, so they won't communicate using this scheme anymore.
            continue;
        }

        let i = actions
            .iter()
            .position(|a| *a == *action)
            .expect("action not found in non_game_ending_actions");

        *space = limit_range(*space, i, actions.len());
    }

    limits
}

fn pick_action(
    community_cards: [Card; 5],
    mut my_hand: [Card; 2],
    available_action_count: usize,
    space: RangeInclusive<usize>,
) -> Result<usize, String> {
    assert!(
        available_action_count >= 2,
        "impossible to have less than 2 actions"
    );

    if space.start == space.last {
        // No info to communicate anymore, just pass.
        return Ok(0);
    }

    // Partition the remaining space by the number of non-game-ending actions,
    // and pick the one corresponding to our index
    my_hand.sort();
    let our_index = possible_holes(community_cards)
        .iter()
        .position(|h| *h == my_hand)
        .expect("possible_holes did not include our hand");
    let space_size_minus_one = space.last - space.start;
    let relative_index = our_index - space.start;

    let partition_size = space_size_minus_one / available_action_count;

    if partition_size == 0 {
        // Goes to zero if the divider is too large -> we can just send the absolute index directly.
        return Ok(relative_index);
    }

    let partition_index = relative_index / partition_size;
    Ok(partition_index.min(available_action_count - 1))
}

/// Inverse function of pick_action
fn limit_range(
    space: RangeInclusive<usize>,
    picked_action_index: usize,
    available_action_count: usize,
) -> RangeInclusive<usize> {
    let space_size_minus_one = space.last - space.start;

    let partition_size = space_size_minus_one / available_action_count;
    if partition_size == 0 {
        let absolute_index = space.start + picked_action_index;
        return (absolute_index..=absolute_index).into();
    }
    let partition_index = picked_action_index;

    // Start of the partition that corresponds to the picked action
    let parition_start = space.start + partition_index * partition_size;
    let partition_end = if partition_index == available_action_count - 1 {
        // Last partition takes the remainder of the space
        space.last
    } else {
        parition_start + partition_size - 1
    };

    (parition_start..=partition_end).into()
}

/// Actions that don't end the game, in deterministic order.
/// Does not consider the round limit.
fn non_game_ending_actions(
    action_log: &[Action],
    player_index: usize,
    player_count: usize,
) -> Vec<Action> {
    // We can always pass.
    let mut result = vec![Action::Pass];

    let tokens = tokens_held_by_players(action_log, player_count);

    // If we have a token, we can return it.
    if tokens[player_index].is_some() {
        result.push(Action::Return);
    }

    // If there is more than one unclaimed token, we can take any of the remaning ones.
    let unclaimed_tokens: Vec<_> = (0..player_count)
        .map(Token)
        .filter(|t| !tokens.contains(&Some(*t)))
        .collect();
    if unclaimed_tokens.len() > 1 {
        for tok in unclaimed_tokens {
            result.push(Action::Take(tok));
        }
    }

    // And lastly, tokens from other players are always takeable
    for (i, tok) in tokens.iter().enumerate() {
        if i != player_index
            && let Some(tok) = tok
        {
            result.push(Action::Take(*tok));
        }
    }

    result
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::card::{Rank, Suit};

    #[test]
    fn test_ranking_incr_duplicates() {
        assert_eq!(ranking_incr_duplicates(vec![0, 0, 0, 0]), vec![0, 1, 2, 3]);
        assert_eq!(ranking_incr_duplicates(vec![0, 1, 1, 3]), vec![0, 1, 2, 3]);
        assert_eq!(ranking_incr_duplicates(vec![3, 1, 1, 0]), vec![3, 1, 2, 0]);
    }

    #[test]
    fn pick_action_and_limit_range_bijection() {
        let community_cards = [
            Card {
                rank: Rank::new(1),
                suit: Suit::Diamonds,
            },
            Card {
                rank: Rank::new(2),
                suit: Suit::Diamonds,
            },
            Card {
                rank: Rank::new(3),
                suit: Suit::Diamonds,
            },
            Card {
                rank: Rank::new(4),
                suit: Suit::Diamonds,
            },
            Card {
                rank: Rank::new(5),
                suit: Suit::Diamonds,
            },
        ];

        let possible_holes = possible_holes(community_cards);
        let limit = possible_holes.len();
        let full_range: RangeInclusive<usize> = (0..=limit).into();

        for meta_aac in 2..10 {
            for (i, mut holes) in possible_holes.iter().copied().enumerate() {
                let aac = ((meta_aac + i) % 5) + 2; // pseudo rng

                holes.sort();

                let holes_index = possible_holes
                    .iter()
                    .position(|h| *h == holes)
                    .expect("possible_holes did not include our holes");

                let mut space = full_range;
                while space.start != space.last {
                    let action = pick_action(community_cards, holes, aac, space).unwrap();
                    space = limit_range(space, action, aac);
                }
                assert_eq!(space.start, holes_index);
            }
        }
    }

    #[test]
    fn always_wins() {
        let player_count = 4;
        let game_rounds = 100;
        for _ in 0..10 {
            let players: Vec<Box<dyn Player>> = (0..player_count)
                .map(|player_index| -> Box<dyn Player> {
                    Box::new(DeterministicPlayer::new(
                        player_index,
                        player_count,
                        game_rounds,
                    ))
                })
                .collect();
            let victory = crate::runner::play_game(players, game_rounds);
            assert!(victory, "DeterministicPlayer should always win");
        }
    }
}
