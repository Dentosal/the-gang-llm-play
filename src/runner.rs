use crate::state::{Action, State, StateView, tokens_held_by_players};

pub trait Player {
    fn name(&self) -> &str;
    fn choose_action(
        &mut self,
        state: &StateView,
        round: usize,
        max_rounds: usize,
    ) -> Result<Action, String>;
}

/// Run a full game with the given players (one per seat).
/// `max_rounds` is the maximum number of full rounds (each player acts once per round).
/// Returns `true` if the players achieved a correct ranking before the limit, `false` otherwise.
pub fn play_game(mut players: Vec<Box<dyn Player>>, max_rounds: usize) -> bool {
    let n = players.len();
    let mut state = State::new(n);
    let names: Vec<&str> = players.iter().map(|p| p.name()).collect();

    println!("=== New game: {} players, models: {:?} ===", n, names);
    print_initial_state(&state);

    for round in 1..=max_rounds {
        println!("\n--- Round {} ---", round);
        for (player_index, player) in players.iter_mut().enumerate() {
            let state_view = state.view_for_player(player_index);
            let action = match player.choose_action(&state_view, round, max_rounds) {
                Ok(a) => a,
                Err(e) => {
                    eprintln!("\n=== Game aborted: {} ===", e);
                    return false;
                }
            };
            println!("[Player {} / {}] {:?}", player_index, player.name(), action);

            state.action_log.push(action);

            if let Some(victory) = state.is_victory() {
                println!(
                    "\n=== Game over after round {}, turn {} ===",
                    round, player_index
                );
                print_result(&state, victory);
                return victory;
            }
        }
    }

    println!("\n=== Round limit ({}) reached ===", max_rounds);
    let final_result = state.is_victory();
    let victory = final_result == Some(true);
    print_result(&state, victory);
    victory
}

fn print_initial_state(state: &State) {
    let community: String = state
        .river_cards
        .community_cards
        .iter()
        .map(|c| c.to_string())
        .collect::<Vec<_>>()
        .join(" ");
    let ranking = state.river_cards.rank_hands();
    println!("Community: {}", community);
    for (i, hole) in state.river_cards.hole_cards_per_player.iter().enumerate() {
        let score = crate::hand::Score::best_score(*hole, state.river_cards.community_cards);
        println!(
            "Player {}: {} {}  →  {}  (rank {})",
            i, hole[0], hole[1], score, ranking[i]
        );
    }
}

fn print_result(state: &State, victory: bool) {
    let tokens = tokens_held_by_players(&state.action_log, state.players());
    let ranking = state.river_cards.rank_hands();
    println!("Correct ranking : {:?}", ranking);
    println!(
        "Tokens held     : {:?}",
        tokens
            .iter()
            .map(|t| t.map(|tok| tok.0))
            .collect::<Vec<_>>()
    );
    println!("{}", if victory { "VICTORY" } else { "INCORRECT" });
}
