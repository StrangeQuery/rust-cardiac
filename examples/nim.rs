//! Single-pile Nim, as the CARDIAC manual sets it: ten pebbles in a pile, and each player in turn takes one, two or
//! three of them, but never as many as the other player just took. A player who can't move loses: the pile is
//! empty, or one pebble is left and the other player just took one.
//!
//! You play CARDIAC. The first card says who starts: 0 for CARDIAC, 1 for you. Then each of your moves is a card,
//! 1 to 3, and each of CARDIAC's is a value it outputs, how many it takes; a move of yours that the rules don't allow
//! is answered -1, and CARDIAC reads another. When you can't move CARDIAC outputs 999, it has won; when it can't,
//! 0, you have. Typing your moves as it asks, `burn nim.cqb`; or all at once, `burn
//! --bin nim -- 1 1 3 1`.
//!
//! CARDIAC plays as well as anyone can. Worked back from the end, a pile of 4k is lost for the player to move, and
//! one of 4k + 1, 4k + 2 or 4k + 3 is won by taking 1, 2 or 3 (2 or 1 from 4k + 2), unless that is the number just
//! taken. From ten, whoever starts wins.
use cardiac::*;

const PEBBLES: i16 = 10;

/// CARDIAC's move, when it may take `least` or more: the one that leaves a multiple of 4, if it is allowed (from
/// 4k + 2 after a 2, that is 1); if none is, the position is lost, and it takes as few as it may, to play on. The
/// pile is never more than ten, so its remainder by 4 is a few subtractions, where `%` would bring in a division
/// routine.
fn choose(pile: i16, last: i16, least: i16) -> i16 {
    let mut take = pile;
    while take >= 4 {
        take -= 4;
    }
    if take == 0 || take == last {
        take = least;
    }
    take
}

fn main() {
    let mut pile = PEBBLES;
    // how many the player who moved last took (0: no one has moved)
    let mut last = 0;
    let mut cardiacs_turn = inp() == 0;
    loop {
        // the fewest the player to move may take: 1, or 2 just after a 1
        let least = if last == 1 { 2 } else { 1 };
        if pile < least {
            // the player to move can't, and has lost
            out(if cardiacs_turn { 0 } else { 999 });
            return;
        }
        let take = if cardiacs_turn {
            let take = choose(pile, last, least);
            out(take);
            take
        } else {
            let mut card = inp();
            while card < 1 || card > 3 || card == last || card > pile {
                out(-1);
                card = inp();
            }
            card
        };
        pile -= take;
        last = take;
        cardiacs_turn = !cardiacs_turn;
    }
}
