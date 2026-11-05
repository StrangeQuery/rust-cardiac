#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

/// Methods rustc calls rather than inlines: an inherent method and a trait's, each a function of its own,
/// defined (in the driver's IR) under the name the calls give them.
#[derive(Clone, Copy)]
struct P {
    x: i16,
    y: i16,
}

impl P {
    #[inline(never)]
    fn sum(self) -> i16 {
        self.x + self.y
    }
}

trait Twice {
    fn twice(self) -> i16;
}

impl Twice for P {
    #[inline(never)]
    fn twice(self) -> i16 {
        2 * self.sum()
    }
}

#[inline(never)]
fn make(x: i16) -> P {
    P { x, y: x + 1 }
}

pub fn main() {
    let mut k = 0;
    while k < 3 {
        let p = make(k);
        out(p.sum());
        out(p.twice());
        k += 1;
    }
}
