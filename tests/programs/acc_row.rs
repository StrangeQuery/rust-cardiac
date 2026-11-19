#![no_std]
#[inline(never)]
pub fn out(_v: i16) {}
#[inline(never)]
pub fn inp() -> i16 {
    0
}

unsafe fn assume(condition: bool) {
    unsafe { core::hint::assert_unchecked(condition) }
}

/// A row of adds that blocks fall into, each block also entered from the match (#36): the sum stays in the
/// accumulator from one block into the next, and each way in from the match loads it on its own way. Each add is
/// then one instruction; test_compiler.sh checks the sum and the instructions a run takes for each d.
pub fn main() {
    {
        let d = inp();
        let c = inp();
        unsafe { assume(d >= 0 && d <= 9) };
        let mut s = 0;
        'r0: {
            'r1: {
                'r2: {
                    'r3: {
                        'r4: {
                            'r5: {
                                'r6: {
                                    'r7: {
                                        'r8: {
                                            match d {
                                                0 => break 'r0,
                                                1 => break 'r1,
                                                2 => break 'r2,
                                                3 => break 'r3,
                                                4 => break 'r4,
                                                5 => break 'r5,
                                                6 => break 'r6,
                                                7 => break 'r7,
                                                8 => break 'r8,
                                                _ => {}
                                            }
                                            s += c;
                                        }
                                        s += c;
                                    }
                                    s += c;
                                }
                                s += c;
                            }
                            s += c;
                        }
                        s += c;
                    }
                    s += c;
                }
                s += c;
            }
            s += c;
        }
        out(s);
    }
}
