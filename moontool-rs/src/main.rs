mod cards;

use crate::cards::{CardType, Cards};

fn main() {
    println!("Hello, world!");
    let n_draw = 3;

    println!("Empty:");
    for (deck, drawn, prob) in Cards::new().enumerate_draws(n_draw) {
        println!("{}, {}, {}", deck, drawn, prob);
    }

    println!("Full:");
    let a = CardType { play_func: || 1.0, letter: 'R', color: "96" };
    let cards = Cards::from_iter(&[
        &a,
        &a,
        &CardType { play_func: || 1.0, letter: 'T', color: "93" },
        &CardType { play_func: || 1.0, letter: 'S', color: "92" },
    ]);
    for (deck, drawn, prob) in cards.enumerate_draws(n_draw) {
        println!("{}, {}, {}", deck, drawn, prob);
    }

    println!("Drop:");
    let iter = cards.enumerate_draws(n_draw);
    std::mem::drop(cards);
    for (deck, drawn, prob) in iter {
        println!("{}, {}, {}", deck, drawn, prob);
    }
}
