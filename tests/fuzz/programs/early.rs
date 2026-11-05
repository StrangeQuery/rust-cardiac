//! A panic before the first card is read or punched. Natively the -999 comes from the built-ins crate's panic hook,
//! which it now sets up before main; on CARDIAC main is no code before the panic routine.
use cardiac::*;

const LIMIT: i16 = 3;

fn main() {
    if LIMIT > 2 {
        panic!();
    }
    out(inp());
}
