use serde::{Deserialize, Serialize};

use crate::hand::Score;
use crate::runner::Player;
use crate::state::{Action, StateView, Token, tokens_held_by_players};

#[derive(Serialize, Deserialize, Clone)]
struct Message {
    role: String,
    content: String,
}

/// A player backed by a local Ollama model.
/// Maintains a per-player chat history so the model can reason across turns.
pub struct OllamaPlayer {
    model: String,
    history: Vec<Message>,
    player_index: usize,
    player_count: usize,
}

impl OllamaPlayer {
    pub fn new(
        model: impl Into<String>,
        player_index: usize,
        player_count: usize,
        max_rounds: usize,
    ) -> Self {
        let model = model.into();
        let system = format!(
            "You are playing a cooperative poker hand-ranking game with {} players.\n\
             You can see your own hole cards and the 5 community cards. You cannot see other players' hole cards.\n\
             There are {} tokens numbered 0 to {}.\n\
             Token N means: exactly N other players have a worse hand than you.\n\
             So token 0 = worst hand, token {} = best hand.\n\
             The hands are compared using standard texas hold'em rules.\n\
             All cards are originally dealt from a standard 52-card deck.\n\
             On your turn you must choose one of:\n\
               - PASS (do nothing)\n\
               - TAKE a token: take any token, either from another player or from the unclaimed pool.\n\
                 Your previously held token, if any, goes back to the unclaimed pool.\n\
                 If that token was held by another player, they no longer have a token.\n\
               - RETURN: give back the token you are currently holding to the unclaimed pool.\n\
                If you currently hold no token, RETURN is invalid.\n\
             You have multiple rounds that you can use to coordinate with other players, \
             and you can signal about your hand strength using your actions.\n\
             On each round, players take turns choosing an action starting from Player 0.\n\
             You cannot communicate except through your token actions.\n\
             The game ends immediately once every player holds a token, or after {} rounds.\n\
             Once the last token is taken, the game ends instantly allowing no further corrections.\n\
             It's your goal to ensure that every player holds the correct token by the end of the game.\n\
             Victory: every player holds the token whose number equals how many players have a strictly worse hand than them.\n\
             Tied hands are treated as interchangeable, and any ordering of them is accepted.\n\
             Loss: at termination, any player holds an incorrect token or no token.\n\
             This is a cooperative game: you all win or lose as a team.\n\
             \n\
             Before answering, reason step by step inside a <think>...</think> block.\n\
             After the block, respond with exactly one of:\n\
               PASS\n\
               TAKE <n>   (where n is the token number)\n\
               RETURN\n\
             Example: <think>I have two pair, probably mid-strength.</think> TAKE 1",
            player_count,
            player_count,
            player_count - 1,
            player_count - 1,
            max_rounds,
        );
        Self {
            model,
            history: vec![Message {
                role: "system".into(),
                content: system,
            }],
            player_index,
            player_count,
        }
    }
}

impl Player for OllamaPlayer {
    fn name(&self) -> &str {
        &self.model
    }

    fn choose_action(
        &mut self,
        state: &StateView<'_>,
        round: usize,
        max_rounds: usize,
    ) -> Result<Action, String> {
        let score = Score::best_score(state.hand, state.community_cards);
        let tokens = tokens_held_by_players(state.action_log, self.player_count);
        let my_token = tokens[self.player_index];
        let my_token_str = match my_token {
            Some(tok) => format!("token {}", tok.0),
            None => "no token".to_string(),
        };
        let all_token_status: String = (0..self.player_count)
            .map(|i| match tokens[i] {
                Some(tok) => format!("  Token {}: held by Player {}", tok.0, i),
                None => format!("  Token {}: unclaimed", i),
            })
            .collect::<Vec<_>>()
            .join("\n");

        let action_history: String = if state.action_log.is_empty() {
            "  (none yet)".into()
        } else {
            state
                .action_log
                .iter()
                .enumerate()
                .map(|(i, action)| {
                    let actor = i % self.player_count;
                    match action {
                        crate::state::Action::Pass => format!("  Player {}: pass", actor),
                        crate::state::Action::Take(tok) => {
                            format!("  Player {}: take token {}", actor, tok.0)
                        }
                        crate::state::Action::Return => format!("  Player {}: return token", actor),
                    }
                })
                .collect::<Vec<_>>()
                .join("\n")
        };

        let user_msg = format!(
            "=== Round {}/{} — your turn (you are Player {}) ===\n\
             You currently hold: {}\n\
             {}\n\
             Your best hand when combining your hole cards with the community cards: {}\n\n\
             Action history:\n{}\n\n\
             Token state:\n{}\n\nWhat do you do?",
            round,
            max_rounds,
            self.player_index,
            my_token_str,
            state,
            score,
            action_history,
            all_token_status,
        );

        self.history.push(Message {
            role: "user".into(),
            content: user_msg,
        });

        let reply = self.call_ollama().map_err(|e| {
            format!(
                "[Player {} / {}] Ollama error: {}",
                self.player_index, self.model, e
            )
        })?;

        self.history.push(Message {
            role: "assistant".into(),
            content: reply.clone(),
        });

        parse_action(&reply).map_err(|e| {
            format!(
                "[Player {} / {}] Invalid action {:?}: {}",
                self.player_index,
                self.model,
                reply.trim(),
                e
            )
        })
    }
}

impl OllamaPlayer {
    fn call_ollama(&self) -> Result<String, Box<dyn std::error::Error>> {
        #[derive(Serialize)]
        struct Request<'a> {
            model: &'a str,
            messages: &'a [Message],
            stream: bool,
        }
        #[derive(Deserialize)]
        struct Response {
            message: Message,
        }

        let body = Request {
            model: &self.model,
            messages: &self.history,
            stream: false,
        };

        let response: Response = ureq::post("http://localhost:11434/api/chat")
            .send_json(&body)?
            .body_mut()
            .read_json()?;

        Ok(response.message.content)
    }
}

fn parse_action(text: &str) -> Result<Action, &'static str> {
    // Strip an optional <think>...</think> blocks, then parse the remainder.
    let text = regex::Regex::new("(?s)<think>.*?</think>")
        .unwrap()
        .replace_all(text, "")
        .to_string()
        .trim()
        .to_uppercase();

    if text.contains("PASS") {
        return Ok(Action::Pass);
    }
    if text.contains("RETURN") {
        return Ok(Action::Return);
    }
    if let Some(pos) = text.find("TAKE") {
        let after = &text[pos + 4..];
        let digits: String = after
            .chars()
            .skip_while(|c| !c.is_ascii_digit())
            .take_while(|c| c.is_ascii_digit())
            .collect();
        if let Ok(n) = digits.parse::<usize>() {
            return Ok(Action::Take(Token(n)));
        }
    }
    Err("expected PASS or SWAP <n>")
}
