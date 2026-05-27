# Simplified *The Gang* solution and LLM play

[*The Gang*](https://whatsericplaying.com/2025/01/27/the-gang) is a cooperative poker game.

The standard rules are all over the place. Worst of all there's a trivial and boring solution that wins every time, as each player can fully communicate their hidden cards in multiple ways. The most trivial is: pick a token, hold it for N seconds where N is the rank of the card. Put the token back. Repeat for your other card. This way all players will know exactly what cards you hold. If needed, you can also communicate suit with token number, but it doesn't seem to matter in this game. If the timing side-channel is prohibited, you can do morse code or whatever other method you wish by alternating which token you pick. Naturally such methods take all fun out of the game, and to my great astonishment you can simply refuse to abuse them. I was, however, rather interested in seeing if any LLMs could figure this out.

As timing side-channels are rather annoying for automatic play, especially as LLMs can take a lot of time to think, we'll consider a simplified version of the game instead. The most important change is that we only play the *river* phase, i.e. all five community cards have been revealed and only the final round of chip assignment remains. No modifier cards are used. The game is played in a turn-based fashion with a fixed upper limit. By tuning the turn limit, it's possible to enforce probabilistic play.

## Actual rules

There are N players and N tokens numbered 0 to N−1. Token K means "exactly K other players have a worse hand than me". All tokens start from the unclaimed token pool. Hands are ordered using standard [Texas hold 'em rules](https://en.wikipedia.org/wiki/Texas_hold_%27em). Players take turns; on each turn a player may:

- **PASS** — do nothing
- **TAKE K** — take token K (from the unclaimed pool or from another player); your previously held token, if any, returns to the pool
- **RETURN** — return your current token to the pool

The game ends immediately once every player holds a token, or after a fixed round limit. Victory requires every player to hold the correct token: token K for a player who beats exactly K others. Tied hands are interchangeable, and any assignment within a tied group is accepted.

The only allowed communication channel is the observable sequence of token actions.

## Usage

This project relies on a [ollama](https://ollama.com/) to run the LLMs. I've tried it with the following models: `llama3.3:70b`, `qwen3.6:latest`, and `gemma3:latest`. They tend to work and at least didn't produce invalid output in my testing.

Example:

```bash
cargo run -- --model llama3.3:70b --players 4 --rounds 10
```
