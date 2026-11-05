//! A running sum, smallest and largest of the numbers off the cards, printed after each, until the cards run out.
//! A sum past what a card holds overflows.
use cardiac::*;

fn main() {
    let mut sum = 0;
    let mut least = 999;
    let mut most = -999;
    loop {
        let v = inp();
        sum += v;
        if v < least {
            least = v;
        }
        if v > most {
            most = v;
        }
        out(sum);
        out(least);
        out(most);
    }
}
