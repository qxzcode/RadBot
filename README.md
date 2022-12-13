# RadBot

This is an AI that plays the card game [Radlands](https://roxley.com/products/radlands) by Roxley Games. It's somewhat of a work-in-progress, but can already play well in the subset of the game that has been implemented.

## How does the AI work?

RadBot uses [Monte Carlo tree search](https://en.wikipedia.org/wiki/Monte_Carlo_tree_search) with uniform random rollouts.

At a high level, the algorithm repeatedly runs simulations of future outcomes. When performing a simulation from a given game state, it picks a valid action that can be taken in that state. It is biased toward picking actions that, in previous simulations, have led to a high proportion of wins. A certain formula balances this with picking actions that have not been simulated many times, so that the algorithm explores many possible lines of play.

After picking an action, it simulates what new state the game ends up in, then chooses an action to perform from that state. It recursively traverses down the tree in this manner until it reaches a node that has never been simulated before.

This new leaf node is added to the tree. Random moves are then made until a game-ending state is reached, and the winning player is determined. Then, all nodes along the path from the root node to this leaf record that result and update their local "win rates."

After performing many simulations like this, the algorithm will identify lines of play where each player maximizes their probability of winning. In the limit, it converges to game-theoretic optimal play – although the naive uniform random rollouts that this program currently uses causes it to sometimes misjudge long-term consequences and blunder.

## Usage

To run the program:

1. Make sure you have [Rust installed](https://www.rust-lang.org/tools/install).
2. Clone/download this repo, and go to it in a terminal.
    - Tip: make your terminal window nice and big. There's a lot in the UI.
3. Run `cargo run --release -- --ui`
    - When run for the first time, this will automatically build the executable.

The UI is terminal-based and lets you play against the AI. By default, the AI will "think" for 3 seconds per action. The AI is Player 1; you are Player 2.

 - Press <kbd>Enter</kbd> to focus the input bar when it is your turn to choose an action. Type the number of the action you wish to make, then press <kbd>Enter</kbd> to submit it. Press <kbd>Esc</kbd> to un-focus the input bar.
 - Press <kbd>D</kbd> to toggle the <b>d</b>ebug stats view between showing (a) the options at the current choice root or (b) the most-visited sequence of actions.
 - Press <kbd>S</kbd> to <b>s</b>hrink the "Options" pane to fit the current set displayed.
 - Press <kbd>Q</kbd> to <b>q</b>uit the program.
