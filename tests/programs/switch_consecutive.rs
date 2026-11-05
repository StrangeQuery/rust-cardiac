#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}

#[derive(Clone, Copy)]
enum Dir {
    Left,
    Ahead,
    Right,
}

#[inline(never)]
fn turn(d: Dir) -> i16 {
    match d {
        Dir::Left => -1,
        Dir::Ahead => 0,
        Dir::Right => 1,
    }
}

/// A switch on consecutive values, a match's `-1 => .., 0 => .., 1 => ..` or an enum's variants: one
/// subtraction and a SUB 00 and TAC an arm. Values below, among and above the arms.
pub fn main() {
    let mut x = -3;
    while x < 4 {
        out(match x {
            -1 => 10,
            0 => 20,
            1 => 30,
            _ => 40,
        });
        x += 1;
    }
    out(turn(Dir::Left));
    out(turn(Dir::Ahead));
    out(turn(Dir::Right));
}
