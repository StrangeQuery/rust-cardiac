//! Products: of two numbers off cards (the multiply routine), and by constants (adds and shifts). A product past
//! what a card holds overflows.
use cardiac::*;

fn main() {
    let x = inp();
    let y = inp();
    out(x * y);
    out(x * 7);
    out(x * -3);
    out(y * 10);
}
