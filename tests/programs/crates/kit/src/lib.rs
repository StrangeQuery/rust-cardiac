//! A library crate for the two-crate examples: functions, a table, a struct and its consts.
#![no_std]

/// A point in space, three cells.
#[derive(Clone, Copy)]
pub struct Point { pub x: i16, pub y: i16, pub z: i16 }

impl Point {
    pub const ORIGIN: Point = Point { x: 0, y: 0, z: 0 };
    pub const FAR: Point = Point { x: 30, y: -40, z: 12 };
}

pub const BASE: Point = Point { x: 12, y: 30, z: 30 };

pub const STEP: i16 = 3;

pub mod tables {
    pub const SQUARES: [i16; 5] = [0, 1, 4, 9, 16];

    /// i squared, looked up: the table read inside this crate.
    #[inline(never)]
    pub fn square(i: usize) -> i16 { SQUARES[i] }
}

pub mod arith {
    #[inline(never)]
    pub fn twice(v: i16) -> i16 { v + v }

    /// Four times v: one function here calling another.
    #[inline(never)]
    pub fn quad(v: i16) -> i16 { twice(twice(v)) }

    /// Small enough that rustc inlines it into the caller at opt-level 2.
    pub fn diff(a: i16, z: i16) -> i16 { if a < z { z - a } else { a - z } }
}

/// The point (v, v, v) (the origin for none), built here and returned whole.
#[inline(never)]
pub fn diagonal(v: i16) -> Point { if v > 0 { Point { x: v, y: v, z: v } } else { Point::ORIGIN } }

/// Moved by STEP along each axis.
#[inline(never)]
pub fn moved(p: Point) -> Point { Point { x: p.x + STEP, y: p.y + STEP, z: p.z + STEP } }

/// The first element of any table: generic, so rustc writes no MIR for it a caller in another crate could use.
#[inline(never)]
pub fn first<const N: usize>(t: [i16; N]) -> i16 { t[0] }
