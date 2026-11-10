# What cargo-cardiac compiles

Every row is a small program here. Each is compiled for CARDIAC four ways (opt-level 1 and 2, with
overflow checks on and off), run, and compared with the same program run natively on the same input. The
status is what those runs did: **Supported** means CARDIAC printed what native Rust printed every time,
**Refused** that the build stopped with an error every time. Cells are for a release build (opt-level 2,
overflow checks off), then with overflow checks on. `./test_features.sh` remakes this page.

Made with cargo-cardiac v0.1.1-150-g861d383.

| Status | Rows |
|---|---|
| Differs by design | 3 |
| Refused | 21 |
| Supported | 722 |
| Test too big | 2 |

## Types

### Boolean type

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| `&`, `\|` and `!` on `bool` (not short-circuiting) | Supported | 48 / 50 |  |  | [bool_ops.rs](types/bool_ops.rs) |
| A `bool` passed to and returned from a function | Supported | 40 / 40 |  |  | [bool_param.rs](types/bool_param.rs) |
| A `bool` made from a comparison, cast to `i16` | Supported | 24 / 24 |  |  | [bool_value.rs](types/bool_value.rs) |
| `^` on `bool` | Supported | 39 / 41 |  | `a ^ b` on `bool` is `a != b`, which the compiler already does. | [bool_xor.rs](types/bool_xor.rs) |

### Integer types

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| `i16 as i8` and `i8 as i16` | Supported | 35 / 35 |  | The values here fit in an `i8`, so the cast keeps them as they are. | [cast_i16_i8.rs](types/cast_i16_i8.rs) |
| `u8 as usize` and `usize as u8` | Supported | 41 / 48 |  |  | [cast_u8_usize.rs](types/cast_u8_usize.rs) |
| An `i32` holding more than a card (50 * 50 = 2500), divided back onto one | Differs from native Rust by design | 36 / 54 | Never | A cell holds -999..=999, so an `i32` past a card cannot be held, even for a moment. The product 2500 is past a card: with overflow checks on CARDIAC panics, and with them off it keeps the low three digits (500), so it prints 50 where Rust prints 250. | [i32_past_card.rs](types/i32_past_card.rs) |
| `i128`: declared, added, subtracted, compared and cast back to `i16` | Refused |  | Never | The compiler says: `RCE0004: unsupported local type i128 for _1 (this backend has u8, i16, usize and bool)` | [int_i128.rs](types/int_i128.rs) |
| `i16`: declared, added, subtracted, compared and cast back to `i16` | Supported | 33 / 46 |  |  | [int_i16.rs](types/int_i16.rs) |
| `i32`: declared, added, subtracted, compared and cast back to `i16` | Supported | 33 / 48 |  | The values fit on a card here; an `i32` past a card is its own row. | [int_i32.rs](types/int_i32.rs) |
| `i64`: declared, added, subtracted, compared and cast back to `i16` | Refused |  | Never | The compiler says: `RCE0004: unsupported local type i64 for _1 (this backend has u8, i16, usize and bool)` | [int_i64.rs](types/int_i64.rs) |
| `i8`: declared, added, subtracted, compared and cast back to `i16` | Supported | 75 / 72 |  | An `i8` holds -128..=127, all on a card; only the overflow check at 127 is new. | [int_i8.rs](types/int_i8.rs) |
| `u8::MAX`, `i8::MIN` and `i8::MAX` used with an input | Supported | 36 / 49 |  |  | [int_min_max.rs](types/int_min_max.rs) |
| `u128`: declared, added, subtracted, compared and cast back to `i16` | Refused |  | Never | The compiler says: `RCE0004: unsupported local type u128 for _1 (this backend has u8, i16, usize and bool)` | [int_u128.rs](types/int_u128.rs) |
| `u16`: declared, added, subtracted, compared and cast back to `i16` | Supported | 41 / 49 |  | Every `u16` on a card is fine and one past a card can't be held, as for `i16`. | [int_u16.rs](types/int_u16.rs) |
| `u32`: declared, added, subtracted, compared and cast back to `i16` | Refused |  | Never | The compiler says: `RCE0004: unsupported local type u32 for _1 (this backend has u8, i16, usize and bool)` | [int_u32.rs](types/int_u32.rs) |
| `u64`: declared, added, subtracted, compared and cast back to `i16` | Refused |  | Never | The compiler says: `RCE0004: unsupported local type u64 for _1 (this backend has u8, i16, usize and bool)` | [int_u64.rs](types/int_u64.rs) |
| `u8`: declared, added, subtracted, compared and cast back to `i16` | Supported | 79 / 76 |  |  | [int_u8.rs](types/int_u8.rs) |

### Machine-dependent integer types

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| `isize`: declared, added, subtracted, compared and cast back to `i16` | Supported | 33 / 48 |  | An `isize` is 64 bits natively, but one that fits on a card can be held as an `i16` is. | [int_isize.rs](types/int_isize.rs) |
| `usize`: declared, added, subtracted, compared and cast back to `i16` | Supported | 40 / 57 |  |  | [int_usize.rs](types/int_usize.rs) |

### Integer literals

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| Byte literals (`b'a'`, `b'\n'`), which are `u8` | Supported | 52 / 50 |  |  | [literal_byte.rs](types/literal_byte.rs) |
| A byte string literal (`b"CARD"`), indexed by an input | Supported | 28 / 30 |  | A byte string literal is a constant `u8` table, which the compiler already has for `const` arrays. | [literal_byte_string.rs](types/literal_byte_string.rs) |
| Hexadecimal, octal and binary literals (`0x1F`, `0o17`, `0b101`) | Supported | 24 / 41 |  |  | [literal_hex_oct_bin.rs](types/literal_hex_oct_bin.rs) |
| Integer literals with type suffixes (`5i16`, `3u8`, `2usize`) | Supported | 44 / 50 |  |  | [literal_suffix.rs](types/literal_suffix.rs) |
| Literals with underscores (`1_0`, `0b_1_1`, `9_9_9`) | Supported | 19 / 33 |  |  | [literal_underscore.rs](types/literal_underscore.rs) |

### Floating-point types

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| `f32` arithmetic, cast back to `i16` | Refused |  | Never | The compiler says: `RCE0004: unsupported local type f32 for _1 (this backend has u8, i16, usize and bool)` | [float_f32.rs](types/float_f32.rs) |
| `f64` arithmetic and comparison | Refused |  | Never | The compiler says: `RCE0004: unsupported local type f64 for _1 (this backend has u8, i16, usize and bool)` | [float_f64.rs](types/float_f64.rs) |

### Textual types

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| `char` literals compared with an input (`'0'..='9'`) | Supported | 40 / 42 |  |  | [char_literal.rs](types/char_literal.rs) |
| A `char` from a `u8`, compared and cast back | Supported | 45 / 45 |  | An ASCII `char` fits on a card; one past 999 never would. | [char_value.rs](types/char_value.rs) |
| `==` of two string literals chosen by inputs | Refused |  | Never | A string compare is a byte loop over every literal it might meet, each a cell a byte: two short literals compared come to about 124 cells, more than CARDIAC has. The compiler says: `RCE0007: call to unknown function compare_bytes` | [str_eq.rs](types/str_eq.rs) |
| `.len()` of one of two string literals chosen by an input | Supported | 19 / 21 |  | A `&str` is a slice: an address and a length. | [str_len.rs](types/str_len.rs) |
| A string literal's bytes, indexed by an input | Supported | 37 / 37 |  | A `&str` passed to a function is refused (RCE0019: the length of the slice it is passed isn't known here), though its `.as_bytes()` can be passed. | [str_literal_bytes.rs](types/str_literal_bytes.rs) |

### Never type

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| `break` as an expression of type `!` in one arm of an `if` | Supported | 12 / 23 |  |  | [never_break_expr.rs](types/never_break_expr.rs) |
| A function returning `!` (it panics), called on one branch | Supported | 13 / 13 |  | A call to a function that never returns is an ordinary call with nothing after it; `panic!()` itself already works. | [never_fn.rs](types/never_fn.rs) |

### Tuple types

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| A nested tuple `((i16, i16), i16)` | Supported | 15 / 28 |  |  | [tuple_nested.rs](types/tuple_nested.rs) |
| A nested tuple returned from a function | Supported | 36 / 53 |  |  | [tuple_nested_return.rs](types/tuple_nested_return.rs) |
| A tuple of `i16`, `bool` and `u8` | Supported | 21 / 32 |  |  | [tuple_scalars.rs](types/tuple_scalars.rs) |
| The unit type `()`: a function returning it, a tuple holding it | Supported | 16 / 27 |  |  | [tuple_unit.rs](types/tuple_unit.rs) |

### Array types

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| An array of arrays `[[i16; 2]; 2]` | Supported | 53 / 53 |  | It is one array, laid out by tens where the program fits: `t[r][c]` is `t[10 * r + c]`, the row one shift. | [array_arrays.rs](types/array_arrays.rs) |
| An array of `bool` | Supported | 52 / 52 |  |  | [array_bool.rs](types/array_bool.rs) |
| An array of `char` | Supported | 56 / 56 |  |  | [array_char.rs](types/array_char.rs) |
| An array of `i16` | Supported | 41 / 41 |  |  | [array_i16.rs](types/array_i16.rs) |
| An array of `i32` whose values fit on a card | Supported | 43 / 43 |  |  | [array_i32.rs](types/array_i32.rs) |
| An array of `i8` | Supported | 56 / 56 |  |  | [array_i8.rs](types/array_i8.rs) |
| `.len()` of an array passed by reference | Supported | 55 / 69 |  |  | [array_len.rs](types/array_len.rs) |
| An array made with `[x; N]` from an input | Supported | 19 / 27 |  |  | [array_repeat.rs](types/array_repeat.rs) |
| An array of structs | Supported | 60 / 70 |  |  | [array_structs.rs](types/array_structs.rs) |
| An array of tuples `[(i16, i16); 3]` | Supported | 43 / 43 |  | The compiler could keep one array per tuple field, as it does for an array of structs. | [array_tuples.rs](types/array_tuples.rs) |
| An array of `u8` | Supported | 61 / 61 |  |  | [array_u8.rs](types/array_u8.rs) |
| An array of `usize` | Supported | 45 / 45 |  | A `usize` element is a cell, as an `i16` one is. | [array_usize.rs](types/array_usize.rs) |
| A zero-length array `[i16; 0]` | Supported | 21 / 32 |  |  | [array_zero.rs](types/array_zero.rs) |

### Slice types

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| A mutable slice `&mut [i16]` written by a function | Supported | 70 / 72 |  |  | [slice_mut.rs](types/slice_mut.rs) |
| A slice `&[i16]` passed to a function and read in a loop | Supported | 52 / 62 |  | A slice is an address and a length, two cells, read the way arrays by reference already are. | [slice_param.rs](types/slice_param.rs) |
| A sub-slice `&t[1..3]`, indexed | Supported | 32 / 32 |  |  | [slice_range.rs](types/slice_range.rs) |

### Struct types

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| A generic struct `Pair<T>` with `T = i16` and `T = u8` | Supported | 43 / 53 |  |  | [struct_generic.rs](types/struct_generic.rs) |
| A tuple struct `struct Meters(i16)` passed and returned | Supported | 15 / 26 |  |  | [struct_tuple.rs](types/struct_tuple.rs) |
| A struct type with named fields | Supported | 9 / 20 |  |  | [struct_type.rs](types/struct_type.rs) |
| A unit struct passed to a function | Supported | 13 / 24 |  | A unit struct is no cells at all. | [struct_unit.rs](types/struct_unit.rs) |

### Enumerated types

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| An enum type with and without data | Supported | 53 / 53 |  |  | [enum_type.rs](types/enum_type.rs) |

### Union types

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| A `union` of `i16` and `u8`, read back through the field written | Supported | 27 / 27 |  | Reading another field than the one written would reinterpret bytes, which a decimal cell has none of. | [union_basic.rs](types/union_basic.rs) |

### Function item types

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| A function item held in a variable and called | Supported | 13 / 24 |  |  | [fn_item.rs](types/fn_item.rs) |

### Closure types

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| A closure that captures a variable, called where it is made | Supported | 9 / 20 |  | At opt-level 1 rustc calls the closure rather than inlining it; the closure is a struct of references and a function, both of which the compiler already handles. | [closure_capture.rs](types/closure_capture.rs) |
| An `FnMut` closure that changes a captured counter | Supported | 16 / 28 |  | At opt-level 1 rustc calls the closure rather than inlining it; the closure is a struct of references and a function, both of which the compiler already handles. | [closure_fnmut.rs](types/closure_fnmut.rs) |
| A closure passed to a generic `F: Fn(i16) -> i16` parameter | Supported | 31 / 42 |  | A generic function rustc doesn't inline gets one copy per closure type, each an ordinary function. | [closure_generic_arg.rs](types/closure_generic_arg.rs) |

### Pointer types

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| Pointer arithmetic: `p.add(i)` into an array | Supported | 16 / 16 |  |  | [raw_add.rs](types/raw_add.rs) |
| A raw pointer `*const i16` read with `unsafe` | Supported | 7 / 18 |  |  | [raw_const.rs](types/raw_const.rs) |
| A raw pointer `*mut i16` written through in a function | Supported | 24 / 36 |  |  | [raw_mut_fn.rs](types/raw_mut_fn.rs) |
| A mutable reference `&mut i16` passed to a function | Supported | 30 / 42 |  |  | [ref_mut.rs](types/ref_mut.rs) |
| A `&mut` to an array element chosen by an input | Supported | 49 / 58 |  |  | [ref_mut_element.rs](types/ref_mut_element.rs) |
| A shared reference `&i16` passed to a function | Supported | 13 / 24 |  |  | [ref_shared.rs](types/ref_shared.rs) |
| A reference to a reference `&&i16` | Supported | 30 / 41 |  |  | [ref_to_ref.rs](types/ref_to_ref.rs) |

### Function pointer types

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| An array of function pointers indexed by an input | Supported | 55 / 63 |  |  | [fnptr_array.rs](types/fnptr_array.rs) |
| A function pointer `fn(i16) -> i16` chosen at run time | Supported | 46 / 59 |  | An indirect call is a computed jump, which CARDIAC can make with self-modifying code. | [fnptr_choose.rs](types/fnptr_choose.rs) |
| A function pointer passed as a parameter | Supported | 25 / 39 |  |  | [fnptr_param.rs](types/fnptr_param.rs) |

### Trait object types

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| A `&dyn Trait` whose type is chosen at run time | **The test is too big** (it doesn't fit in 96 cells) | 89 | On demand | The compiler says: `RCE0032: program runs past cell 98` | [dyn_choose.rs](types/dyn_choose.rs) |
| A `&dyn Trait` with one implementation | Supported | 48 / 65 |  | Dynamic dispatch needs a vtable and an indirect call, the same machinery as function pointers. | [dyn_one.rs](types/dyn_one.rs) |

### Impl trait type

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| `impl Trait` as a parameter type | Supported | 63 / 88 |  | `impl Trait` in argument position is a generic function. | [impl_arg.rs](types/impl_arg.rs) |
| `impl Fn(i16) -> i16` returned from a function | Supported | 9 / 20 |  | At opt-level 1 rustc calls `adder`, which returns a closure: a struct the compiler would have to pass back. | [impl_return.rs](types/impl_return.rs) |

### Type aliases

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| Type aliases `type Score = i16` and `type Pair<T> = (T, T)` | Supported | 15 / 26 |  |  | [type_alias.rs](types/type_alias.rs) |

### Type layout

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| `Option<bool>`, whose `None` fills a niche in `bool` | Supported | 43 / 44 |  |  | [option_bool.rs](types/option_bool.rs) |
| `Option` of a C-like enum, whose `None` is a spare variant number | Supported | 35 / 36 |  | The refusal is a constant `Some` of a C-like enum, which is just two numbers. | [option_enum.rs](types/option_enum.rs) |
| `Option<i16>` returned from a function | Supported | 30 / 51 |  |  | [option_i16.rs](types/option_i16.rs) |
| `Option<NonZeroI16>`, whose `None` is zero | Supported | 16 / 18 |  |  | [option_nonzero.rs](types/option_nonzero.rs) |
| A nested `Option<Option<i16>>` | Supported | 48 / 50 |  |  | [option_option.rs](types/option_option.rs) |
| `Option<&i16>`, whose `None` is the null pointer | Supported | 50 / 52 |  |  | [option_ref.rs](types/option_ref.rs) |
| `Option<u8>` | Supported | 54 / 56 |  |  | [option_u8.rs](types/option_u8.rs) |
| `Option<()>`, laid out as a `bool` | Supported | 30 / 32 |  | `Option<()>` is a `bool`. | [option_unit.rs](types/option_unit.rs) |

## Expressions

### Literal expressions

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| `true`, `false` and a `char` literal cast to a number (`'A' as i16`) | Supported | 39 / 39 |  | A `char` is a number, its code point, and one that fits on a card can be held as a `u8` is. | [lit_bool_char.rs](expressions/lit_bool_char.rs) |
| A byte literal (`b'A'`) | Supported | 19 / 19 |  |  | [lit_byte.rs](expressions/lit_byte.rs) |
| Integer literals: decimal, `0x`, `0o`, `0b`, with `_` separators and a type suffix | Supported | 31 / 59 |  |  | [lit_int_forms.rs](expressions/lit_int_forms.rs) |
| A negative literal (`-999`) and the largest a card holds (`999`) | Supported | 25 / 25 |  |  | [lit_negative.rs](expressions/lit_negative.rs) |

### Path expressions

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| An associated constant (`Point::ORIGIN_X`) | Supported | 8 / 19 |  |  | [path_assoc_const.rs](expressions/path_assoc_const.rs) |
| A `const` used in an expression | Supported | 20 / 45 |  |  | [path_const.rs](expressions/path_const.rs) |
| `i16::MAX` compared with a value (`a < i16::MAX`) | Supported | 9 / 9 |  | 32767 is past what a card holds, but comparing a value with it works; `i16::MAX - a` is refused on purpose (RCE0009). | [path_i16_max.rs](expressions/path_i16_max.rs) |
| Reading a `static` | Supported | 8 / 19 |  |  | [path_static.rs](expressions/path_static.rs) |
| Indexing a `static` array | Supported | 42 / 52 |  | A `static` is a fixed place in memory, which every CARDIAC variable already is; a `const` table works today. | [path_static_array.rs](expressions/path_static_array.rs) |
| Reading and writing a `static mut` | Supported | 31 / 44 |  | A `static` is a fixed place in memory, which every CARDIAC variable already is. | [path_static_mut.rs](expressions/path_static_mut.rs) |
| A trait's associated constant (`T::SIZE` in a generic function) | Supported | 14 / 27 |  | At opt-level 1 rustc doesn't inline the generic function, and generic functions are compiled only when rustc inlines them. | [path_trait_assoc_const.rs](expressions/path_trait_assoc_const.rs) |
| `u8::MAX` as a value | Supported | 8 / 19 |  |  | [path_u8_max.rs](expressions/path_u8_max.rs) |

### Block expressions

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| An inline `const { ... }` block | Supported | 8 / 19 |  |  | [block_const.rs](expressions/block_const.rs) |
| Nested blocks shadowing a name | Supported | 13 / 26 |  |  | [block_nested_scope.rs](expressions/block_nested_scope.rs) |
| An `unsafe` block as a value | Supported | 7 / 18 |  |  | [block_unsafe.rs](expressions/block_unsafe.rs) |
| A block as a value, `let x = { ...; e };` | Supported | 11 / 25 |  |  | [block_value.rs](expressions/block_value.rs) |

### Borrow operators

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| `&mut t[i]` to an array element, passed to a function | Supported | 47 / 57 |  |  | [borrow_element.rs](expressions/borrow_element.rs) |
| `&mut s.f` to a struct field, passed to a function | Supported | 35 / 47 |  |  | [borrow_field.rs](expressions/borrow_field.rs) |
| `&mut x` passed to a function that changes it | Supported | 31 / 43 |  |  | [borrow_mut.rs](expressions/borrow_mut.rs) |
| `&x` passed to a function taking `&i16` | Supported | 15 / 26 |  |  | [borrow_shared.rs](expressions/borrow_shared.rs) |

### Dereference operator

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| A reference returned from a function, `pick(c, &a, &b) -> &i16`, then read | Supported | 52 / 52 |  | Wrong answer: reading through the reference the function returns gets the wrong cells (844 and 100 for 40 and 50). | [deref_choose.rs](expressions/deref_choose.rs) |
| `*r` through a local reference, reading and writing | Supported | 8 / 23 |  |  | [deref_local.rs](expressions/deref_local.rs) |
| `*p` through a raw pointer to an array element | Supported | 29 / 29 |  |  | [deref_raw.rs](expressions/deref_raw.rs) |
| `**r` through a reference to a reference | Supported | 29 / 40 |  | A reference to a reference is an address held in a cell, which the compiler doesn't follow twice yet. | [deref_ref_ref.rs](expressions/deref_ref_ref.rs) |

### The question mark operator

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| Several `?` in one function: the first `None` returns | Supported | 52 / 92 |  |  | [try_chain.rs](expressions/try_chain.rs) |
| `?` on an `Option` in a function returning `Option` | Supported | 52 / 64 |  |  | [try_option.rs](expressions/try_option.rs) |
| `?` on a `Result` in a function returning `Result` | Supported | 53 / 81 |  |  | [try_result.rs](expressions/try_result.rs) |
| `?` converting the error with `From` (`Result<_, u8>` into `Result<_, i16>`) | Supported | 57 / 88 |  | Wrong answer at opt-level 1: the error comes back unconverted (7, not 70), as if `From` weren't called. | [try_result_from.rs](expressions/try_result_from.rs) |

### Arithmetic operators

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| `a + b` on `i16` | Supported | 19 / 30 |  |  | [add_i16.rs](expressions/add_i16.rs) |
| `a + b` past a card (999 + 1) | Differs from native Rust by design | 9 / 20 | Never | A cell holds -999 to 999. With overflow checks on, CARDIAC panics where native Rust prints 1000; with them off it keeps the low three digits. | [add_past_card.rs](expressions/add_past_card.rs) |
| `a + b` on `u8` | Supported | 57 / 57 |  |  | [add_u8.rs](expressions/add_u8.rs) |
| `a + b` on `u8` past 255: panics, or wraps with checks off | Supported | 57 / 57 |  |  | [add_u8_overflow.rs](expressions/add_u8_overflow.rs) |
| `a + b` on `usize` | Supported | 14 / 27 |  |  | [add_usize.rs](expressions/add_usize.rs) |
| Integer arithmetic with no type given (`i32`, Rust's default) | Supported | 19 / 43 |  |  | [arith_i32_default.rs](expressions/arith_i32_default.rs) |
| Arithmetic on a variable declared `i32` | Supported | 35 / 56 |  |  | [arith_i32_typed.rs](expressions/arith_i32_typed.rs) |
| `a / 7`: by a constant other than a power of ten | Supported | 57 / 57 |  |  | [div_const_i16.rs](expressions/div_const_i16.rs) |
| `a / 10`: by a power of ten | Supported | 13 / 13 |  | A shift. Rounds toward zero: `-7 / 10` is 0. | [div_pow10_i16.rs](expressions/div_pow10_i16.rs) |
| `a / b` on `u8` | Supported | 66 / 66 |  |  | [div_u8.rs](expressions/div_u8.rs) |
| `a / b` on `usize` | Supported | 32 / 32 |  |  | [div_usize.rs](expressions/div_usize.rs) |
| `a / b` on `i16`, both variables | Supported | 65 / 65 |  | A divide routine, put in once, runs as many steps as the quotient. Rounds toward zero, as Rust does. | [div_var_i16.rs](expressions/div_var_i16.rs) |
| `a / b` with `b` zero panics | Supported | 55 / 55 |  |  | [div_zero_i16.rs](expressions/div_zero_i16.rs) |
| `a * 7`: by a constant | Supported | 16 / 27 |  |  | [mul_const_i16.rs](expressions/mul_const_i16.rs) |
| `a * 10`: by a power of ten | Supported | 13 / 34 |  |  | [mul_pow10_i16.rs](expressions/mul_pow10_i16.rs) |
| `a * b` on `u8` | Supported | 82 / 90 |  |  | [mul_u8.rs](expressions/mul_u8.rs) |
| `a * b` on `usize` | Supported | 46 / 59 |  |  | [mul_usize.rs](expressions/mul_usize.rs) |
| `a * b` on `i16`, both variables | Supported | 44 / 61 |  | A multiply routine, put in once, runs as many steps as the smaller factor. | [mul_var_i16.rs](expressions/mul_var_i16.rs) |
| `-a` on `i16` | Supported | 14 / 14 |  |  | [neg_i16.rs](expressions/neg_i16.rs) |
| `a % 7`: by a constant other than a power of ten | Supported | 68 / 68 |  |  | [rem_const_i16.rs](expressions/rem_const_i16.rs) |
| `a % 10`: by a power of ten | Supported | 13 / 13 |  |  | [rem_pow10_i16.rs](expressions/rem_pow10_i16.rs) |
| `a % b` on `u8` | Supported | 57 / 57 |  |  | [rem_u8.rs](expressions/rem_u8.rs) |
| `a % b` on `usize` | Supported | 24 / 24 |  |  | [rem_usize.rs](expressions/rem_usize.rs) |
| `a % b` on `i16`, both variables | Supported | 78 / 78 |  | The remainder takes the sign of `a`, as in Rust. | [rem_var_i16.rs](expressions/rem_var_i16.rs) |
| `a % b` with `b` zero panics | Supported | 67 / 67 |  |  | [rem_zero_i16.rs](expressions/rem_zero_i16.rs) |
| `a - b` on `i16` | Supported | 19 / 30 |  |  | [sub_i16.rs](expressions/sub_i16.rs) |
| `a - b` on `u8` | Supported | 56 / 53 |  |  | [sub_u8.rs](expressions/sub_u8.rs) |
| `a - b` on `u8` below 0: panics, or wraps with checks off | Supported | 56 / 53 |  |  | [sub_u8_underflow.rs](expressions/sub_u8_underflow.rs) |
| `a - b` on `usize` | Supported | 14 / 22 |  |  | [sub_usize.rs](expressions/sub_usize.rs) |
| `a - b` on `usize` below 0: panics, or wraps with checks off | Supported | 17 / 24 |  |  | [sub_usize_underflow.rs](expressions/sub_usize_underflow.rs) |

### Comparison operators

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| `==`, `!=`, `<`, `>` on `bool` | Supported | 68 / 68 |  |  | [compare_bool.rs](expressions/compare_bool.rs) |
| `==`, `!=`, `<`, `>`, `<=`, `>=` on `i16` | Supported | 77 / 77 |  |  | [compare_i16.rs](expressions/compare_i16.rs) |
| `==`, `!=`, `<` on `u8` | Supported | 83 / 83 |  |  | [compare_u8.rs](expressions/compare_u8.rs) |
| `>`, `<=`, `>=` on `u8` | Supported | 76 / 76 |  |  | [compare_u8_2.rs](expressions/compare_u8_2.rs) |
| `==`, `!=`, `<`, `>`, `<=`, `>=` on `usize` | Supported | 76 / 76 |  |  | [compare_usize.rs](expressions/compare_usize.rs) |

### Lazy boolean operators

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| `a && b`, `a \|\| b` | Supported | 39 / 39 |  |  | [and_or.rs](expressions/and_or.rs) |
| `&&` doesn't evaluate its right side when the left side is false | Supported | 75 / 75 |  | The right side divides by zero: evaluating it would panic. | [and_short_circuit.rs](expressions/and_short_circuit.rs) |
| `\|\|` doesn't evaluate its right side when the left side is true | Supported | 68 / 68 |  | The right side divides by zero: evaluating it would panic. | [or_short_circuit.rs](expressions/or_short_circuit.rs) |

### Negation operators

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| `!b` on `bool` | Supported | 16 / 16 |  |  | [not_bool.rs](expressions/not_bool.rs) |
| `!a` on `i16` (bitwise not) | Supported | 22 / 22 |  | On a signed integer `!a` is `-a - 1`, which CARDIAC can do in two instructions. | [not_i16.rs](expressions/not_i16.rs) |
| `!a` on `u8` (bitwise not) | Supported | 32 / 32 |  | On a `u8`, `!a` is `255 - a`. | [not_u8.rs](expressions/not_u8.rs) |

### Bitwise and shift operators

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| `a & m` with `m` one below a power of two (`a & 3`) | Supported | 22 / 22 |  | Compiled as `a.rem_euclid(4)`, a subtracting loop. | [and_mask.rs](expressions/and_mask.rs) |
| `a & b`, `b` a variable | Supported | 94 / 94 |  | Needs the bits of a decimal number: a digit-by-digit routine. | [and_var.rs](expressions/and_var.rs) |
| `a \| b` | Supported | 94 / 94 |  | Needs the bits of a decimal number: a digit-by-digit routine. | [or_var.rs](expressions/or_var.rs) |
| `a << b`, `a >> b`: shifts by a variable | **The test is too big** (it doesn't fit in 96 cells) |  | On demand | The compiler says: `RCE0032: program runs past cell 98` | [shift_var.rs](expressions/shift_var.rs) |
| `a << 2`: a shift left by a constant | Supported | 26 / 26 |  | `a << k` is `a * 2^k`, which the compiler already does for a constant. | [shl_const.rs](expressions/shl_const.rs) |
| `a >> 2`: a shift right by a constant | Supported | 46 / 46 |  | `a >> k` on an `i16` is `a.div_euclid(2^k)`, which the compiler already does for a constant. | [shr_const.rs](expressions/shr_const.rs) |
| `a ^ b` | Supported | 95 / 95 |  | Needs the bits of a decimal number: a digit-by-digit routine. | [xor_var.rs](expressions/xor_var.rs) |

### Assignment and compound assignment

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| `x += a` on `i16` | Supported | 13 / 24 |  |  | [compound_add.rs](expressions/compound_add.rs) |
| `x /= a` on `i16` | Supported | 55 / 55 |  |  | [compound_div.rs](expressions/compound_div.rs) |
| `t[i] += a` on an array element | Supported | 35 / 45 |  |  | [compound_element.rs](expressions/compound_element.rs) |
| `s.f += a` on a struct field | Supported | 14 / 26 |  |  | [compound_field.rs](expressions/compound_field.rs) |
| `x *= a` on `i16` | Supported | 41 / 58 |  |  | [compound_mul.rs](expressions/compound_mul.rs) |
| `x %= a` on `i16` | Supported | 69 / 69 |  |  | [compound_rem.rs](expressions/compound_rem.rs) |
| `x -= a` on `i16` | Supported | 13 / 24 |  |  | [compound_sub.rs](expressions/compound_sub.rs) |
| Destructuring assignment: `(a, b) = (b, a)` | Supported | 22 / 34 |  |  | [destructuring_assign.rs](expressions/destructuring_assign.rs) |

### Type cast expressions

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| `bool as i16` | Supported | 24 / 24 |  |  | [cast_bool_i16.rs](expressions/cast_bool_i16.rs) |
| `t[i] as i16` straight from a `u8` array | Supported | 28 / 28 |  | Copying the element to a local and casting that works. | [cast_element.rs](expressions/cast_element.rs) |
| `i16 as u8` (it wraps) | Supported | 20 / 20 |  | Wrapping is `a.rem_euclid(256)` on a card. | [cast_i16_u8.rs](expressions/cast_i16_u8.rs) |
| `i16 as usize` and `usize as i16` | Supported | 9 / 18 |  |  | [cast_i16_usize.rs](expressions/cast_i16_usize.rs) |
| `u8 as i16` | Supported | 31 / 31 |  |  | [cast_u8_i16.rs](expressions/cast_u8_i16.rs) |

### Grouped expressions

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| Parentheses changing the order: `(a + b) * 3` against `a + b * 3` | Supported | 27 / 50 |  |  | [grouped_parens.rs](expressions/grouped_parens.rs) |

### Array and array index expressions

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| A 2-D grid in one flat array, `t[r * W + c]` | Supported | 64 / 78 |  |  | [array_2d_flat.rs](expressions/array_2d_flat.rs) |
| An array of arrays, `t[r][c]` | Supported | 68 / 78 |  | It is one array, laid out by tens where the program fits: each row ten cells after the one before, and the program's other cells in the holes at the ends of the rows. | [array_2d_nested.rs](expressions/array_2d_nested.rs) |
| `t[i + 1]` and `t[n - i]`: an index computed in place | Supported | 49 / 63 |  |  | [array_index_expr.rs](expressions/array_index_expr.rs) |
| `t[i]` read and written with `i` from a card | Supported | 33 / 33 |  |  | [array_index_var.rs](expressions/array_index_var.rs) |
| `t.len()` | Supported | 8 / 19 |  |  | [array_len.rs](expressions/array_len.rs) |
| An array literal `[a, b, c]` with values read at run time | Supported | 35 / 43 |  |  | [array_literal.rs](expressions/array_literal.rs) |
| An array of structs, `t[i].f` | Supported | 63 / 73 |  |  | [array_of_struct.rs](expressions/array_of_struct.rs) |
| An index past the end panics | Supported | 20 / 20 |  |  | [array_out_of_bounds.rs](expressions/array_out_of_bounds.rs) |
| A repeat expression `[v; N]` | Supported | 37 / 68 |  |  | [array_repeat.rs](expressions/array_repeat.rs) |

### Tuple and tuple indexing expressions

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| A tuple and `.0`, `.1` | Supported | 10 / 21 |  |  | [tuple_index.rs](expressions/tuple_index.rs) |
| A tuple of `i16`, `bool` and `u8` | Supported | 43 / 54 |  |  | [tuple_mixed.rs](expressions/tuple_mixed.rs) |
| A nested tuple, `t.1.0` | Supported | 14 / 25 |  |  | [tuple_nested.rs](expressions/tuple_nested.rs) |
| A tuple returned from a function and taken apart | Supported | 36 / 57 |  |  | [tuple_return.rs](expressions/tuple_return.rs) |
| A tuple struct's fields, `p.0`, `p.1` | Supported | 10 / 21 |  |  | [tuple_struct_index.rs](expressions/tuple_struct_index.rs) |

### Struct expressions

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| An enum variant built from a call's result, `Shape::Rect(k, inp())`, and returned | Supported | 39 / 63 |  | Wrong answer: the field set from the call's result comes back as 0; built from parameters the same enum is right. | [struct_enum_variant_call.rs](expressions/struct_enum_variant_call.rs) |
| A struct literal with named fields | Supported | 15 / 26 |  |  | [struct_named.rs](expressions/struct_named.rs) |
| A struct literal with a struct field | Supported | 25 / 41 |  |  | [struct_nested.rs](expressions/struct_nested.rs) |
| Field init shorthand, `P { x, y }` | Supported | 14 / 14 |  |  | [struct_shorthand.rs](expressions/struct_shorthand.rs) |
| A tuple struct built with a call, `Pair(a, b)` | Supported | 20 / 20 |  |  | [struct_tuple.rs](expressions/struct_tuple.rs) |
| A unit struct and its method | Supported | 13 / 24 |  | A unit struct has no fields, so it needs no cells at all. | [struct_unit.rs](expressions/struct_unit.rs) |
| Functional update, `P { x: 1, ..base }` | Supported | 32 / 32 |  |  | [struct_update.rs](expressions/struct_update.rs) |

### Call expressions

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| An associated function, `P::new(a, b)` | Supported | 22 / 33 |  |  | [call_assoc_fn.rs](expressions/call_assoc_fn.rs) |
| Calling a function that isn't inlined | Supported | 27 / 42 |  |  | [call_fn.rs](expressions/call_fn.rs) |
| Calling through a function pointer chosen at run time | Supported | 47 / 63 |  | A call through a pointer is an indirect jump, which CARDIAC can build like an indexed read. | [call_fn_pointer.rs](expressions/call_fn_pointer.rs) |
| A function pointer passed to a function | Supported | 59 / 81 |  | A call through a pointer is an indirect jump, which CARDIAC can build like an indexed read. | [call_fn_pointer_param.rs](expressions/call_fn_pointer_param.rs) |
| A generic function, called with two types | Supported | 36 / 36 |  | Generic functions are compiled only when rustc inlines them; each instance would need its own copy. | [call_generic.rs](expressions/call_generic.rs) |
| Nested calls, `f(g(a), g(b))` | Supported | 34 / 50 |  |  | [call_nested.rs](expressions/call_nested.rs) |

### Method-call expressions

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| Auto-ref and auto-deref: a `&self` method called on a value, on `&P` and on `&&P` | Supported | 32 / 43 |  |  | [method_autoref.rs](expressions/method_autoref.rs) |
| Chained methods, `p.moved(1).moved(2).sum()` | Supported | 38 / 73 |  |  | [method_chained.rs](expressions/method_chained.rs) |
| A trait object, `&dyn Trait`, chosen at run time | Supported | 45 / 56 |  | Dynamic dispatch needs a vtable and an indirect call, which CARDIAC can build as it builds an indexed read. | [method_dyn.rs](expressions/method_dyn.rs) |
| An inherent method taking `&self` | Supported | 15 / 26 |  |  | [method_inherent.rs](expressions/method_inherent.rs) |
| A method taking `&mut self` | Supported | 30 / 42 |  |  | [method_mut_self.rs](expressions/method_mut_self.rs) |
| A trait method, two types implementing it | Supported | 56 / 77 |  |  | [method_trait.rs](expressions/method_trait.rs) |
| A trait's default method calling a required one | Supported | 9 / 20 |  | At opt-level 1 rustc calls the default method with the struct by reference, which inherent methods already handle. | [method_trait_default.rs](expressions/method_trait_default.rs) |

### Field access expressions

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| An array field, `s.t[i]` | Supported | 37 / 46 |  |  | [field_array.rs](expressions/field_array.rs) |
| Nested field access, `a.b.c`, read and written | Supported | 12 / 25 |  |  | [field_nested.rs](expressions/field_nested.rs) |
| A field read through `&mut` without `*`, `r.f` | Supported | 65 / 80 |  |  | [field_through_ref.rs](expressions/field_through_ref.rs) |

### Closure expressions

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| A closure changing a captured variable (`FnMut`) | Supported | 13 / 24 |  | At opt-level 1 rustc calls the closure rather than inlining it; the call is a method on a struct of its captures. | [closure_capture_mut.rs](expressions/closure_capture_mut.rs) |
| A closure capturing a variable by reference | Supported | 15 / 36 |  | At opt-level 1 rustc calls the closure rather than inlining it; the call is a method on a struct of its captures. | [closure_capture_ref.rs](expressions/closure_capture_ref.rs) |
| A closure called where it's made | Supported | 13 / 26 |  | At opt-level 1 rustc calls the closure rather than inlining it; the call is a method on a struct of its captures. | [closure_local.rs](expressions/closure_local.rs) |
| A `move` closure | Supported | 9 / 20 |  | At opt-level 1 rustc calls the closure rather than inlining it; the call is a method on a struct of its captures. | [closure_move.rs](expressions/closure_move.rs) |
| A closure that captures nothing, coerced to a `fn` pointer | Supported | 47 / 63 |  | A call through a pointer is an indirect jump, which CARDIAC can build like an indexed read. | [closure_no_capture_fn_ptr.rs](expressions/closure_no_capture_fn_ptr.rs) |
| A closure passed to a generic function | Supported | 39 / 50 |  | Each closure is its own type, so the function it's passed to is generic, compiled once per closure. | [closure_passed.rs](expressions/closure_passed.rs) |
| A closure passed as `&dyn Fn` | Supported | 42 / 53 |  | Dynamic dispatch needs a vtable and an indirect call, which CARDIAC can build as it builds an indexed read. | [closure_passed_dyn.rs](expressions/closure_passed_dyn.rs) |
| A closure returned from a function (`impl Fn`) | Supported | 21 / 34 |  | Each closure is its own type, so the function returning one is generic, compiled once per closure. | [closure_returned.rs](expressions/closure_returned.rs) |
| A closure stored in a struct field and called from there | Supported | 22 / 37 |  | Each closure is its own type, so the struct holding one is generic, compiled once per closure. | [closure_stored.rs](expressions/closure_stored.rs) |

### Range expressions

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| `(a..b).contains(&x)` | Supported | 42 / 42 |  |  | [range_contains.rs](expressions/range_contains.rs) |

### Loop expressions

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| `continue` in a `while` loop | Supported | 20 / 20 |  |  | [continue_while.rs](expressions/continue_while.rs) |
| `for x in array` | Supported | 29 / 29 |  | The same as `for i in 0..N` with `t[i]`. | [for_array.rs](expressions/for_array.rs) |
| `for x in &array` and `array.iter()` | Supported | 45 / 57 |  | The same as `for i in 0..N` with `t[i]`. | [for_array_ref.rs](expressions/for_array_ref.rs) |
| `for (i, x) in array.iter().enumerate()` | Supported | 32 / 47 |  |  | [for_enumerate.rs](expressions/for_enumerate.rs) |
| `for x in array.iter_mut()` | Supported | 37 / 49 |  |  | [for_iter_mut.rs](expressions/for_iter_mut.rs) |
| Nested `for` loops | Supported | 29 / 54 |  |  | [for_nested.rs](expressions/for_nested.rs) |
| `for i in a..b` | Supported | 13 / 13 |  |  | [for_range.rs](expressions/for_range.rs) |
| `for i in a..b` with `a >= b`: no passes | Supported | 20 / 20 |  |  | [for_range_empty.rs](expressions/for_range_empty.rs) |
| `for i in a..=b` | Supported | 13 / 13 |  |  | [for_range_inclusive.rs](expressions/for_range_inclusive.rs) |
| `for i in a..b` with negative bounds | Supported | 13 / 13 |  |  | [for_range_negative.rs](expressions/for_range_negative.rs) |
| `for i in (a..b).rev()` | Supported | 13 / 13 |  |  | [for_rev.rs](expressions/for_rev.rs) |
| `for i in (a..b).step_by(k)` | Supported | 56 / 56 |  | A `while` loop that adds `k` does the same. | [for_step_by.rs](expressions/for_step_by.rs) |
| `for i in 0..n` over `usize`, indexing an array | Supported | 32 / 32 |  |  | [for_usize.rs](expressions/for_usize.rs) |
| `array.iter().map(...).sum()` | Supported | 44 / 48 |  |  | [iter_map_sum.rs](expressions/iter_map_sum.rs) |
| A labelled block, `'a: { ... break 'a v; ... }` | Supported | 17 / 28 |  |  | [labelled_block.rs](expressions/labelled_block.rs) |
| Labelled `break 'outer` and `continue 'outer` | Supported | 43 / 74 |  |  | [labelled_break_continue.rs](expressions/labelled_break_continue.rs) |
| `loop` with `break` | Supported | 9 / 21 |  |  | [loop_break.rs](expressions/loop_break.rs) |
| `loop` with `break value` | Supported | 11 / 11 |  |  | [loop_break_value.rs](expressions/loop_break_value.rs) |
| `(a..b).sum()` | Supported | 28 / 28 |  |  | [range_sum.rs](expressions/range_sum.rs) |
| `while` | Supported | 9 / 9 |  |  | [while.rs](expressions/while.rs) |
| `while let Some(x) = ...` | Supported | 20 / 31 |  |  | [while_let.rs](expressions/while_let.rs) |

### If and if let expressions

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| An `if` / `else if` / `else` chain | Supported | 39 / 39 |  |  | [if_else_chain.rs](expressions/if_else_chain.rs) |
| A let chain, `if let ... && ...` | Supported | 84 / 84 |  | Let chains are Rust 2024's: the row is built in that edition, natively too. | [if_let_chain.rs](expressions/if_let_chain.rs) |
| `if let` on an enum variant with fields | Supported | 51 / 62 |  |  | [if_let_enum.rs](expressions/if_let_enum.rs) |
| `if let Some(x) = ... else` | Supported | 29 / 40 |  |  | [if_let_option.rs](expressions/if_let_option.rs) |
| `if` as a value, `let x = if c { a } else { b };` | Supported | 16 / 16 |  |  | [if_value.rs](expressions/if_value.rs) |
| `let Some(x) = ... else { ... };` | Supported | 37 / 48 |  |  | [let_else.rs](expressions/let_else.rs) |

### Match expressions

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| `match` on an array pattern, `[a, 0, _]` | Supported | 20 / 31 |  |  | [match_array.rs](expressions/match_array.rs) |
| `match` binding with `@`, `n @ 1..=9` | Supported | 23 / 34 |  |  | [match_binding.rs](expressions/match_binding.rs) |
| `match` on a `bool` | Supported | 20 / 20 |  |  | [match_bool.rs](expressions/match_bool.rs) |
| `match` on an enum with fields | Supported | 66 / 93 |  |  | [match_enum.rs](expressions/match_enum.rs) |
| `match` arms with guards, `x if x > b` | Supported | 27 / 27 |  |  | [match_guard.rs](expressions/match_guard.rs) |
| `match` on an `i16` with literal arms and `_` | Supported | 33 / 33 |  |  | [match_i16.rs](expressions/match_i16.rs) |
| `match` on a nested pattern, `Some(Shape::Sq(a))` | Supported | 63 / 63 |  |  | [match_nested.rs](expressions/match_nested.rs) |
| `match` on an `Option` | Supported | 24 / 24 |  |  | [match_option.rs](expressions/match_option.rs) |
| `match` with `\|` alternatives | Supported | 32 / 32 |  |  | [match_or.rs](expressions/match_or.rs) |
| `match` with range patterns, `a..=b` and `..0` | Supported | 39 / 39 |  |  | [match_ranges.rs](expressions/match_ranges.rs) |
| `match` destructuring a struct, `P { x: 0, y }` | Supported | 19 / 44 |  |  | [match_struct.rs](expressions/match_struct.rs) |
| `match` on a tuple of two values | Supported | 37 / 37 |  |  | [match_tuple.rs](expressions/match_tuple.rs) |

### Return expressions

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| An early `return` from a function | Supported | 24 / 35 |  |  | [return_early.rs](expressions/return_early.rs) |
| `return` from inside a loop | Supported | 62 / 62 |  |  | [return_in_loop.rs](expressions/return_in_loop.rs) |
| A bare `return;` from a function that returns nothing | Supported | 17 / 17 |  |  | [return_unit.rs](expressions/return_unit.rs) |

### Underscore expressions

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| `_ = f()`: calling and dropping the result | Supported | 19 / 34 |  |  | [underscore_assign.rs](expressions/underscore_assign.rs) |
| `_` in destructuring assignment, `(a, _) = (x, y)` | Supported | 10 / 21 |  |  | [underscore_destructure.rs](expressions/underscore_destructure.rs) |

## Patterns

### Destructuring

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| `fn f(Point { x, y }: Point)` | Supported | 20 / 37 |  |  | [fn_param_struct.rs](patterns/fn_param_struct.rs) |
| `fn f((a, b): (i16, i16))` | Supported | 20 / 37 |  |  | [fn_param_tuple.rs](patterns/fn_param_tuple.rs) |
| `let Some(x) = o else { ... };` | Supported | 29 / 40 |  |  | [let_else.rs](patterns/let_else.rs) |
| `let Point { x, y } = p;` | Supported | 26 / 37 |  |  | [let_struct.rs](patterns/let_struct.rs) |
| `let (a, b) = (x, y);` | Supported | 15 / 26 |  |  | [let_tuple.rs](patterns/let_tuple.rs) |
| `while let Some(x) = ...` | Supported | 33 / 44 |  |  | [while_let.rs](patterns/while_let.rs) |

### Refutability

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| `let Some(x) = o;` is refused by rustc (refutable pattern in `let`) | Refused |  | Never | rustc itself rejects a refutable pattern in a plain `let` (E0005), so this builds nowhere; `let else` and `if let` are the ways round it. This row shows Rust's own rule: rustc rejects a refutable pattern in a plain `let` (E0005), so it builds nowhere; `let else` and `if let` are the ways round it. The compiler says: `E0005: refutable pattern in local binding` | [refutable_let.rs](patterns/refutable_let.rs) |

### Literal patterns

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| `true` and `false` patterns | Supported | 18 / 18 |  |  | [lit_bool.rs](patterns/lit_bool.rs) |
| Byte literals (`b'A' =>`) on a `u8`, read from a `u8` table | Supported | 45 / 45 |  |  | [lit_byte.rs](patterns/lit_byte.rs) |
| `char` literals (`'a' =>`) | Supported | 36 / 36 |  | A `char` can hold up to 1114111, far past a card, but every ASCII and Latin-1 character fits: like an `i16`, it would work where the value fits. | [lit_char.rs](patterns/lit_char.rs) |
| A literal in `if let` (`if let 3 = v`) | Supported | 18 / 18 |  |  | [lit_if_let.rs](patterns/lit_if_let.rs) |
| Integer literals in a `match` | Supported | 30 / 30 |  |  | [lit_int.rs](patterns/lit_int.rs) |
| Negative integer literals (`-5 =>`) | Supported | 31 / 31 |  |  | [lit_negative.rs](patterns/lit_negative.rs) |

### Identifier patterns

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| `x @ 1..=5` | Supported | 24 / 24 |  |  | [id_at_range.rs](patterns/id_at_range.rs) |
| `x @ Some(_)` | Supported | 29 / 40 |  |  | [id_at_some.rs](patterns/id_at_some.rs) |
| `mut x` binding | Supported | 13 / 24 |  |  | [id_mut.rs](patterns/id_mut.rs) |
| `ref x` binding | Supported | 10 / 21 |  |  | [id_ref.rs](patterns/id_ref.rs) |
| `ref mut x` binding | Supported | 11 / 22 |  |  | [id_ref_mut.rs](patterns/id_ref_mut.rs) |

### Binding modes

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| Match ergonomics through `&mut`: `if let Some(x) = o` with `o: &mut Option<i16>` | Supported | 63 / 75 |  |  | [ergonomics_mut.rs](patterns/ergonomics_mut.rs) |
| Match ergonomics: `&Option<i16>` matched with `Some(x)` | Supported | 30 / 30 |  |  | [ergonomics_ref_option.rs](patterns/ergonomics_ref_option.rs) |
| Match ergonomics: `&Point` matched with `Point { x, y }` | Supported | 20 / 31 |  |  | [ergonomics_ref_struct.rs](patterns/ergonomics_ref_struct.rs) |
| Match ergonomics: `&(i16, i16)` matched with `(a, b)` | Supported | 20 / 31 |  |  | [ergonomics_ref_tuple.rs](patterns/ergonomics_ref_tuple.rs) |

### Wildcard pattern

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| `_` as the last arm | Supported | 20 / 20 |  |  | [wildcard_arm.rs](patterns/wildcard_arm.rs) |
| `_` inside a tuple (`(_, b)`) | Supported | 10 / 10 |  |  | [wildcard_tuple.rs](patterns/wildcard_tuple.rs) |

### Rest patterns

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| `..` in a struct pattern | Supported | 12 / 23 |  |  | [rest_struct.rs](patterns/rest_struct.rs) |
| `..` in a tuple (`(a, .., c)`) | Supported | 9 / 9 |  |  | [rest_tuple.rs](patterns/rest_tuple.rs) |
| `..` in a tuple struct pattern (`T(a, ..)`) | Supported | 8 / 8 |  |  | [rest_tuple_struct.rs](patterns/rest_tuple_struct.rs) |

### Range patterns

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| Consts as range bounds (`LO..=HI`) | Supported | 22 / 22 |  |  | [range_const_bounds.rs](patterns/range_const_bounds.rs) |
| Exclusive `a..b` in a `match` | Supported | 20 / 20 |  |  | [range_exclusive.rs](patterns/range_exclusive.rs) |
| Half-open `a..` in a `match` | Supported | 25 / 25 |  |  | [range_from.rs](patterns/range_from.rs) |
| `a..=b` in a `match` | Supported | 32 / 32 |  |  | [range_inclusive.rs](patterns/range_inclusive.rs) |
| `..=b` in a `match` | Supported | 24 / 24 |  |  | [range_to_inclusive.rs](patterns/range_to_inclusive.rs) |
| Ranges on a `u8` (`b'0'..=b'9'`) | Supported | 50 / 50 |  |  | [range_u8.rs](patterns/range_u8.rs) |

### Reference patterns

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| `&x` and `&(a, b)` in a `match` | Supported | 38 / 55 |  |  | [ref_pat_match.rs](patterns/ref_pat_match.rs) |
| `&x` in a parameter (`fn f(&x: &i16)`) | Supported | 18 / 29 |  |  | [ref_pat_param.rs](patterns/ref_pat_param.rs) |

### Struct patterns

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| Struct-like enum variant (`Shape::Rect { w, h }`) | Supported | 49 / 61 |  |  | [struct_enum_variant.rs](patterns/struct_enum_variant.rs) |
| A literal in a field (`P { x: 0, y }`) in a `match` | Supported | 48 / 48 |  |  | [struct_literal_field.rs](patterns/struct_literal_field.rs) |
| Struct pattern with named fields (`P { x: a, y: b }`) | Supported | 10 / 35 |  |  | [struct_named.rs](patterns/struct_named.rs) |
| Nested struct patterns | Supported | 28 / 39 |  |  | [struct_nested.rs](patterns/struct_nested.rs) |

### Tuple struct patterns

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| Tuple struct pattern (`Pair(a, b)`) | Supported | 9 / 20 |  |  | [tuple_struct.rs](patterns/tuple_struct.rs) |
| Tuple variant patterns (`Shape::Rect(w, h)`) | Supported | 61 / 73 |  | A wrong answer, and not from the pattern: an enum value built in an `#[inline(never)]` function from `inp()` can lose a field (`fn make() -> Option<i16> { Some(inp()) }` gives `Some(0)` for a card of 4); built from parameters it is right. | [tuple_struct_enum.rs](patterns/tuple_struct_enum.rs) |
| A literal inside a variant (`Some(0)`) | Supported | 32 / 32 |  |  | [tuple_struct_literal.rs](patterns/tuple_struct_literal.rs) |
| Nested tuple-struct patterns (`Some(Ok(x))`) | Supported | 57 / 79 |  |  | [tuple_struct_nested_option.rs](patterns/tuple_struct_nested_option.rs) |

### Tuple patterns

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| Tuple of `bool`s (`(true, false) =>`) | Supported | 42 / 42 |  |  | [tuple_bool.rs](patterns/tuple_bool.rs) |
| Tuple pattern in a `match` (`(0, y)`) | Supported | 40 / 61 |  |  | [tuple_match.rs](patterns/tuple_match.rs) |
| Nested tuple pattern (`((a, b), c)`) | Supported | 17 / 52 |  |  | [tuple_nested.rs](patterns/tuple_nested.rs) |

### Grouped patterns

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| Grouped pattern (`&(0..=5)`) | Supported | 20 / 20 |  |  | [grouped.rs](patterns/grouped.rs) |

### Slice patterns

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| Array pattern (`let [a, b, c] = t;`) | Supported | 17 / 52 |  |  | [slice_array_all.rs](patterns/slice_array_all.rs) |
| `[first, .., last]` on an array | Supported | 9 / 9 |  |  | [slice_first_last.rs](patterns/slice_first_last.rs) |
| Array pattern with literals in a `match` (`[0, _, x]`) | Supported | 28 / 28 |  |  | [slice_match_literal.rs](patterns/slice_match_literal.rs) |
| Slice pattern on a slice (`&[i16]`: `[a, ..]`, `[]`) | Supported | 49 / 49 |  |  | [slice_on_slice.rs](patterns/slice_on_slice.rs) |
| Array pattern through `&[i16; 3]` | Supported | 24 / 24 |  |  | [slice_ref_array.rs](patterns/slice_ref_array.rs) |
| `[x, rest @ ..]` on an array | Supported | 36 / 57 |  |  | [slice_rest_binding.rs](patterns/slice_rest_binding.rs) |

### Path patterns

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| An associated const as a pattern (`Limits::MAX =>`) | Supported | 21 / 21 |  |  | [path_assoc_const.rs](patterns/path_assoc_const.rs) |
| A `const` as a pattern | Supported | 21 / 21 |  |  | [path_const.rs](patterns/path_const.rs) |
| Unit enum variants (`Colour::Red =>`) | Supported | 48 / 48 |  |  | [path_enum_unit.rs](patterns/path_enum_unit.rs) |

### Or-patterns

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| Or-pattern binding the same name (`(x, 0) \| (0, x)`) | Supported | 25 / 25 |  |  | [or_binding.rs](patterns/or_binding.rs) |
| `1 \| 2 =>` | Supported | 30 / 30 |  |  | [or_ints.rs](patterns/or_ints.rs) |
| Or-pattern in `let` (`let (Ok(x) \| Err(x)) = r;`) | Supported | 34 / 46 |  |  | [or_let.rs](patterns/or_let.rs) |
| Nested or-pattern (`Some(1 \| 2)`) | Supported | 41 / 41 |  |  | [or_nested.rs](patterns/or_nested.rs) |

## Statements

### Let statements

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| `let x = { ...; e };` with a block as initialiser | Supported | 10 / 25 |  |  | [let_block_init.rs](statements/let_block_init.rs) |
| `let x;` without an initialiser, assigned later | Supported | 9 / 20 |  |  | [let_deferred.rs](statements/let_deferred.rs) |
| `let x;` assigned in each branch of an `if` | Supported | 24 / 35 |  |  | [let_deferred_branch.rs](statements/let_deferred_branch.rs) |
| `let x;` assigned in each arm of a `match` | Supported | 24 / 24 |  |  | [let_deferred_match.rs](statements/let_deferred_match.rs) |
| `let x = e;` with the type inferred | Supported | 9 / 20 |  |  | [let_inferred.rs](statements/let_inferred.rs) |
| `let mut x = 0;` whose type comes from a later use | Supported | 16 / 27 |  |  | [let_inferred_later.rs](statements/let_inferred_later.rs) |
| `let mut x = e;` and later assignments | Supported | 13 / 24 |  |  | [let_mut_reassign.rs](statements/let_mut_reassign.rs) |
| shadowing: `let a = a + 1;` | Supported | 11 / 35 |  |  | [let_shadow.rs](statements/let_shadow.rs) |
| shadowing in an inner block ends with the block | Supported | 7 / 7 |  |  | [let_shadow_scope.rs](statements/let_shadow_scope.rs) |
| shadowing with a new type (`i16` to `bool` to `u8`) | Supported | 18 / 18 |  |  | [let_shadow_type.rs](statements/let_shadow_type.rs) |
| `let (a, b) = (e1, e2);` | Supported | 10 / 21 |  |  | [let_tuple.rs](statements/let_tuple.rs) |
| `let x: i16 = e;` with a type | Supported | 9 / 20 |  |  | [let_typed.rs](statements/let_typed.rs) |
| `let _ = e;` evaluates `e` and drops it | Supported | 5 / 5 |  |  | [let_underscore.rs](statements/let_underscore.rs) |
| `let _x = e;` (an unused binding) | Supported | 5 / 5 |  |  | [let_underscore_named.rs](statements/let_underscore_named.rs) |

### Let-else

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| `let Some(v) = e else { ... };` (let-else) | Supported | 18 / 29 |  |  | [let_else.rs](statements/let_else.rs) |
| let-else whose `else` panics | Supported | 15 / 15 |  |  | [let_else_panic.rs](statements/let_else_panic.rs) |
| let-else with a refutable range pattern | Supported | 22 / 22 |  |  | [let_else_range.rs](statements/let_else_range.rs) |

### Item declarations

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| a `const` declared inside a function | Supported | 8 / 19 |  |  | [item_const.rs](statements/item_const.rs) |
| an `enum` declared inside a function | Supported | 14 / 14 |  |  | [item_enum.rs](statements/item_enum.rs) |
| a `fn` declared inside a function | Supported | 12 / 25 |  |  | [item_fn.rs](statements/item_fn.rs) |
| a `static` declared inside a function | Supported | 30 / 30 |  |  | [item_static.rs](statements/item_static.rs) |
| a `struct` with an `impl` declared inside a function | Supported | 9 / 20 |  |  | [item_struct.rs](statements/item_struct.rs) |
| a `use` declaration inside a function | Supported | 11 / 11 |  |  | [item_use.rs](statements/item_use.rs) |

### Expression statements

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| `if` and `match` as statements without a semicolon | Supported | 26 / 26 |  |  | [expr_stmt_block.rs](statements/expr_stmt_block.rs) |
| a call as a statement, its result unused | Supported | 6 / 6 |  |  | [expr_stmt_call.rs](statements/expr_stmt_call.rs) |
| a block whose value is its tail expression | Supported | 9 / 23 |  |  | [expr_stmt_tail.rs](statements/expr_stmt_tail.rs) |
| an unused arithmetic result (`a + b;`) | Supported | 9 / 20 |  |  | [expr_stmt_unused.rs](statements/expr_stmt_unused.rs) |

## Items

### Modules

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| An inline `mod` with a `pub fn` | Supported | 12 / 25 |  |  | [mod_inline.rs](items/mod_inline.rs) |
| Nested modules and a full path (`crate::a::b::f`) | Supported | 8 / 19 |  |  | [mod_nested.rs](items/mod_nested.rs) |
| A struct with a private field, built by a module's `pub fn new` | Supported | 9 / 22 |  |  | [mod_private_field.rs](items/mod_private_field.rs) |
| A `pub struct` with `pub` fields from a module | Supported | 10 / 21 |  |  | [mod_pub_struct.rs](items/mod_pub_struct.rs) |
| `super::` and `self::` paths | Supported | 8 / 23 |  |  | [mod_super.rs](items/mod_super.rs) |

### Use declarations

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| `use Enum::*` to bring variants into scope | Supported | 30 / 30 |  |  | [use_enum_variants.rs](items/use_enum_variants.rs) |
| `use m::*` (glob import) | Supported | 19 / 34 |  |  | [use_glob.rs](items/use_glob.rs) |
| `use m::{self, f}` (nested groups with `self`) | Supported | 16 / 27 |  |  | [use_group_self.rs](items/use_group_self.rs) |
| `pub use` to re-export | Supported | 34 / 51 |  |  | [use_pub_reexport.rs](items/use_pub_reexport.rs) |
| `use ... as` to rename | Supported | 8 / 8 |  |  | [use_rename.rs](items/use_rename.rs) |
| `use` of a function from a module | Supported | 8 / 8 |  |  | [use_simple.rs](items/use_simple.rs) |

### Functions

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| A non-inlined function calling another | Supported | 21 / 32 |  |  | [fn_calls_fn.rs](items/fn_calls_fn.rs) |
| A `const fn` called with a value read at run time | Supported | 16 / 37 |  |  | [fn_const_fn_runtime.rs](items/fn_const_fn_runtime.rs) |
| A const generic parameter (`fn f<const N: usize>`) | Supported | 31 / 50 |  | rustc writes the MIR of each instance (`f::<i16>`); compiling it as an ordinary function is what the compiler already does for non-generic ones. | [fn_const_generic.rs](items/fn_const_generic.rs) |
| A const generic array length (`fn sum<const N: usize>(t: &[i16; N])`) | Supported | 50 / 86 |  | rustc writes the MIR of each instance (`f::<i16>`); compiling it as an ordinary function is what the compiler already does for non-generic ones. | [fn_const_generic_array.rs](items/fn_const_generic_array.rs) |
| A diverging function (`-> !`) | Supported | 19 / 19 |  | A call that never returns needs nothing after it; the second card is negative, so the program then panics. | [fn_diverging.rs](items/fn_diverging.rs) |
| A generic function rustc inlines | Supported | 63 / 63 |  | rustc writes the MIR of each instance (`f::<i16>`); compiling it as an ordinary function is what the compiler already does for non-generic ones. | [fn_generic_inlined.rs](items/fn_generic_inlined.rs) |
| A generic function marked `#[inline(never)]` | Supported | 63 / 63 |  | rustc writes the MIR of each instance (`f::<i16>`); compiling it as an ordinary function is what the compiler already does for non-generic ones. | [fn_generic_not_inlined.rs](items/fn_generic_not_inlined.rs) |
| `impl Trait` in argument position | Supported | 68 / 89 |  | rustc writes the MIR of each instance (`f::<i16>`); compiling it as an ordinary function is what the compiler already does for non-generic ones. | [fn_impl_trait_arg.rs](items/fn_impl_trait_arg.rs) |
| `impl Trait` in return position | Supported | 14 / 25 |  |  | [fn_impl_trait_return.rs](items/fn_impl_trait_return.rs) |
| `#[inline(always)]` on a helper | Supported | 11 / 25 |  |  | [fn_inline_always.rs](items/fn_inline_always.rs) |
| A small helper rustc inlines, called twice | Supported | 13 / 26 |  |  | [fn_inlined.rs](items/fn_inlined.rs) |
| A function of six parameters, not inlined | Supported | 27 / 38 |  |  | [fn_many_params.rs](items/fn_many_params.rs) |
| A function defined inside another | Supported | 20 / 45 |  |  | [fn_nested.rs](items/fn_nested.rs) |
| A function with no result (`-> ()`) that prints | Supported | 14 / 14 |  |  | [fn_no_return.rs](items/fn_no_return.rs) |
| A function taking and returning `bool`, not inlined | Supported | 47 / 47 |  |  | [fn_param_bool.rs](items/fn_param_bool.rs) |
| A function taking and returning `i16`, not inlined | Supported | 26 / 37 |  |  | [fn_param_i16.rs](items/fn_param_i16.rs) |
| A function taking and returning `u8`, not inlined | Supported | 49 / 46 |  |  | [fn_param_u8.rs](items/fn_param_u8.rs) |
| A function taking and returning `usize`, not inlined | Supported | 27 / 34 |  |  | [fn_param_usize.rs](items/fn_param_usize.rs) |
| A function pointer (`fn(i16) -> i16`) chosen at run time | Supported | 42 / 53 |  | A call through a cell holding an address can be built with self-modifying code, as variable indexing is. | [fn_pointer.rs](items/fn_pointer.rs) |
| Direct recursion (`fn sum(n) { n + sum(n - 1) }`) | Supported | 96 / 96 |  | CARDIAC has no stack and each function's locals and return address are fixed cells, so a recursive call needs a software stack of its own (an array and self-modifying loads); possible, but slow and costly in cells, and a loop does the same. | [fn_recursion_direct.rs](items/fn_recursion_direct.rs) |
| Mutual recursion (`is_even` and `is_odd`) | Supported | 36 / 49 |  | As for direct recursion: it needs a software stack, since each function's cells are fixed. | [fn_recursion_mutual.rs](items/fn_recursion_mutual.rs) |
| Tail recursion (`fn count(n, acc)`) | Supported | 28 / 42 |  | A call in tail position needs no stack: the compiler could turn it into a jump back to the start, as a loop. | [fn_recursion_tail.rs](items/fn_recursion_tail.rs) |
| A tuple passed as a parameter | Supported | 15 / 26 |  |  | [fn_tuple_param.rs](items/fn_tuple_param.rs) |
| Several results returned as a tuple | Supported | 87 / 87 |  |  | [fn_tuple_return.rs](items/fn_tuple_return.rs) |
| A `where` clause on a generic function | Supported | 14 / 27 |  | rustc writes the MIR of each instance (`f::<i16>`); compiling it as an ordinary function is what the compiler already does for non-generic ones. | [fn_where_clause.rs](items/fn_where_clause.rs) |

### Type aliases

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| A type alias for an array and a tuple | Supported | 35 / 46 |  |  | [type_alias_array.rs](items/type_alias_array.rs) |
| A type alias for a scalar (`type Card = i16`) | Supported | 7 / 18 |  |  | [type_alias_scalar.rs](items/type_alias_scalar.rs) |

### Structs

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| An array of structs | Supported | 43 / 64 |  |  | [struct_array_of.rs](items/struct_array_of.rs) |
| A generic struct (`struct Pair<T>`) at two types | Supported | 31 / 41 |  |  | [struct_generic.rs](items/struct_generic.rs) |
| A struct with named fields, passed by value | Supported | 35 / 48 |  |  | [struct_named.rs](items/struct_named.rs) |
| A struct inside a struct | Supported | 25 / 41 |  |  | [struct_nested.rs](items/struct_nested.rs) |
| A newtype struct (`struct Cents(i16)`) | Supported | 15 / 26 |  |  | [struct_newtype.rs](items/struct_newtype.rs) |
| A struct with an array field | Supported | 39 / 39 |  |  | [struct_of_array.rs](items/struct_of_array.rs) |
| A tuple struct | Supported | 20 / 20 |  |  | [struct_tuple.rs](items/struct_tuple.rs) |
| A unit struct with a method | Supported | 7 / 18 |  | A unit struct holds nothing, so it needs no cells. | [struct_unit.rs](items/struct_unit.rs) |
| Struct update syntax (`S { x: 1, ..s }`) | Supported | 10 / 21 |  |  | [struct_update.rs](items/struct_update.rs) |

### Enumerations

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| A C-like enum with given discriminants, cast with `as` | Supported | 26 / 26 |  |  | [enum_clike.rs](items/enum_clike.rs) |
| An enum whose variants carry data, built from `inp()` in a function that returns it | Supported | 63 / 80 |  | A wrong answer: `Shape::Rect(inp(), inp())` returned from a function loses its second field (prints 3 for 3 - 4); with the fields read into locals first it is right. Even `fn read() -> Shape { Shape::Rect(inp(), inp()) }` alone gives 0, 0. | [enum_data.rs](items/enum_data.rs) |
| An enum whose variants carry data, built from locals and passed to a function | Supported | 64 / 84 |  |  | [enum_data_locals.rs](items/enum_data_locals.rs) |
| `==` on an enum with `#[derive(PartialEq)]` | Supported | 72 / 72 |  |  | [enum_derive_partialeq.rs](items/enum_derive_partialeq.rs) |
| A generic enum of your own (`enum Maybe<T>`) | Supported | 37 / 37 |  | Once rustc fills in `T`, the layout is the same as a non-generic enum's, which already works. | [enum_generic.rs](items/enum_generic.rs) |
| A recursive enum through `Box` | Refused |  | Never | `Box` needs a heap; this row uses a `&'static` link instead, the nearest a `no_std` program gets without an allocator. The compiler says: `RCE0004: enum main::List: field 1 of Cons is &main::List: the fields of an enum's variants must be scalars (i16, u8, usize, bool), enums, structs or arrays of scalars for now` | [enum_recursive_box.rs](items/enum_recursive_box.rs) |
| A `#[repr(i16)]` C-like enum | Supported | 58 / 70 |  |  | [enum_repr_i16.rs](items/enum_repr_i16.rs) |
| An enum with a struct-like variant (`V { x, y }`) | Supported | 31 / 31 |  | A variant with named fields is laid out as one with unnamed fields, which already works. | [enum_struct_variant.rs](items/enum_struct_variant.rs) |

### Unions

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| A `union` read through the field it was written by | Supported | 4 / 4 |  | A union's point is reading one type's bits as another, and CARDIAC's cells hold decimal numbers, not bits. | [union_basic.rs](items/union_basic.rs) |

### Constant items

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| A `const` array indexed at run time | Supported | 36 / 36 |  |  | [const_array.rs](items/const_array.rs) |
| A `const` computed from other constants | Supported | 22 / 22 |  | rustc has already worked out the array; the compiler only has to read its value, as it does for a literal table. | [const_computed.rs](items/const_computed.rs) |
| A `const` from a `const fn` | Supported | 22 / 22 |  | rustc has already run the `const fn`; the compiler only has to read the array it made. | [const_from_const_fn.rs](items/const_from_const_fn.rs) |
| A `const` past a card (`const BIG: i16 = 1500`) | Supported | 8 / 19 |  | Only `BIG / 10` (150) is used, and rustc works it out before the compiler sees it; `BIG` used at run time, as in `BIG / inp()`, is refused on purpose (RCE0009). | [const_past_card.rs](items/const_past_card.rs) |
| A `const` scalar | Supported | 8 / 19 |  |  | [const_scalar.rs](items/const_scalar.rs) |
| A `const` struct | Supported | 17 / 17 |  |  | [const_struct.rs](items/const_struct.rs) |

### Static items

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| A `static` array indexed at run time | Supported | 37 / 37 |  | An immutable `static` is a table in fixed cells, as a `const` array already is. | [static_array.rs](items/static_array.rs) |
| A `static mut` counter | Supported | 38 / 51 |  | A `static mut` is just a fixed cell, which is what every CARDIAC variable already is. | [static_mut.rs](items/static_mut.rs) |
| A `static` scalar | Supported | 8 / 19 |  |  | [static_scalar.rs](items/static_scalar.rs) |

### Traits

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| An associated `const` in a trait | Supported | 17 / 30 |  | rustc writes the MIR of each instance (`f::<i16>`); compiling it as an ordinary function is what the compiler already does for non-generic ones. | [trait_assoc_const.rs](items/trait_assoc_const.rs) |
| An associated type in a trait | Supported | 11 / 11 |  |  | [trait_assoc_type.rs](items/trait_assoc_type.rs) |
| A blanket impl (`impl<T: A> B for T`) | Supported | 7 / 18 |  |  | [trait_blanket_impl.rs](items/trait_blanket_impl.rs) |
| A trait's default method | Supported | 9 / 20 |  |  | [trait_default_method.rs](items/trait_default_method.rs) |
| `#[derive(Clone, Copy)]` and `.clone()` | Supported | 9 / 21 |  |  | [trait_derive_clone_copy.rs](items/trait_derive_clone_copy.rs) |
| Dynamic dispatch: `&dyn Trait` chosen at run time | Supported | 53 / 64 |  | A `&dyn` is a pointer and a table of functions: a call through it is a computed jump, which self-modifying code can do. | [trait_dyn_dispatch.rs](items/trait_dyn_dispatch.rs) |
| A `&dyn Trait` with one implementation | Supported | 23 / 34 |  | A `&dyn` is a pointer and a table of functions: a call through it is a computed jump, which self-modifying code can do. | [trait_dyn_single.rs](items/trait_dyn_single.rs) |
| A generic trait (`trait Scale<T>`) | Supported | 14 / 25 |  |  | [trait_generic.rs](items/trait_generic.rs) |
| A trait method, static dispatch | Supported | 7 / 18 |  |  | [trait_method.rs](items/trait_method.rs) |
| Operator overloading: `core::ops::Add` on a struct | Supported | 17 / 30 |  |  | [trait_ops_add.rs](items/trait_ops_add.rs) |
| `AddAssign` on a struct | Supported | 13 / 25 |  |  | [trait_ops_addassign_neg.rs](items/trait_ops_addassign_neg.rs) |
| `Neg` on a struct | Supported | 14 / 14 |  |  | [trait_ops_neg.rs](items/trait_ops_neg.rs) |
| `PartialEq` implemented by hand | Supported | 41 / 41 |  |  | [trait_partialeq_by_hand.rs](items/trait_partialeq_by_hand.rs) |
| `PartialOrd` implemented by hand | Supported | 81 / 84 |  | `partial_cmp` returns `Option<Ordering>`, and rustc turns `<` into a three-way compare the compiler doesn't handle yet. | [trait_partialord_by_hand.rs](items/trait_partialord_by_hand.rs) |
| A trait's associated function (no `self`): `T::zero()` | Supported | 25 / 36 |  | rustc writes the MIR of each instance (`f::<i16>`); compiling it as an ordinary function is what the compiler already does for non-generic ones. | [trait_static_method.rs](items/trait_static_method.rs) |
| A supertrait | Supported | 7 / 40 |  |  | [trait_supertrait.rs](items/trait_supertrait.rs) |

### Implementations

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| Chained methods returning `Self` | Supported | 13 / 27 |  |  | [impl_chained.rs](items/impl_chained.rs) |
| A generic `impl<T>` block | Supported | 11 / 11 |  |  | [impl_generic.rs](items/impl_generic.rs) |
| A method of a generic `impl<T>` block that stays a call | Supported | 41 / 41 |  | `#[inline(never)]` keeps the method a call at every opt-level, so `Pair<i16>` is a local of its own: each instance of a generic struct is laid out as a plain struct. | [impl_generic_call.rs](items/impl_generic_call.rs) |
| A generic struct at two types | Supported | 40 / 40 |  | `Pair<i16>` and `Pair<bool>` are two structs, each with its own cells. | [impl_generic_two.rs](items/impl_generic_two.rs) |
| A method taking `&mut self`, not inlined | Supported | 61 / 76 |  |  | [impl_mut_self.rs](items/impl_mut_self.rs) |
| An associated function `new()` and `Self` | Supported | 32 / 43 |  |  | [impl_new.rs](items/impl_new.rs) |
| A method taking `&self`, not inlined | Supported | 19 / 30 |  |  | [impl_ref_self.rs](items/impl_ref_self.rs) |
| A method taking `self` by value | Supported | 15 / 26 |  |  | [impl_self_value.rs](items/impl_self_value.rs) |
| A trait implemented for an enum | Supported | 38 / 38 |  |  | [impl_trait_for_enum.rs](items/impl_trait_for_enum.rs) |

### External blocks

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| An `extern "C"` block declaring a function | Refused |  | Never | There is nothing on the other side of an FFI call on CARDIAC; the card reader and the printer are the whole outside world. The call never runs here, but natively it needs a definition to link. The compiler says: `RCE0007: call to unknown function host_add` | [extern_block.rs](items/extern_block.rs) |
| An `extern "C" fn` defined in Rust and called from Rust | Supported | 13 / 24 |  |  | [extern_fn_def.rs](items/extern_fn_def.rs) |

### Generic parameters

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| A function returning a reference (`fn id(a: &i16) -> &i16`) | Supported | 20 / 20 |  | A wrong answer: the reference that comes back reads the wrong cell (609 for 8), at every setting. | [fn_return_ref.rs](items/fn_return_ref.rs) |
| A generic type parameter with a default (`struct W<T = i16>`) | Supported | 7 / 18 |  |  | [generic_default_param.rs](items/generic_default_param.rs) |
| Several trait bounds (`T: Copy + PartialOrd + Add`) | Supported | 34 / 46 |  | rustc writes the MIR of each instance (`f::<i16>`); compiling it as an ordinary function is what the compiler already does for non-generic ones. | [generic_trait_bound_multi.rs](items/generic_trait_bound_multi.rs) |
| A lifetime-annotated function (`fn pick<'a>(&'a i16, &'a i16) -> &'a i16`) | Supported | 45 / 45 |  | A wrong answer: the returned reference reads the wrong cell (128 for 8), as in the row for a function returning a reference. | [lifetime_fn.rs](items/lifetime_fn.rs) |
| A struct holding a reference (`struct View<'a>`) | Supported | 24 / 35 |  | A function that returns such a struct is refused (RCE0004: unsupported local type `View<'_>`). | [lifetime_struct.rs](items/lifetime_struct.rs) |

### Associated items

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| An associated `const` in an inherent impl | Supported | 13 / 28 |  |  | [assoc_const_inherent.rs](items/assoc_const_inherent.rs) |

## Attributes

### Conditional compilation

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| `#[cfg_attr(all(), inline(never))]` and `#[cfg_attr(any(), ...)]` | Supported | 20 / 33 |  |  | [cfg_attr.rs](attributes/cfg_attr.rs) |
| `#[cfg(debug_assertions)]` and `cfg!(debug_assertions)` | Supported | 5 / 5 |  |  | [cfg_debug_assertions.rs](attributes/cfg_debug_assertions.rs) |
| `cfg!(...)` as a value | Supported | 4 / 4 |  |  | [cfg_macro.rs](attributes/cfg_macro.rs) |
| `#[cfg(not(...))]` on a statement and a block | Supported | 8 / 19 |  |  | [cfg_not.rs](attributes/cfg_not.rs) |
| `#[cfg]` on a struct field, a match arm and an expression in a list | Supported | 17 / 28 |  |  | [cfg_on_field_and_arm.rs](attributes/cfg_on_field_and_arm.rs) |
| `cfg!(target_arch = ...)`: which architecture the CARDIAC build reports | Supported | 9 / 20 |  | The CARDIAC build reports the host's architecture (`aarch64` on this Mac), not one of its own. | [cfg_target_arch.rs](attributes/cfg_target_arch.rs) |
| `cfg!(target_endian = "little")` | Supported | 4 / 4 |  |  | [cfg_target_endian.rs](attributes/cfg_target_endian.rs) |
| `cfg!(target_os = ...)` and `cfg!(unix)`: which system the CARDIAC build reports | Supported | 10 / 31 |  | The CARDIAC build reports the host's system (`macos` and `unix` on this Mac), not one of its own. | [cfg_target_os.rs](attributes/cfg_target_os.rs) |
| `cfg!(target_pointer_width = "16")`, `"32"`, `"64"` | Supported | 16 / 37 |  | The CARDIAC build reports the host's target, so `usize` is 64 bits to rustc on a 64-bit host. | [cfg_target_pointer_width.rs](attributes/cfg_target_pointer_width.rs) |
| `#[cfg(all())]` and `#[cfg(any())]` on functions | Supported | 7 / 18 |  |  | [cfg_true_false.rs](attributes/cfg_true_false.rs) |

### Testing attributes

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| `#[cfg(test)]` code and `cfg!(test)` | Supported | 10 / 21 |  |  | [cfg_test.rs](attributes/cfg_test.rs) |
| `#[ignore]` and `#[should_panic]` beside `#[test]` | Supported | 4 / 4 |  |  | [ignore_should_panic.rs](attributes/ignore_should_panic.rs) |
| a `#[test]` function in the program | Supported | 4 / 4 |  | Outside `cargo test` a `#[test]` function isn't compiled at all, so it costs nothing. | [test_fn_present.rs](attributes/test_fn_present.rs) |

### Derive

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| `#[derive(Clone, Copy)]` on a struct: `.clone()` and copies | Supported | 13 / 25 |  |  | [derive_clone_copy_struct.rs](attributes/derive_clone_copy_struct.rs) |
| `#[derive(Clone, Copy)]` on an enum with data: `.clone()` | Supported | 19 / 30 |  |  | [derive_clone_enum.rs](attributes/derive_clone_enum.rs) |
| `#[derive(Debug)]` on a struct and an enum that are never formatted | Supported | 12 / 23 |  |  | [derive_debug_unused.rs](attributes/derive_debug_unused.rs) |
| `#[derive(Default)]` on an enum with `#[default]` | Supported | 7 / 18 |  |  | [derive_default_enum.rs](attributes/derive_default_enum.rs) |
| `#[derive(Default)]` on a struct: `Default::default()` and `..Default::default()` | Supported | 9 / 9 |  | At opt-level 1 rustc calls `<i16 as Default>::default`, which is just 0. | [derive_default_struct.rs](attributes/derive_default_struct.rs) |
| `#[derive(Hash)]` on a struct and an enum, never hashed | Supported | 10 / 21 |  |  | [derive_hash_unused.rs](attributes/derive_hash_unused.rs) |
| `.hash()` of a `#[derive(Hash)]` value with a hand-written `Hasher` | Supported | 16 / 25 |  | At opt-level 1 the struct is passed whole to the generic derived `hash`. The core library's `Hasher` works in `u64`, and `write_i16` goes through `to_ne_bytes`; a hand-written `Hasher` that folds `i16`s stays on cards. | [derive_hash_used.rs](attributes/derive_hash_used.rs) |
| `#[derive(Ord)]` on a struct: `.cmp()` matched on `Ordering` | Supported | 48 / 48 |  |  | [derive_ord_cmp.rs](attributes/derive_ord_cmp.rs) |
| `#[derive(Ord)]` on a C-like enum: `.cmp()` and `.max()` | Supported | 78 / 78 |  |  | [derive_ord_enum.rs](attributes/derive_ord_enum.rs) |
| `.max()` and `.min()` on a `#[derive(Ord)]` struct | Supported | 85 / 85 |  | The structs are passed whole to the derived `max`, which the compiler doesn't do yet. | [derive_ord_max.rs](attributes/derive_ord_max.rs) |
| `#[derive(PartialEq, Eq)]` on an enum with data: `==` | Supported | 79 / 79 |  | At opt-level 1 the derived `eq` calls `<&i16 as PartialEq>::eq` on the payloads, which is a plain `==`. | [derive_partialeq_enum_data.rs](attributes/derive_partialeq_enum_data.rs) |
| `#[derive(PartialEq)]` on a struct: `==` and `!=` | Supported | 53 / 53 |  |  | [derive_partialeq_struct.rs](attributes/derive_partialeq_struct.rs) |
| `#[derive(PartialOrd)]` on a C-like enum: `<` by declaration order | Supported | 53 / 53 |  |  | [derive_partialord_enum.rs](attributes/derive_partialord_enum.rs) |
| `#[derive(PartialOrd)]` on a struct: `<`, `>=` compare field by field | Supported | 89 / 89 |  | Passes the structs whole by reference (opt-level 1) and builds an `Option<Ordering>` from `partial_cmp` (opt-level 2). | [derive_partialord_struct.rs](attributes/derive_partialord_struct.rs) |

### Diagnostic attributes

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| `#[allow(arithmetic_overflow)]` over a sum rustc knows overflows | Supported | 9 / 6 |  | rustc folds the sum; with checks on both builds panic, with checks off both wrap as `i16` does. | [allow_arithmetic_overflow.rs](attributes/allow_arithmetic_overflow.rs) |
| `#[allow(unconditional_panic)]` over an index rustc knows is out of bounds | Supported | 9 / 18 |  | The program prints, then panics on the index; CARDIAC's output should end where the native one does. | [allow_unconditional_panic.rs](attributes/allow_unconditional_panic.rs) |
| `#[deprecated]` on a function the program calls | Supported | 7 / 18 |  |  | [deprecated_used.rs](attributes/deprecated_used.rs) |
| `#[diagnostic::on_unimplemented]` and `#[diagnostic::do_not_recommend]` | Supported | 7 / 18 |  |  | [diagnostic_on_unimplemented.rs](attributes/diagnostic_on_unimplemented.rs) |
| `#[allow(...)]`, `#[warn(...)]`, `#[deny(...)]`, `#[forbid(...)]` on items | Supported | 4 / 4 |  |  | [lint_allow_deny.rs](attributes/lint_allow_deny.rs) |
| `#[expect(...)]` | Supported | 4 / 4 |  |  | [lint_expect.rs](attributes/lint_expect.rs) |
| `#[must_use]` on a function and a type, the value used and dropped with `let _` | Supported | 20 / 31 |  |  | [must_use.rs](attributes/must_use.rs) |

### Code generation attributes

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| `#[cold]` on an error path | Supported | 24 / 39 |  |  | [cold.rs](attributes/cold.rs) |
| `#[inline(always)]` on a helper called from four places | Supported | 85 / 73 |  | 85 cells at opt-level 2 with checks off, against 56 for `#[inline(never)]`. Compare the cells column across these three rows: always copies the body into each caller, never keeps one copy. | [inline_always.rs](attributes/inline_always.rs) |
| `#[inline(never)]` on a helper called from four places | Supported | 54 / 73 |  | Compare the cells column across these three rows: always copies the body into each caller, never keeps one copy. | [inline_never_three_callers.rs](attributes/inline_never_three_callers.rs) |
| `#[inline]` on a helper called from four places | Supported | 85 / 73 |  | Compare the cells column across these three rows: always copies the body into each caller, never keeps one copy. | [inline_plain.rs](attributes/inline_plain.rs) |
| `#[track_caller]` on a function that may panic | Supported | 21 / 29 |  | The second call panics in `unwrap()`; the panic's location is the caller's line. | [track_caller.rs](attributes/track_caller.rs) |

### Type system attributes

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| `#[non_exhaustive]` on a struct, an enum and a variant (within the crate) | Supported | 20 / 20 |  |  | [non_exhaustive.rs](attributes/non_exhaustive.rs) |

### Type layout (repr)

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| `#[repr(C)]` on an enum with data | Supported | 37 / 37 |  |  | [repr_c_enum_data.rs](attributes/repr_c_enum_data.rs) |
| `#[repr(C)]` and `#[repr(transparent)]` on structs | Supported | 19 / 30 |  |  | [repr_c_struct.rs](attributes/repr_c_struct.rs) |
| `#[repr(i16)]` on a C-like enum with numbers given, `as i16` | Supported | 25 / 25 |  |  | [repr_i16_enum.rs](attributes/repr_i16_enum.rs) |
| `#[repr(i32)]` on a C-like enum whose numbers fit on a card | Supported | 16 / 16 |  |  | [repr_i32_enum.rs](attributes/repr_i32_enum.rs) |
| `#[repr(packed)]` and `#[repr(align(4))]` on structs | Supported | 32 / 43 |  |  | [repr_packed_align.rs](attributes/repr_packed_align.rs) |
| `#[repr(u8)]` on a C-like enum, `as u8` and `as i16` | Supported | 25 / 25 |  |  | [repr_u8_enum.rs](attributes/repr_u8_enum.rs) |
| `#[repr(u8)]` on an enum with data, discriminants given | Supported | 40 / 40 |  |  | [repr_u8_enum_data.rs](attributes/repr_u8_enum_data.rs) |

### Linkage attributes

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| `#[unsafe(export_name = "...")]` on a helper the program calls | Supported | 14 / 25 |  |  | [export_name.rs](attributes/export_name.rs) |
| `#[unsafe(no_mangle)]` on a helper the program calls | Supported | 13 / 24 |  |  | [no_mangle.rs](attributes/no_mangle.rs) |
| `#[no_mangle]` written without `unsafe(...)` (edition 2021) | Supported | 14 / 25 |  | In edition 2024 this form is an error; the row shows which edition the build uses. | [no_mangle_old_form.rs](attributes/no_mangle_old_form.rs) |
| `#[used]` on a static the program reads | Supported | 8 / 19 |  |  | [used_static.rs](attributes/used_static.rs) |

### Documentation attributes

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| `#[doc = "..."]`, `///` comments and `#[doc(hidden)]` | Supported | 7 / 18 |  |  | [doc_attr.rs](attributes/doc_attr.rs) |

## Macros

### Macros by example

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| a macro taking a `$b:block` and repeating it | Supported | 17 / 32 |  |  | [mbe_block_tt.rs](macros/mbe_block_tt.rs) |
| hygiene: a macro's own `let` doesn't capture the caller's name | Supported | 16 / 33 |  |  | [mbe_hygiene.rs](macros/mbe_hygiene.rs) |
| a macro that defines a function from an `$name:ident` | Supported | 14 / 27 |  |  | [mbe_ident_item.rs](macros/mbe_ident_item.rs) |
| a recursive macro (counting its arguments) | Supported | 8 / 19 |  |  | [mbe_recursive.rs](macros/mbe_recursive.rs) |
| a recursive macro (the largest of its arguments) | Supported | 18 / 18 |  |  | [mbe_recursive_max.rs](macros/mbe_recursive_max.rs) |
| `macro_rules!` with a repetition `$($e:expr),*` | Supported | 15 / 26 |  |  | [mbe_repeat.rs](macros/mbe_repeat.rs) |
| a repetition expanding to one statement each | Supported | 11 / 22 |  |  | [mbe_repeat_stmts.rs](macros/mbe_repeat_stmts.rs) |
| `macro_rules!` with several rules, chosen by the tokens | Supported | 7 / 7 |  |  | [mbe_rules.rs](macros/mbe_rules.rs) |
| `macro_rules!` with one rule and an `$e:expr` | Supported | 7 / 18 |  |  | [mbe_simple.rs](macros/mbe_simple.rs) |
| a macro that expands to statements | Supported | 7 / 7 |  |  | [mbe_stmts.rs](macros/mbe_stmts.rs) |

### Assertions

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| `assert_eq!(a, b, "message")` | Supported | 20 / 20 |  | The message is a literal, which could go in the panic map as `unwrap()`'s and `expect()`'s messages do. | [assert_eq_message.rs](macros/assert_eq_message.rs) |
| `assert_eq!(a, b)`: passing, then failing | Supported | 20 / 20 |  | rustc calls `core::panicking::assert_failed` with both values; the compiler could treat it as the panic of `assert!(a == b)`. | [assert_eq_rows.rs](macros/assert_eq_rows.rs) |
| `assert!(cond, "{}", x)` with a formatted message | Supported | 15 / 15 |  | The check could keep the message's literal text and leave out the values; formatting the values themselves is never. | [assert_format.rs](macros/assert_format.rs) |
| `assert!(cond, "message")` | Supported | 15 / 15 |  | The message is a literal, which could go in the panic map as `unwrap()`'s and `expect()`'s messages do. | [assert_message.rs](macros/assert_message.rs) |
| `assert_ne!(a, b)`: passing, then failing | Supported | 21 / 21 |  | rustc calls `core::panicking::assert_failed` with both values; the compiler could treat it as the panic of `assert!(a == b)`. | [assert_ne_rows.rs](macros/assert_ne_rows.rs) |
| `assert!(cond)`: passing, then failing | Supported | 15 / 15 |  |  | [assert_pass_fail.rs](macros/assert_pass_fail.rs) |
| `debug_assert!(cond)` | Supported | 15 / 15 |  |  | [debug_assert.rs](macros/debug_assert.rs) |
| `debug_assert_eq!` and `debug_assert_ne!` | Supported | 24 / 24 |  |  | [debug_assert_eq.rs](macros/debug_assert_eq.rs) |

### Panics

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| `panic!()` | Supported | 14 / 14 |  |  | [panic_bare.rs](macros/panic_bare.rs) |
| `panic!("{}", x)` with a formatted message | Supported | 14 / 14 |  | The check could keep the message's literal text and leave out the values; formatting the values themselves is never. | [panic_format.rs](macros/panic_format.rs) |
| `panic!("message")` | Supported | 14 / 14 |  | The message is a literal, which could go in the panic map as `unwrap()`'s and `expect()`'s messages do. | [panic_message.rs](macros/panic_message.rs) |
| `todo!()` | Supported | 26 / 26 |  |  | [todo_macro.rs](macros/todo_macro.rs) |
| `unimplemented!()` | Supported | 15 / 15 |  |  | [unimplemented_bare.rs](macros/unimplemented_bare.rs) |
| `unimplemented!("message")` | Supported | 15 / 15 |  | The message is a literal, which could go in the panic map as `unwrap()`'s and `expect()`'s messages do. | [unimplemented_message.rs](macros/unimplemented_message.rs) |
| `unreachable!()` | Supported | 27 / 27 |  |  | [unreachable_bare.rs](macros/unreachable_bare.rs) |
| `unreachable!("{}", x)` with a formatted message | Supported | 19 / 19 |  | The check could keep the message's literal text and leave out the values; formatting the values themselves is never. | [unreachable_format.rs](macros/unreachable_format.rs) |
| `unreachable!("message")` | Supported | 19 / 19 |  | The message is a literal, which could go in the panic map as `unwrap()`'s and `expect()`'s messages do. | [unreachable_message.rs](macros/unreachable_message.rs) |

### Other core macros

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| `cfg!(...)` as a condition | Supported | 8 / 19 |  |  | [cfg_macro.rs](macros/cfg_macro.rs) |
| `compile_error!` stops the build (natively too) | Refused |  | Never | This row can't build anywhere: it shows rustc rejecting the program before the compiler sees it. The compiler says: `error: this row is meant not to build` | [compile_error_macro.rs](macros/compile_error_macro.rs) |
| `concat!` and `stringify!`, measured with `.len()` | Supported | 10 / 24 |  | The compiler has no string data yet; the length of a literal is a constant it could fold. | [concat_stringify.rs](macros/concat_stringify.rs) |
| `option_env!` of an unset variable | Supported | 4 / 4 |  |  | [env_option.rs](macros/env_option.rs) |
| `file!()` and `module_path!()`, measured with `.len()` | Supported | 9 / 23 |  | The compiler has no string data yet; the length of a literal is a constant it could fold. | [file_macro.rs](macros/file_macro.rs) |
| `include!` of a Rust expression | Supported | 8 / 19 |  |  | [include_macro.rs](macros/include_macro.rs) |
| `line!()` and `column!()` | Supported | 12 / 23 |  | The row prints the distance between two `line!()`s, since the harness's native copy has a longer prelude. | [line_column.rs](macros/line_column.rs) |
| `matches!(x, pattern if guard)` on an `Option` | Supported | 22 / 22 |  |  | [matches_guard.rs](macros/matches_guard.rs) |
| `matches!(x, pattern)` | Supported | 34 / 34 |  |  | [matches_macro.rs](macros/matches_macro.rs) |

### Formatting macros

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| `core::write!` and `format_args!` into a `core::fmt::Write` | Refused |  | Never | String formatting: the `fmt` machinery is dynamic dispatch over trait objects and digit loops in `u32`. The compiler says: `RCE0035: format! and {} formatting aren't supported on CARDIAC, which has no text: print numbers with out() (enum core::fmt::rt::ArgumentType: field 1 of Placeholder is : the fields of an enum's varia...` | [format_args_write.rs](macros/format_args_write.rs) |

### Standard library macros

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| `format!` and `vec!`, with `extern crate std` | Refused |  | Never | Both need the heap. The compiler says: `RCE0035: format! and {} formatting aren't supported on CARDIAC, which has no text: print numbers with out() (enum core::fmt::rt::ArgumentType: field 1 of Placeholder is : the fields of an enum's varia...` | [std_format_vec.rs](macros/std_format_vec.rs) |
| `println!`, with `extern crate std` | Refused |  | Never | Needs `std` and string formatting; `out()` prints a value. The compiler says: `RCE0034: println! and print! aren't supported on CARDIAC, which prints only numbers: use out() (enum core::fmt::rt::ArgumentType: field 1 of Placeholder is : the fields of an enum's variants must be s...` | [std_println.rs](macros/std_println.rs) |

## Integer methods

### Sign and absolute value

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| `a.abs_diff(b)` on `i16` | Supported | 46 / 46 |  | It returns a `u16`, which the compiler lacks; on a card the answer is a subtraction and a compare. | [abs_diff_i16.rs](integers/abs_diff_i16.rs) |
| `a.abs_diff(b)` on `u8` | Supported | 59 / 59 |  | At opt-level 2 rustc does the subtraction in `u32`, which the compiler lacks. | [abs_diff_u8.rs](integers/abs_diff_u8.rs) |
| two `u8::abs_diff`s in one function, added with `+` (a panic past 255 with overflow checks on) | Supported | 82 / 79 |  |  | [abs_diff_u8_add.rs](integers/abs_diff_u8_add.rs) |
| `u8::abs_diff` against constants, and with a u8 array element | Supported | 68 / 74 |  |  | [abs_diff_u8_const.rs](integers/abs_diff_u8_const.rs) |
| `a.abs_diff(b)` on `u8`, both orders of every end, through casts of negative cards | Supported | 42 / 42 |  |  | [abs_diff_u8_ends.rs](integers/abs_diff_u8_ends.rs) |
| `u8::abs_diff` of neighbours in a loop, summed in an i16 | Supported | 46 / 57 |  |  | [abs_diff_u8_loop.rs](integers/abs_diff_u8_loop.rs) |
| two `u8::abs_diff`s in one function, added as u8 (overflows past 255) | Supported | 82 / 82 |  |  | [abs_diff_u8_sum.rs](integers/abs_diff_u8_sum.rs) |
| `a.abs_diff(b)` on `usize` | Supported | 33 / 33 |  |  | [abs_diff_usize.rs](integers/abs_diff_usize.rs) |
| `a.abs()` on `i16` | Supported | 18 / 18 |  |  | [abs_i16.rs](integers/abs_i16.rs) |
| `a.is_negative()` on `i16` | Supported | 18 / 18 |  |  | [is_negative_i16.rs](integers/is_negative_i16.rs) |
| `a.is_positive()` on `i16` | Supported | 19 / 19 |  |  | [is_positive_i16.rs](integers/is_positive_i16.rs) |
| `a.signum()` on `i16` | Supported | 24 / 24 |  | At opt-level 2 rustc inlines it into a three-way compare (`Cmp`), which is two tests on CARDIAC. | [signum_i16.rs](integers/signum_i16.rs) |
| `a.unsigned_abs()` on `i16` | Supported | 18 / 18 |  | It returns a `u16`, which the compiler lacks; on a card it is `abs`. | [unsigned_abs_i16.rs](integers/unsigned_abs_i16.rs) |
| `a.wrapping_abs()` on `i16` | Supported | 18 / 18 |  |  | [wrapping_abs_i16.rs](integers/wrapping_abs_i16.rs) |

### Min, max and clamp

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| `a.clamp(lo, hi)` on `i16` | Supported | 30 / 30 |  |  | [clamp_i16.rs](integers/clamp_i16.rs) |
| `a.clamp(lo, hi)` with `lo > hi` panics | Supported | 24 / 24 |  |  | [clamp_panic_i16.rs](integers/clamp_panic_i16.rs) |
| `a.clamp(2, 200)` on `u8` | Supported | 42 / 42 |  |  | [clamp_u8.rs](integers/clamp_u8.rs) |
| `core::cmp::min(a, b)` and `core::cmp::max(a, b)` on `i16` | Supported | 31 / 31 |  |  | [cmp_min_max_i16.rs](integers/cmp_min_max_i16.rs) |
| `a.max(b)` on `i16` | Supported | 21 / 21 |  |  | [max_i16.rs](integers/max_i16.rs) |
| `a.max(b)` on `usize` | Supported | 28 / 28 |  |  | [max_usize.rs](integers/max_usize.rs) |
| `a.min(b)` on `i16` | Supported | 22 / 22 |  |  | [min_i16.rs](integers/min_i16.rs) |
| `a.min(b)` on `u8` | Supported | 55 / 55 |  |  | [min_u8.rs](integers/min_u8.rs) |

### Comparison and Ord

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| `a.cmp(&b)` on `i16`, matched on `Ordering` | Supported | 32 / 32 |  | `Ord::cmp` is a call at opt-level 1 and a three-way compare (`Cmp`) at opt-level 2; either is two tests on CARDIAC. | [ord_cmp_i16.rs](integers/ord_cmp_i16.rs) |
| `a.cmp(&b).is_lt()` and `.is_ge()` on `i16` | Supported | 46 / 46 |  | As for `cmp`; at opt-level 2 rustc also reads the `Ordering` as an `i8`, which the compiler lacks. | [ord_cmp_is_lt_i16.rs](integers/ord_cmp_is_lt_i16.rs) |
| `a.cmp(&b)` on `u8`, matched on `Ordering` | Supported | 65 / 65 |  | As for `cmp` on `i16`. | [ord_cmp_u8.rs](integers/ord_cmp_u8.rs) |

### Checked

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| `a.checked_add(b)` on `i16`, on a card | Supported | 38 / 38 |  |  | [checked_add_i16.rs](integers/checked_add_i16.rs) |
| `a.checked_add(b)` on `i16` past a card | Differs from native Rust by design |  | Never | The `None` is CARDIAC's overflow: every `i16` overflows past a card. CARDIAC's `i16` overflows past a card (999), so 1000 is `None` where native Rust has `Some(1000)`. | [checked_add_past_card.rs](integers/checked_add_past_card.rs) |
| `a.checked_add(b)` on `u8` | Supported | 77 / 77 |  | At opt-level 1 rustc calls the method rather than inlining it; the compiler could write what rustc inlines at opt-level 2, as it does for `min` and `max`. | [checked_add_u8.rs](integers/checked_add_u8.rs) |
| `a.checked_div(b)` on `i16` | Supported | 65 / 65 |  | The compiler crashes at opt-level 2: rustc's inlined test against `i16::MIN` puts -32768 into a card, and the compiler panics instead of refusing it. No card holds `i16::MIN`, so on CARDIAC the test is never true. At opt-level 1 it is a call. | [checked_div_i16.rs](integers/checked_div_i16.rs) |
| `a.checked_mul(b)` on `i16` | Supported | 71 / 71 |  |  | [checked_mul_i16.rs](integers/checked_mul_i16.rs) |
| `a.checked_neg()` on `i16` | Supported | 14 / 14 |  | The compiler crashes at opt-level 2: rustc's inlined test against `i16::MIN` puts -32768 into a card, and the compiler panics instead of refusing it. No card holds `i16::MIN`, so on CARDIAC the test is never true. At opt-level 1 it is a call. | [checked_neg_i16.rs](integers/checked_neg_i16.rs) |
| `a.checked_pow(3)` on `i16` | Supported | 79 / 79 |  | A loop of checked multiplies; rustc calls it at both opt-levels. | [checked_pow_i16.rs](integers/checked_pow_i16.rs) |
| `a.checked_rem(b)` on `i16` | Supported | 78 / 78 |  | The compiler crashes at opt-level 2: rustc's inlined test against `i16::MIN` puts -32768 into a card, and the compiler panics instead of refusing it. No card holds `i16::MIN`, so on CARDIAC the test is never true. At opt-level 1 it is a call. | [checked_rem_i16.rs](integers/checked_rem_i16.rs) |
| `a.checked_sub(b)` on `i16` | Supported | 38 / 38 |  |  | [checked_sub_i16.rs](integers/checked_sub_i16.rs) |
| `a.checked_sub(b)` on `usize` | Supported | 32 / 32 |  | At opt-level 1 rustc calls the method rather than inlining it; the compiler could write what rustc inlines at opt-level 2, as it does for `min` and `max`. | [checked_sub_usize.rs](integers/checked_sub_usize.rs) |

### Wrapping

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| `a.wrapping_add(b)` on `i16`, on a card | Supported | 19 / 19 |  |  | [wrapping_add_i16.rs](integers/wrapping_add_i16.rs) |
| `a.wrapping_add(b)` on `u8` | Supported | 57 / 57 |  | At opt-level 1 rustc calls the method rather than inlining it; the compiler could write what rustc inlines at opt-level 2, as it does for `min` and `max`. | [wrapping_add_u8.rs](integers/wrapping_add_u8.rs) |
| `a.wrapping_div(b)` on `i16`, on a card | Supported | 65 / 65 |  | On a card it is plain `/`; rustc calls `overflowing_div` even at opt-level 2, for the `i16::MIN / -1` case that no card holds. | [wrapping_div_i16.rs](integers/wrapping_div_i16.rs) |
| `a.wrapping_mul(b)` on `i16`, on a card | Supported | 44 / 44 |  |  | [wrapping_mul_i16.rs](integers/wrapping_mul_i16.rs) |
| `a.wrapping_neg()` on `i16` | Supported | 14 / 14 |  |  | [wrapping_neg_i16.rs](integers/wrapping_neg_i16.rs) |
| `a.wrapping_rem(b)` on `i16`, on a card | Supported | 87 / 87 |  | At opt-level 1 rustc calls the method rather than inlining it; the compiler could write what rustc inlines at opt-level 2, as it does for `min` and `max`. | [wrapping_rem_i16.rs](integers/wrapping_rem_i16.rs) |
| `a.wrapping_sub(b)` on `i16`, on a card | Supported | 19 / 19 |  |  | [wrapping_sub_i16.rs](integers/wrapping_sub_i16.rs) |
| `a.wrapping_sub(b)` on `u8` | Supported | 56 / 56 |  | At opt-level 1 rustc calls the method rather than inlining it; the compiler could write what rustc inlines at opt-level 2, as it does for `min` and `max`. | [wrapping_sub_u8.rs](integers/wrapping_sub_u8.rs) |

### Saturating

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| `a.saturating_add(b)` on `i16`, on a card | Supported | 30 / 30 |  | Rust saturates at 32767, which no card holds, so past a card CARDIAC would print something else; on a card it is a plain add. | [saturating_add_i16.rs](integers/saturating_add_i16.rs) |
| `a.saturating_add(b)` on `u8` | Supported | 60 / 60 |  | 255 fits on a card, so this is an add and a compare; rustc uses the `saturating_add` intrinsic, which the compiler doesn't know. | [saturating_add_u8.rs](integers/saturating_add_u8.rs) |
| `a.saturating_mul(b)` on `i16`, on a card | Supported | 51 / 51 |  | As for `saturating_add`. At opt-level 2 the compiler crashes: rustc's inlined version picks between 32767 and -32768, and the compiler panics on a constant no card holds instead of refusing it. | [saturating_mul_i16.rs](integers/saturating_mul_i16.rs) |
| `a.saturating_mul(b)` on `u8` | Supported | 67 / 67 |  | At opt-level 2 rustc's inlined version doesn't fit in the cells; a multiply and a compare would. | [saturating_mul_u8.rs](integers/saturating_mul_u8.rs) |
| `a.saturating_sub(b)` on `i16`, on a card | Supported | 30 / 30 |  | As for `saturating_add`: Rust's limit is -32768, which no card holds. | [saturating_sub_i16.rs](integers/saturating_sub_i16.rs) |
| `a.saturating_sub(b)` on `u8` | Supported | 58 / 58 |  |  | [saturating_sub_u8.rs](integers/saturating_sub_u8.rs) |
| `a.saturating_sub(b)` on `usize` | Supported | 31 / 31 |  |  | [saturating_sub_usize.rs](integers/saturating_sub_usize.rs) |

### Overflowing

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| `a.overflowing_add(b)` on `i16`, on a card | Supported | 35 / 35 |  | At opt-level 1 rustc calls the method rather than inlining it; the compiler could write what rustc inlines at opt-level 2, as it does for `min` and `max`. | [overflowing_add_i16.rs](integers/overflowing_add_i16.rs) |
| `a.overflowing_add(b)` on `u8` | Supported | 70 / 70 |  | A wrong answer at opt-level 2: the flag is right but the sum isn't wrapped, so 200 + 100 prints 300 where Rust prints 44. At opt-level 1 it is a call. | [overflowing_add_u8.rs](integers/overflowing_add_u8.rs) |
| `a.overflowing_sub(b)` on `i16`, on a card | Supported | 35 / 35 |  | At opt-level 1 rustc calls the method rather than inlining it; the compiler could write what rustc inlines at opt-level 2, as it does for `min` and `max`. | [overflowing_sub_i16.rs](integers/overflowing_sub_i16.rs) |

### Euclidean division

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| `a.div_euclid(7)` on `i16` | Supported | 67 / 67 |  |  | [div_euclid_const_i16.rs](integers/div_euclid_const_i16.rs) |
| `a.div_euclid(k)` on `i16` by 10 | Supported | 26 / 26 |  |  | [div_euclid_k10.rs](integers/div_euclid_k10.rs) |
| `a.div_euclid(k)` on `i16` by 3 | Supported | 69 / 69 |  |  | [div_euclid_k3.rs](integers/div_euclid_k3.rs) |
| `a.div_euclid(k)` on `i16` by 7 | Supported | 69 / 69 |  |  | [div_euclid_k7.rs](integers/div_euclid_k7.rs) |
| `a.div_euclid(k)` on `i16` by -1 | Supported | 69 / 69 |  |  | [div_euclid_km1.rs](integers/div_euclid_km1.rs) |
| `a.div_euclid(k)` on `i16` by -100 | Supported | 30 / 30 |  |  | [div_euclid_km100.rs](integers/div_euclid_km100.rs) |
| `a.div_euclid(k)` on `i16` by -7 | Supported | 69 / 69 |  |  | [div_euclid_km7.rs](integers/div_euclid_km7.rs) |
| `a.div_euclid(b)` on `i16` | Supported | 87 / 87 |  | `rem_euclid` by a variable works; `div_euclid` is the same test around a divide. | [div_euclid_var_i16.rs](integers/div_euclid_var_i16.rs) |
| `a = a.div_euclid(b)`, the result in the dividend's own cell | Supported | 84 / 84 |  |  | [div_euclid_var_inplace.rs](integers/div_euclid_var_inplace.rs) |
| `b = a.div_euclid(b)`, the result in the divisor's own cell | Supported | 84 / 84 |  |  | [div_euclid_var_inplace_b.rs](integers/div_euclid_var_inplace_b.rs) |
| `a.div_euclid(b)` on `i16` by a variable: every sign, exact and not, the ends of a card | Supported | 87 / 87 |  |  | [div_euclid_var_signs.rs](integers/div_euclid_var_signs.rs) |
| `a.div_euclid(b)` by a variable that is 0: Rust's panic | Supported | 77 / 77 |  |  | [div_euclid_var_zero.rs](integers/div_euclid_var_zero.rs) |
| `a.rem_euclid(7)` on `i16` | Supported | 74 / 74 |  |  | [rem_euclid_const_i16.rs](integers/rem_euclid_const_i16.rs) |
| `a.rem_euclid(b)` on `i16` | Supported | 92 / 92 |  |  | [rem_euclid_var_i16.rs](integers/rem_euclid_var_i16.rs) |
| `a.rem_euclid(b)` with `b` zero panics | Supported | 82 / 82 |  |  | [rem_euclid_zero_i16.rs](integers/rem_euclid_zero_i16.rs) |

### Powers and roots

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| `a.ilog10()` on `i16` | Supported | 33 / 33 |  | On a card it is two compares (against 10 and 100); rustc works in `u16` and `u32`, which the compiler lacks. | [ilog10_i16.rs](integers/ilog10_i16.rs) |
| `a.ilog10()` on `i16` at every boundary, then a negative: Rust's panic | Supported | 28 / 28 |  |  | [ilog10_i16_ends.rs](integers/ilog10_i16_ends.rs) |
| `ilog10` in a function of the program's own, its result cast, added to and compared | Supported | 46 / 46 |  |  | [ilog10_i16_fn.rs](integers/ilog10_i16_fn.rs) |
| `a.ilog10()` on `i16` at every boundary, then 0: Rust's panic | Supported | 28 / 28 |  |  | [ilog10_i16_zero.rs](integers/ilog10_i16_zero.rs) |
| `a.is_power_of_two()` on `u8` | Supported | 66 / 66 |  | rustc counts bits (in `u32`); on a decimal machine it would be a table or a loop of halvings. | [is_power_of_two_u8.rs](integers/is_power_of_two_u8.rs) |
| `a.is_power_of_two()` on `usize` | Supported | 66 / 66 |  | As for `u8`. | [is_power_of_two_usize.rs](integers/is_power_of_two_usize.rs) |
| `a.isqrt()` on `i16` | Supported | 35 / 35 |  | It works in `u16`, which the compiler lacks; a short loop would do on a card. | [isqrt_i16.rs](integers/isqrt_i16.rs) |
| `a.midpoint(b)` on `i16` | Refused |  | Never | Refused on purpose: rustc adds the two in `i32` and halves, and where the sum passes a card (999 and 998) CARDIAC would print 598 where Rust prints 998. `(a + b) / 2` works where the sum stays on a card. The compiler says: `RCE0002: unsupported call to core::num::<impl i16>::midpoint, in main: '_4 = core::num::<impl i16>::midpoint(move _4, move _2) -> [return: bb7, unwind continue]'` The compiler says: `RCE0010: core::num::<impl i16>::midpoint, inlined in main, works in i32 so that it can't overflow, and on CARDIAC an i32 is a card (-999 to 999), past which it can: it is not supported` | [midpoint_i16.rs](integers/midpoint_i16.rs) |
| `i16::midpoint` of values whose sum is on a card: still refused, since the rule can't know that | Refused |  | Never | The compiler says: `RCE0002: unsupported call to core::num::<impl i16>::midpoint, in main: '_4 = core::num::<impl i16>::midpoint(move _4, move _2) -> [return: bb7, unwind continue]'` The compiler says: `RCE0010: core::num::<impl i16>::midpoint, inlined in main, works in i32 so that it can't overflow, and on CARDIAC an i32 is a card (-999 to 999), past which it can: it is not supported` | [midpoint_i16_small.rs](integers/midpoint_i16_small.rs) |
| `a.midpoint(b)` on `u8` | Supported | 58 / 58 |  | rustc adds in `u16`, which the compiler lacks; the sum of two `u8`s always fits on a card. | [midpoint_u8.rs](integers/midpoint_u8.rs) |
| `a.next_power_of_two()` on `u8` | Supported | 89 / 89 |  | rustc counts leading zeros (in `u32`); on a card it would be a loop of doublings. | [next_power_of_two_u8.rs](integers/next_power_of_two_u8.rs) |
| `a.next_power_of_two()` on `usize` | Supported | 91 / 96 |  | As for `u8`. | [next_power_of_two_usize.rs](integers/next_power_of_two_usize.rs) |
| `a.pow(3)` on `i16` | Supported | 67 / 67 |  | With a constant exponent `pow` is a few multiplies, but rustc calls it at both opt-levels. | [pow_const_i16.rs](integers/pow_const_i16.rs) |
| `a.pow(n)` on `i16`, `n` from a card | Supported | 76 / 76 |  | The exponent is a `u32`, which the compiler lacks; after that it is a loop of multiplies. | [pow_var_i16.rs](integers/pow_var_i16.rs) |

### Bits

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| `a.count_ones()` on `i16` | Supported | 55 / 55 |  | Bit counts are `u32`s, and on a decimal machine a loop of halvings. | [count_ones_i16.rs](integers/count_ones_i16.rs) |
| `a.count_ones()` on `u8` | Supported | 57 / 57 |  | As for `i16`. | [count_ones_u8.rs](integers/count_ones_u8.rs) |
| `a.leading_zeros()` on `i16` | Supported | 42 / 42 |  | As for `count_ones`. | [leading_zeros_i16.rs](integers/leading_zeros_i16.rs) |
| `a.rotate_left(1)` on `u8` | Supported | 38 / 38 |  | A rotate is a bit operation; rustc calls an intrinsic for it. | [rotate_left_u8.rs](integers/rotate_left_u8.rs) |
| `a.swap_bytes()` on `i16` | Refused |  | Never | Swapping an i16's bytes moves its low byte to the high one: most values on a card come out far past one (100 becomes 25600), so the result can't be held. The compiler says: `RCE0004: unsupported local type u16 for _4 (this backend has u8, i16, usize and bool)` | [swap_bytes_i16.rs](integers/swap_bytes_i16.rs) |
| `a.trailing_zeros()` on `i16` | Supported | 54 / 54 |  | As for `count_ones`. | [trailing_zeros_i16.rs](integers/trailing_zeros_i16.rs) |

### Conversions

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| `i16::from_str_radix(s, 10)` | Refused |  | Never | A program's input is numbers on cards, so there is no text to parse; the strings to parse could only be literals, a cell a byte. The compiler says: `RCE0004: unsupported local type [&str; 2] for _4 (this backend has u8, i16, usize and bool)` | [from_str_radix_i16.rs](integers/from_str_radix_i16.rs) |
| `i16::from(a)` for a `u8` | Supported | 28 / 28 |  | At opt-level 1 rustc calls `From::from`; it is `as i16`, which works. | [from_u8_i16.rs](integers/from_u8_i16.rs) |
| `usize::from(a)` for a `u8` | Supported | 28 / 28 |  | As for `i16::from`. | [from_u8_usize.rs](integers/from_u8_usize.rs) |
| `a.into()` from `u8` to `i16` | Supported | 28 / 28 |  | As for `i16::from`. | [into_u8_i16.rs](integers/into_u8_i16.rs) |
| `u8::try_from(a)` for an `i16` | Supported | 21 / 21 |  | It is a range check (0 to 255) and then the cast. | [try_from_i16_u8.rs](integers/try_from_i16_u8.rs) |
| `usize::try_from(a)` for an `i16` | Supported | 17 / 17 |  | At opt-level 1 rustc calls the method rather than inlining it; the compiler could write what rustc inlines at opt-level 2, as it does for `min` and `max`. | [try_from_i16_usize.rs](integers/try_from_i16_usize.rs) |
| `i16::try_from(a)` for a `usize` | Supported | 22 / 22 |  | At opt-level 2 rustc compares against 32767, which no card holds; every `usize` on a card passes that test. | [try_from_usize_i16.rs](integers/try_from_usize_i16.rs) |
| `u8::try_from(a)` for a `usize` | Supported | 28 / 28 |  | At opt-level 1 rustc calls the method rather than inlining it; the compiler could write what rustc inlines at opt-level 2, as it does for `min` and `max`. | [try_from_usize_u8.rs](integers/try_from_usize_u8.rs) |
| `a.try_into()` from `i16` to `u8` | Supported | 18 / 18 |  | As for `u8::try_from`. | [try_into_i16_u8.rs](integers/try_into_i16_u8.rs) |

### Constants

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| `i16::BITS`, `u8::BITS` and `usize::BITS` | Supported | 33 / 66 |  |  | [bits.rs](integers/bits.rs) |
| `i16::MAX` | Refused |  | Never | Refused on purpose (RCE0009), as other constants past a card are; comparing with one works. `i16::MAX` is 32767 in Rust, which no card holds; on CARDIAC an `i16` ends at 999. The compiler says: `RCE0009: constant out of range: i16::MAX (32767) is past what a card holds (-999 to 999): no i16 on a card reaches it, so code that uses it as a value (a saturating method, say) can't be compiled, in ...` | [i16_max.rs](integers/i16_max.rs) |
| `i16::MIN` | Refused |  | Never | Refused on purpose (RCE0009), as other constants past a card are; comparing with one works. `i16::MIN` is -32768 in Rust, which no card holds; on CARDIAC an `i16` ends at -999. The compiler says: `RCE0009: constant out of range: i16::MIN (-32768) is past what a card holds (-999 to 999): no i16 on a card reaches it, so code that uses it as a value (a saturating method, say) can't be compiled, in...` | [i16_min.rs](integers/i16_min.rs) |
| `u8::MIN` and `u8::MAX` | Supported | 9 / 20 |  |  | [u8_min_max.rs](integers/u8_min_max.rs) |

## The rest of the core library

### Option

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| `Option::and` | Supported | 41 / 41 |  | At opt-level 1 rustc calls it rather than inlining it, and the compiler doesn't yet write out that call. | [option_and.rs](library/option_and.rs) |
| `Option::and_then` | Supported | 34 / 34 |  | At opt-level 1 rustc calls it, as it does `map`, whose call the compiler already writes out. | [option_and_then.rs](library/option_and_then.rs) |
| `Option::as_mut` | Supported | 33 / 45 |  | It makes an `Option<&mut i16>`, and an enum's fields must be scalars for now. | [option_as_mut.rs](library/option_as_mut.rs) |
| `Option::as_ref` | Supported | 31 / 43 |  | It makes an `Option<&i16>`, and an enum's fields must be scalars for now. | [option_as_ref.rs](library/option_as_ref.rs) |
| `Option<&i16>::cloned` | Supported | 16 / 16 |  | An `Option<&i16>` holds a reference, and an enum's fields must be scalars for now. | [option_cloned.rs](library/option_cloned.rs) |
| `Option<&i16>::copied` | Supported | 16 / 16 |  | An `Option<&i16>` holds a reference, and an enum's fields must be scalars for now. | [option_copied.rs](library/option_copied.rs) |
| `==` between two `Option<i16>` | Supported | 60 / 60 |  |  | [option_eq.rs](library/option_eq.rs) |
| `Option::expect` | Supported | 28 / 28 |  |  | [option_expect.rs](library/option_expect.rs) |
| `Option::filter` | Supported | 48 / 48 |  | rustc calls it at both opt-levels; the predicate takes a reference to the payload, which is a scalar. | [option_filter.rs](library/option_filter.rs) |
| `Option::get_or_insert` | Supported | 41 / 52 |  | It returns a `&mut` into the `Option`, and at opt-level 2 it goes through a closure inside core. | [option_get_or_insert.rs](library/option_get_or_insert.rs) |
| `Option::insert` | Supported | 27 / 38 |  | At opt-level 1 rustc calls it on a `&mut Option`, which the compiler doesn't yet write out. | [option_insert.rs](library/option_insert.rs) |
| `Option::is_some` and `is_none` | Supported | 35 / 35 |  |  | [option_is_some.rs](library/option_is_some.rs) |
| `Option::is_some_and` | Supported | 38 / 38 |  | At opt-level 1 rustc calls it, as it does `map`, whose call the compiler already writes out. | [option_is_some_and.rs](library/option_is_some_and.rs) |
| `<` between two `Option<i16>` (`None` is least) | Supported | 69 / 69 |  | It goes through `Ordering` (an `i8`) at opt-level 2 and a `PartialOrd::lt` call at opt-level 1. | [option_lt.rs](library/option_lt.rs) |
| `Option::map` with a closure that captures nothing | Supported | 31 / 42 |  |  | [option_map.rs](library/option_map.rs) |
| `Option::map` with a closure that captures a variable | Supported | 33 / 44 |  | The closure captures a variable, which the compiler can't yet pass where rustc calls `map` (opt-level 1). | [option_map_capture.rs](library/option_map_capture.rs) |
| `Option::map_or` | Supported | 38 / 49 |  | rustc calls it at both opt-levels; it is `map` and then `unwrap_or`, both of which work. | [option_map_or.rs](library/option_map_or.rs) |
| `Option::map_or_else` | Supported | 44 / 55 |  | rustc calls it at both opt-levels; it is `map` and then `unwrap_or_else`. | [option_map_or_else.rs](library/option_map_or_else.rs) |
| `Option::ok_or` | Supported | 29 / 29 |  |  | [option_ok_or.rs](library/option_ok_or.rs) |
| `Option::ok_or_else` | Supported | 29 / 29 |  | At opt-level 1 rustc calls it, as it does `map`, whose call the compiler already writes out. | [option_ok_or_else.rs](library/option_ok_or_else.rs) |
| `Option::or` | Supported | 42 / 42 |  | At opt-level 1 rustc calls it rather than inlining it, and the compiler doesn't yet write out that call. | [option_or.rs](library/option_or.rs) |
| `Option::or_else` | Supported | 40 / 40 |  | rustc calls it at both opt-levels, and the compiler doesn't yet write out that call. | [option_or_else.rs](library/option_or_else.rs) |
| `Option::replace` | Supported | 43 / 43 |  | At opt-level 1 rustc calls it on a `&mut Option`, which the compiler doesn't yet write out. | [option_replace.rs](library/option_replace.rs) |
| `Option::take` | Supported | 43 / 43 |  | At opt-level 1 rustc calls it on a `&mut Option`, which the compiler doesn't yet write out. | [option_take.rs](library/option_take.rs) |
| `Option::unwrap` | Supported | 28 / 28 |  |  | [option_unwrap.rs](library/option_unwrap.rs) |
| `Option::unwrap_or` | Supported | 29 / 29 |  |  | [option_unwrap_or.rs](library/option_unwrap_or.rs) |
| `Option::unwrap_or_default` | Supported | 29 / 29 |  | At opt-level 1 rustc calls it; it is `unwrap_or(0)`. | [option_unwrap_or_default.rs](library/option_unwrap_or_default.rs) |
| `Option::unwrap_or_else` with a capturing closure | Supported | 30 / 30 |  | The closure captures a variable, which the compiler can't yet pass where rustc calls the method (opt-level 1). | [option_unwrap_or_else.rs](library/option_unwrap_or_else.rs) |
| `Option::xor` | Supported | 45 / 45 |  | At opt-level 1 rustc calls it rather than inlining it, and the compiler doesn't yet write out that call. | [option_xor.rs](library/option_xor.rs) |
| `Option::zip` | Supported | 44 / 69 |  | The payload of the result is a tuple, and an enum's fields must be scalars for now. | [option_zip.rs](library/option_zip.rs) |

### Result

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| `Result::and_then` | Supported | 46 / 57 |  | At opt-level 1 rustc calls it, as it does `map`, whose call the compiler already writes out. | [result_and_then.rs](library/result_and_then.rs) |
| `Result::expect` | Supported | 35 / 35 |  |  | [result_expect.rs](library/result_expect.rs) |
| `Result::is_ok` and `is_err` | Supported | 40 / 40 |  |  | [result_is_ok.rs](library/result_is_ok.rs) |
| `Result::map` | Supported | 37 / 48 |  |  | [result_map.rs](library/result_map.rs) |
| `Result::map_err` | Supported | 34 / 55 |  | At opt-level 1 rustc calls it, as it does `map`, whose call the compiler already writes out. | [result_map_err.rs](library/result_map_err.rs) |
| `Result::ok` and `err` | Supported | 45 / 45 |  |  | [result_ok_err.rs](library/result_ok_err.rs) |
| `?` converting the error with `From` | Supported | 59 / 91 |  | A wrong answer at opt-level 1: the error comes out unconverted, as if `From::from` weren't called (-3 where Rust prints -30). | [result_question_from.rs](library/result_question_from.rs) |
| `Result::unwrap_or` | Supported | 34 / 34 |  |  | [result_unwrap_or.rs](library/result_unwrap_or.rs) |
| `Result::unwrap_or_else` | Supported | 35 / 56 |  | At opt-level 1 rustc calls it, as it does `map`, whose call the compiler already writes out. | [result_unwrap_or_else.rs](library/result_unwrap_or_else.rs) |

### Arrays and slices

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| `array.binary_search(&x)` | Supported | 70 / 70 |  |  | [array_binary_search.rs](library/array_binary_search.rs) |
| `array.chunks(2)` | Supported | 74 / 84 |  | Each chunk is a slice in an `Option`, which the compiler doesn't have. | [array_chunks.rs](library/array_chunks.rs) |
| `array.contains(&x)` | Supported | 45 / 45 |  | On an array of known length it is a short loop the compiler already writes for `for i in 0..N`. | [array_contains.rs](library/array_contains.rs) |
| `slice.copy_from_slice(...)` | Supported | 68 / 68 |  | Subslices with run-time bounds need slices, which the compiler doesn't have. | [array_copy_from_slice.rs](library/array_copy_from_slice.rs) |
| `==` between two arrays | Supported | 46 / 46 |  | Element by element, as `a[0] == b[0] && ...`; at opt-level 2 rustc compares the bytes as a `u64`, which the compiler would have to recognise. | [array_eq.rs](library/array_eq.rs) |
| `array.fill(x)` | Supported | 33 / 33 |  | On an array of known length it is a short loop the compiler already writes for `for i in 0..N`. | [array_fill.rs](library/array_fill.rs) |
| `array.first()` and `last()` | Supported | 20 / 20 |  | They return an `Option<&i16>`, and an enum's fields must be scalars for now. | [array_first_last.rs](library/array_first_last.rs) |
| `core::array::from_fn` | Supported | 74 / 85 |  |  | [array_from_fn.rs](library/array_from_fn.rs) |
| `array.get(i)`, which returns an `Option` | Supported | 32 / 32 |  | It returns an `Option<&i16>`, and an enum's fields must be scalars for now. | [array_get.rs](library/array_get.rs) |
| `array.iter().all(...)` | Supported | 84 / 84 |  | The same loop as `for i in 0..N` with an early return. | [array_iter_all.rs](library/array_iter_all.rs) |
| `array.iter().any(...)` | Supported | 83 / 83 |  | The same loop as `for i in 0..N` with an early return. | [array_iter_any.rs](library/array_iter_any.rs) |
| `array.iter().filter(...).count()` | Supported | 38 / 38 |  | A `filter` adaptor over a slice iterator. | [array_iter_count.rs](library/array_iter_count.rs) |
| `array.iter().find(...)` | Supported | 61 / 61 |  | It returns an `Option<&i16>`, and an enum's fields must be scalars for now. | [array_iter_find.rs](library/array_iter_find.rs) |
| `array.iter().fold(...)` | Supported | 41 / 65 |  | The same loop as `for i in 0..N` with the closure's body inside it. | [array_iter_fold.rs](library/array_iter_fold.rs) |
| `array.iter().max()` | Supported | 54 / 54 |  | It returns an `Option<&i16>`, and an enum's fields must be scalars for now. | [array_iter_max.rs](library/array_iter_max.rs) |
| `array.iter().min()` | Supported | 53 / 53 |  | It returns an `Option<&i16>`, and an enum's fields must be scalars for now. | [array_iter_min.rs](library/array_iter_min.rs) |
| `array.iter().position(...)` | Supported |  |  | The same loop as `for i in 0..N` with an early return of `i`. | [array_iter_position.rs](library/array_iter_position.rs) |
| `array.iter().sum()` | Supported | 39 / 39 |  | The same loop as `for i in 0..N` adding `t[i]`. | [array_iter_sum.rs](library/array_iter_sum.rs) |
| `[T; N]::map` | Supported | 60 / 71 |  |  | [array_map.rs](library/array_map.rs) |
| `array.reverse()` | Supported | 46 / 46 |  | On an array of known length it is a short loop of swaps. | [array_reverse.rs](library/array_reverse.rs) |
| `array.rotate_left(k)` | Supported | 87 / 87 |  |  | [array_rotate_left.rs](library/array_rotate_left.rs) |
| `array.sort_unstable()` on four elements | Supported | 80 / 80 |  | core's sort is far bigger than 96 cells would allow; a small insertion sort written out fits. | [array_sort_unstable.rs](library/array_sort_unstable.rs) |
| `slice.split_at(k)` | Supported | 47 / 47 |  | It returns two slices, which the compiler doesn't have. | [array_split_at.rs](library/array_split_at.rs) |
| `array.swap(i, j)` | Supported | 74 / 74 |  | On an array it is two indexed reads and two writes, with bounds checks. | [array_swap.rs](library/array_swap.rs) |
| `array.windows(2)` | Supported | 62 / 73 |  | Each window is a slice in an `Option`, which the compiler doesn't have. | [array_windows.rs](library/array_windows.rs) |
| `slice.is_empty()` | Supported | 32 / 32 |  | A slice is an address and a length; the compiler has arrays of known length only. | [slice_is_empty.rs](library/slice_is_empty.rs) |
| `slice.len()` on a slice of run-time length | Supported | 21 / 21 |  | A slice is an address and a length; the compiler has arrays of known length only. | [slice_len.rs](library/slice_len.rs) |

### Iterators

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| `(a..b).chain(c..d)` | Supported | 49 / 49 |  |  | [iter_chain.rs](library/iter_chain.rs) |
| `t.iter().enumerate()` in an adaptor chain | Supported | 45 / 45 |  |  | [iter_enumerate_fold.rs](library/iter_enumerate_fold.rs) |
| `(a..b).filter(...).sum()` | Supported | 33 / 33 |  |  | [iter_filter.rs](library/iter_filter.rs) |
| `.map(...).max()` and `.min()` | Supported |  |  |  | [iter_map_max.rs](library/iter_map_max.rs) |
| `core::iter::once(x)` chained | Supported | 33 / 33 |  |  | [iter_once.rs](library/iter_once.rs) |
| `(a..b).count()` | Supported | 17 / 17 |  | On a range it is `b - a` (or 0). | [iter_range_count.rs](library/iter_range_count.rs) |
| `(a..b).fold(...)` | Supported | 31 / 47 |  |  | [iter_range_fold.rs](library/iter_range_fold.rs) |
| `(a..b).last()` | Supported | 16 / 16 |  | On a range it is `b - 1`, or `None` when the range is empty. | [iter_range_last.rs](library/iter_range_last.rs) |
| `(a..b).map(...).sum()` | Supported | 32 / 36 |  |  | [iter_range_map_sum.rs](library/iter_range_map_sum.rs) |
| `(a..b).nth(n)` | Supported | 32 / 32 |  | On a range it is `a + n`, or `None` past the end. | [iter_range_nth.rs](library/iter_range_nth.rs) |
| `core::iter::repeat(x).take(n)` | Supported | 34 / 34 |  |  | [iter_repeat.rs](library/iter_repeat.rs) |
| `t.iter().rev().fold(...)` | Supported | 27 / 52 |  |  | [iter_rev_fold.rs](library/iter_rev_fold.rs) |
| `(a..b).skip(n)` | Supported | 40 / 40 |  |  | [iter_skip.rs](library/iter_skip.rs) |
| `(a..b).step_by(k).sum()` | Supported | 62 / 62 |  |  | [iter_step_by_sum.rs](library/iter_step_by_sum.rs) |
| `core::iter::successors` | Supported | 26 / 38 |  |  | [iter_successors.rs](library/iter_successors.rs) |
| `(a..b).take(n)` | Supported | 41 / 41 |  |  | [iter_take.rs](library/iter_take.rs) |
| `t.iter().zip(u.iter())` | Supported | 59 / 60 |  |  | [iter_zip.rs](library/iter_zip.rs) |

### Comparison (cmp, Ordering)

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| `Ord::clamp` on `bool` | Supported | 37 / 37 |  | At opt-level 1 rustc calls `Ord::clamp` on `bool`, as it calls `max` and `min`. | [cmp_bool_clamp.rs](library/cmp_bool_clamp.rs) |
| `Ord::max` and `min` on `bool` | Supported | 39 / 39 |  | At opt-level 1 rustc calls `Ord::max` on `bool`; it is `a \|\| b` and `a && b`. | [cmp_bool_max_min.rs](library/cmp_bool_max_min.rs) |
| `match a.cmp(&b)` on `Ordering` | Supported | 27 / 27 |  | An `Ordering` is -1, 0 or 1 (an `i8`), and rustc's three-way compare is two tests, as `x == k` already is. | [cmp_match.rs](library/cmp_match.rs) |
| `a.cmp(&b) as i16` | Supported | 27 / 27 |  | An `Ordering` is -1, 0 or 1 (an `i8`), and rustc's three-way compare is two tests, as `x == k` already is. | [cmp_ordering_as_i16.rs](library/cmp_ordering_as_i16.rs) |
| `Ordering::is_lt` and `is_ge` | Supported | 41 / 41 |  | An `Ordering` is -1, 0 or 1 (an `i8`), and rustc's three-way compare is two tests, as `x == k` already is. | [cmp_ordering_is_lt.rs](library/cmp_ordering_is_lt.rs) |
| `Ordering::then` (compare by two keys) | Supported | 52 / 52 |  | An `Ordering` is -1, 0 or 1 (an `i8`); `then` is a test of the first for zero. | [cmp_ordering_then.rs](library/cmp_ordering_then.rs) |
| `PartialOrd::partial_cmp` | Supported | 37 / 37 |  | An `Option<Ordering>`: once `Ordering` is supported it is a small enum like any other. | [cmp_partial_cmp.rs](library/cmp_partial_cmp.rs) |
| `core::cmp::Reverse` | Supported | 28 / 28 |  | `Reverse` is a one-field struct from core whose comparisons swap their operands. | [cmp_reverse.rs](library/cmp_reverse.rs) |

### Memory (core::mem)

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| `core::mem::replace` | Supported | 12 / 12 |  | At opt-level 1 rustc calls it; on a scalar it is a read and a write. | [mem_replace.rs](library/mem_replace.rs) |
| `core::mem::swap` | Supported | 14 / 14 |  | On scalars it is two reads and two writes; rustc calls it at opt-level 1 and uses an intrinsic at opt-level 2. | [mem_swap.rs](library/mem_swap.rs) |
| `core::mem::take` | Supported | 11 / 11 |  | At opt-level 1 rustc calls it; on a scalar it is a read and a write of 0. | [mem_take.rs](library/mem_take.rs) |

### bool and char

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| `bool::then` | Supported | 19 / 30 |  | At opt-level 1 rustc calls it with a closure; it is `if b { Some(f()) } else { None }`. | [bool_then.rs](library/bool_then.rs) |
| `bool::then_some` | Supported | 16 / 16 |  | At opt-level 1 rustc calls it; it is `if b { Some(x) } else { None }`. | [bool_then_some.rs](library/bool_then_some.rs) |
| `==` and `<` on `char` | Supported | 50 / 50 |  | The compiler has no `char` type yet; a code point is 32 bits, but ASCII fits on a card. | [char_compare.rs](library/char_compare.rs) |
| `char::from_digit` | Supported | 32 / 32 |  | The compiler has no `char` type yet, and `from_digit` takes a `u32`. | [char_from_digit.rs](library/char_from_digit.rs) |
| `char::is_ascii_digit` | Supported | 44 / 44 |  | The compiler has no `char` type yet; a code point is 32 bits, but ASCII fits on a card. | [char_is_ascii_digit.rs](library/char_is_ascii_digit.rs) |
| `char::to_digit(10)` | Supported | 48 / 48 |  | The compiler has no `char` type yet, and `to_digit` returns an `Option<u32>`. | [char_to_digit.rs](library/char_to_digit.rs) |

### Hints (core::hint)

| Feature | Status | Cells | If not supported | Notes | Test |
|---|---|---|---|---|---|
| `core::hint::assert_unchecked` | Supported | 14 / 14 |  |  | [hint_assert_unchecked.rs](library/hint_assert_unchecked.rs) |
| `core::hint::black_box` | Supported | 7 / 18 |  | On CARDIAC it can simply be the value itself. | [hint_black_box.rs](library/hint_black_box.rs) |
| `core::hint::unreachable_unchecked` | Supported | 20 / 20 |  |  | [hint_unreachable_unchecked.rs](library/hint_unreachable_unchecked.rs) |
