mod ai;
mod card;
mod deterministic;
mod hand;
mod runner;
mod state;

use ai::OllamaPlayer;
use clap::Parser;
use runner::Player;

#[derive(Parser)]
struct Args {
    /// Ollama model name (used for all players)
    #[arg(long, default_value = "qwen3.6:latest")]
    model: String,

    /// Number of players (minimum 3)
    #[arg(long, default_value_t = 4, value_parser = parse_players)]
    players: usize,

    /// Maximum number of rounds (minimum 1)
    #[arg(long, default_value_t = 10, value_parser = parse_rounds)]
    rounds: usize,
}

fn parse_players(s: &str) -> Result<usize, String> {
    let n: usize = s
        .parse()
        .map_err(|_| format!("'{s}' is not a valid number"))?;
    if n >= 3 {
        Ok(n)
    } else {
        Err("player count must be at least 3".into())
    }
}

fn parse_rounds(s: &str) -> Result<usize, String> {
    let n: usize = s
        .parse()
        .map_err(|_| format!("'{s}' is not a valid number"))?;
    if n >= 1 {
        Ok(n)
    } else {
        Err("round count must be at least 1".into())
    }
}

fn main() {
    let args = Args::parse();
    let players: Vec<Box<dyn Player>> = (0..args.players)
        .map(|player_index| -> Box<dyn Player> {
            if args.model == "deterministic" {
                Box::new(deterministic::DeterministicPlayer::new(
                    player_index,
                    args.players,
                    args.rounds,
                ))
            } else {
                Box::new(OllamaPlayer::new(
                    &args.model,
                    player_index,
                    args.players,
                    args.rounds,
                ))
            }
        })
        .collect();
    runner::play_game(players, args.rounds);
}
