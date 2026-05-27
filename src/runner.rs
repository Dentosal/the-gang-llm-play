use crate::ai::OllamaPlayer;
use crate::state::State;

/// Run a full game with the given Ollama models (one per player).
/// `max_rounds` is the maximum number of full rounds (each player acts once per round).
/// Returns `true` if the players achieved a correct ranking before the limit, `false` otherwise.
pub fn play_game(models: Vec<String>, max_rounds: usize) -> bool {
    let players = models.len();
    let mut state = State::new(players);
    let mut ai: Vec<OllamaPlayer> = models
        .iter()
        .map(|m| OllamaPlayer::new(m, players, max_rounds))
        .collect();

    println!(
        "=== New game: {} players, models: {:?} ===",
        players, models
    );
    print_initial_state(&state);

    for round in 1..=max_rounds {
        println!("\n--- Round {} ---", round);
        for (player_idx, player_ai) in ai.iter_mut().enumerate().take(players) {
            let action = match player_ai.choose_action(&state, player_idx, round, max_rounds) {
                Ok(a) => a,
                Err(e) => {
                    eprintln!("\n=== Game aborted: {} ===", e);
                    return false;
                }
            };
            state.action_log.push(action);

            if let Some(victory) = state.is_victory() {
                println!(
                    "\n=== Game over after round {}, turn {} ===",
                    round, player_idx
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
    let tokens = state.tokens_held_by_players();
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
