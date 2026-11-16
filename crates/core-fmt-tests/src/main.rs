//@ mode: run
//@ exit: 0
//! Unit tests for `core::fmt` of the ekostd bootstrap core: every formatting option against the string Rust's own
//! `core` produces, written here as a literal.
//!
//! A test harness links std, so this is a plain `no_main` program built on the empty sysroot: it formats each case
//! into a fixed buffer, compares, prints one line per failure and a count, and exits 0 only when every case passed.
#![no_std]
#![no_main]

use core::fmt::{self, Debug, Formatter, Write};
use core::time::Duration;

unsafe extern "C" {
    fn write(fd: i32, buf: *const u8, count: usize) -> isize;
    fn abort() -> !;
}

fn out(text: &str) {
    let mut done = 0;
    while done < text.len() {
        let n = unsafe { write(1, text.as_ptr().add(done), text.len() - done) };
        if n <= 0 { return; }
        done += n as usize;
    }
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! {
    out("core-fmt-tests: panicked\n");
    unsafe { abort() }
}

/// Formatting target with a fixed capacity; running out of room is a formatting error, not a truncation.
struct Buf { bytes: [u8; 512], len: usize }
impl Buf {
    fn new() -> Self { Buf { bytes: [0; 512], len: 0 } }
    fn as_str(&self) -> &str {
        match core::str::from_utf8(&self.bytes[..self.len]) { Ok(s) => s, Err(_) => "<invalid UTF-8>" }
    }
}
impl Write for Buf {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let b = s.as_bytes();
        if b.len() > self.bytes.len() - self.len { return Err(fmt::Error); }
        let mut i = 0;
        while i < b.len() { self.bytes[self.len + i] = b[i]; i += 1; }
        self.len += b.len();
        Ok(())
    }
}

struct Run { passed: u32, failed: u32 }
impl Run {
    fn check(&mut self, line: u32, want: &str, args: fmt::Arguments<'_>) {
        let mut buf = Buf::new();
        let result = buf.write_fmt(args);
        if result.is_ok() && buf.as_str() == want {
            self.passed += 1;
            return;
        }
        self.failed += 1;
        let mut msg = Buf::new();
        let _ = write!(msg, "FAIL line {}: want {:?} got {:?}{}\n", line, want, buf.as_str(),
            if result.is_err() { " (fmt::Error)" } else { "" });
        out(msg.as_str());
    }
}

macro_rules! check {
    ($run:expr, $want:expr, $($arg:tt)+) => { $run.check(line!(), $want, format_args!($($arg)+)) };
}

#[derive(Debug)]
struct Point { x: i32, y: i32 }
#[derive(Debug)]
struct Unit;
#[derive(Debug)]
struct Pair(i32, &'static str);

/// Builder output other than what derive produces: maps, sets and the non-exhaustive forms.
enum Built { Map, MapNested, Set, SetRest, ListRest, ListRestEmpty, StructRest, StructRestEmpty, TupleRest, TupleRestEmpty, MapRest }
impl Debug for Built {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Built::Map => f.debug_map().entry(&1, &"a").entry(&2, &"b").finish(),
            Built::MapNested => f.debug_map().entry(&"k", &[1, 2]).finish(),
            Built::Set => f.debug_set().entry(&1).entry(&2).finish(),
            Built::SetRest => f.debug_set().entry(&1).finish_non_exhaustive(),
            Built::ListRest => f.debug_list().entry(&1).entry(&2).finish_non_exhaustive(),
            Built::ListRestEmpty => f.debug_list().finish_non_exhaustive(),
            Built::StructRest => f.debug_struct("S").field("a", &1).finish_non_exhaustive(),
            Built::StructRestEmpty => f.debug_struct("S").finish_non_exhaustive(),
            Built::TupleRest => f.debug_tuple("T").field(&1).finish_non_exhaustive(),
            Built::TupleRestEmpty => f.debug_tuple("T").finish_non_exhaustive(),
            Built::MapRest => f.debug_map().entry(&1, &2).finish_non_exhaustive(),
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn main(_argc: i32, _argv: *const *const u8) -> i32 {
    let mut r = Run { passed: 0, failed: 0 };

    // Width and default alignment: numbers right, text left.
    check!(r, "   42|", "{:5}|", 42);
    check!(r, "ab   |", "{:5}|", "ab");
    check!(r, "c    |", "{:5}|", 'c');
    check!(r, "true  |", "{:6}|", true);
    check!(r, "12345", "{:2}", 12345);
    check!(r, "abcdef", "{:3}", "abcdef");
    check!(r, "日本   |", "{:5}|", "日本");
    // Alignment.
    check!(r, "42   |", "{:<5}|", 42);
    check!(r, " 42  |", "{:^5}|", 42);
    check!(r, "   ab|", "{:>5}|", "ab");
    check!(r, "  ab  |", "{:^6}|", "ab");
    check!(r, "  ab   |", "{:^7}|", "ab");
    check!(r, "    a|", "{:>5}|", 'a');
    // Fill.
    check!(r, "***mid***", "{:*^9}", "mid");
    check!(r, "x-----", "{:-<6}", "x");
    check!(r, "###7", "{:#>4}", 7);
    check!(r, "éé1éé", "{:é^5}", 1);
    check!(r, "__-3", "{:_>4}", -3);
    // Width and precision from arguments.
    check!(r, "   7", "{:1$}", 7, 4);
    check!(r, "7   |", "{:<w$}|", 7, w = 4);
    check!(r, "1.23", "{:.*}", 2, 1.23456);
    check!(r, "3.1", "{:.p$}", 3.14159, p = 1);
    check!(r, "  1.2", "{:w$.p$}", 1.23, w = 5, p = 1);
    // Sign.
    check!(r, "+5", "{:+}", 5);
    check!(r, "-5", "{:+}", -5);
    check!(r, "+0", "{:+}", 0);
    check!(r, "   +5", "{:+5}", 5);
    check!(r, "+1.50", "{:+.2}", 1.5);
    check!(r, "+1.5", "{:+}", 1.5);
    check!(r, "-9223372036854775808", "{}", i64::MIN);
    check!(r, "340282366920938463463374607431768211455", "{}", u128::MAX);
    check!(r, "-128", "{}", i8::MIN);
    // Sign-aware zero padding: after the sign and prefix, alignment and fill ignored.
    check!(r, "00042", "{:05}", 42);
    check!(r, "-0042", "{:05}", -42);
    check!(r, "+0042", "{:+05}", 42);
    check!(r, "00042", "{:<05}", 42);
    check!(r, "0x00ff", "{:#06x}", 255);
    check!(r, "0b00000101", "{:#010b}", 5);
    check!(r, "-001.500", "{:08.3}", -1.5);
    check!(r, "-001.5", "{:06}", -1.5);
    check!(r, "ab   ", "{:05}", "ab");
    // Radix and the `#` prefix.
    check!(r, "ff", "{:x}", 255);
    check!(r, "FF", "{:X}", 255);
    check!(r, "0xff", "{:#x}", 255);
    check!(r, "0xFF", "{:#X}", 255);
    check!(r, "101", "{:b}", 5);
    check!(r, "0b101", "{:#b}", 5);
    check!(r, "10", "{:o}", 8);
    check!(r, "0o10", "{:#o}", 8);
    check!(r, "ff", "{:x}", -1i8);
    check!(r, "ffffffff", "{:x}", -1i32);
    check!(r, "BEE", "{:X}", 3054);
    check!(r, "        ff|", "{:10x}|", 255);
    check!(r, "0xff    |", "{:<#8x}|", 255);
    check!(r, "  0xff  |", "{:^#8x}|", 255);
    // Debug with hex flags.
    check!(r, "ff", "{:x?}", 255);
    check!(r, "FF", "{:X?}", 255);
    check!(r, "0xff", "{:#x?}", 255);
    check!(r, "[a, ff]", "{:x?}", [10, 255]);
    check!(r, "   42", "{:5?}", 42);
    check!(r, "+1", "{:+?}", 1);
    // Precision: text truncated by characters, then padded.
    check!(r, "he", "{:.2}", "hello");
    check!(r, "he   |", "{:5.2}|", "hello");
    check!(r, "    a", "{:>5.1}", "abc");
    check!(r, "日", "{:.1}", "日本");
    check!(r, "hi", "{:.5}", "hi");
    // Precision: floats, exact decimal value rounded half to even.
    check!(r, "3.14", "{:.2}", 3.14159);
    check!(r, "1.000", "{:.3}", 1.0);
    check!(r, "2", "{:.0}", 2.5);
    check!(r, "4", "{:.0}", 3.5);
    check!(r, "0.2", "{:.1}", 0.25);
    check!(r, "0.1000000000000000055511151231257827", "{:.34}", 0.1);
    check!(r, "   3.142", "{:8.3}", 3.14159);
    check!(r, "3.142   |", "{:<8.3}|", 3.14159);
    check!(r, "-0.00", "{:.2}", -0.0);
    check!(r, "1.50", "{:.2}", 1.5f32);
    // Exponent forms.
    check!(r, "1.2345e3", "{:e}", 1234.5);
    check!(r, "1.23e3", "{:.2e}", 1234.5);
    check!(r, "1.2E-4", "{:E}", 0.00012);
    check!(r, "    1.23e3", "{:10.2e}", 1234.5);
    check!(r, "1e0", "{:e}", 1.0);
    check!(r, "1.000e2", "{:.3e}", 100.0);
    check!(r, "1.234e3", "{:e}", 1234);
    check!(r, "1.2e3", "{:e}", 1200u32);
    check!(r, "1e2", "{:e}", 100u8);
    check!(r, "0e0", "{:e}", 0i64);
    check!(r, "-1.234E3", "{:E}", -1234i16);
    check!(r, "1.23400e3", "{:.5e}", 1234);
    check!(r, "2e3", "{:.0e}", 1500);
    check!(r, "2e3", "{:.0e}", 2500);
    check!(r, "1e2", "{:.0e}", 99);
    check!(r, "   1.234e3", "{:10e}", 1234);
    check!(r, "+5e0", "{:+e}", 5);
    check!(r, "3.40282366920938463463374607431768211455e38", "{:e}", u128::MAX);
    check!(r, "-1.70141183460469231731687303715884105728e38", "{:e}", i128::MIN);
    // Floats without precision.
    check!(r, "1.5", "{}", 1.5);
    check!(r, "-2.25", "{}", -2.25);
    check!(r, "100", "{}", 100.0);
    check!(r, "1.0", "{:?}", 1.0);
    check!(r, "-0", "{}", -0.0);
    check!(r, "-0.0", "{:?}", -0.0);
    check!(r, "  1.5", "{:5}", 1.5);
    check!(r, "1.5   |", "{:<6}|", 1.5);
    check!(r, "  inf", "{:5}", f64::INFINITY);
    check!(r, "-inf", "{}", f64::NEG_INFINITY);
    check!(r, "NaN", "{}", f64::NAN);
    check!(r, "NaN", "{:+}", f64::NAN);
    check!(r, "+inf", "{:+}", f64::INFINITY);
    check!(r, "0.30000000000000004", "{}", 0.1 + 0.2);
    check!(r, "0.1", "{}", 0.1);
    check!(r, "0.1", "{}", 0.1f32);
    check!(r, "0.3333333333333333", "{}", 1.0 / 3.0);
    check!(r, "0.33333334", "{}", 1.0f32 / 3.0);
    check!(r, "1000000000000000000000", "{}", 1e21);
    check!(r, "0.0000001", "{}", 1e-7);
    check!(r, "123456789012345680", "{}", 123456789012345678.0);
    check!(r, "16777216", "{}", 16777216f32);
    check!(r, "1000000000000000.0", "{:?}", 1e15);
    check!(r, "1e16", "{:?}", 1e16);
    check!(r, "0.0001", "{:?}", 0.0001);
    check!(r, "1e-5", "{:?}", 0.00001);
    check!(r, "1e-7", "{:?}", 1e-7);
    check!(r, "5e-324", "{:?}", 5e-324);
    check!(r, "1.7976931348623157e308", "{:?}", f64::MAX);
    check!(r, "3.4028235e38", "{:?}", f32::MAX);
    check!(r, "1.7976931348623157e308", "{:e}", f64::MAX);
    check!(r, "2.5e-8", "{:e}", 2.5e-8);
    // Pointers: always 0x lower hex; `#` zero-extends to the pointer width.
    let p = 0x1000 as *const u8;
    check!(r, "0x1000", "{:p}", p);
    check!(r, "0x0000000000001000", "{:#p}", p);
    check!(r, "    0x1000", "{:10p}", p);
    check!(r, "0x1000    |", "{:<10p}|", p);
    check!(r, "0x00001000", "{:010p}", p);
    check!(r, "0x1000", "{:?}", p);
    // Debug of text: quoted and escaped, width does not apply.
    check!(r, "\"a\\\"b\\n\"", "{:?}", "a\"b\n");
    check!(r, "'\\''", "{:?}", '\'');
    check!(r, "'\"'", "{:?}", '"');
    check!(r, "\"tab\\t\"", "{:?}", "tab\t");
    check!(r, "()", "{:?}", ());
    // Debug of composites.
    check!(r, "Some(1)", "{:?}", Some(1));
    check!(r, "None", "{:?}", None::<i32>);
    check!(r, "Ok(3)", "{:?}", Ok::<i32, ()>(3));
    check!(r, "Err(\"e\")", "{:?}", Err::<(), &str>("e"));
    check!(r, "(1,)", "{:?}", (1,));
    check!(r, "(1, \"a\")", "{:?}", (1, "a"));
    check!(r, "(1, 2, 3)", "{:?}", (1, 2, 3));
    check!(r, "[1, 2]", "{:?}", [1, 2]);
    check!(r, "[]", "{:?}", [0i32; 0]);
    check!(r, "Point { x: 1, y: -2 }", "{:?}", Point { x: 1, y: -2 });
    check!(r, "Unit", "{:?}", Unit);
    check!(r, "Pair(1, \"a\")", "{:?}", Pair(1, "a"));
    check!(r, "Some(  1)", "{:3?}", Some(1));
    check!(r, "{1: \"a\", 2: \"b\"}", "{:?}", Built::Map);
    check!(r, "{\"k\": [1, 2]}", "{:?}", Built::MapNested);
    check!(r, "{1, 2}", "{:?}", Built::Set);
    check!(r, "{1, ..}", "{:?}", Built::SetRest);
    check!(r, "[1, 2, ..]", "{:?}", Built::ListRest);
    check!(r, "[..]", "{:?}", Built::ListRestEmpty);
    check!(r, "S { a: 1, .. }", "{:?}", Built::StructRest);
    check!(r, "S { .. }", "{:?}", Built::StructRestEmpty);
    check!(r, "T(1, ..)", "{:?}", Built::TupleRest);
    check!(r, "T(..)", "{:?}", Built::TupleRestEmpty);
    check!(r, "{1: 2, ..}", "{:?}", Built::MapRest);
    // Pretty Debug (`{:#?}`): one field per line, indented four spaces per level, trailing commas.
    check!(r, "Some(\n    1,\n)", "{:#?}", Some(1));
    check!(r, "None", "{:#?}", None::<i32>);
    check!(r, "Ok(\n    3,\n)", "{:#?}", Ok::<i32, ()>(3));
    check!(r, "(\n    1,\n)", "{:#?}", (1,));
    check!(r, "(\n    1,\n    \"a\",\n)", "{:#?}", (1, "a"));
    check!(r, "[\n    1,\n    2,\n]", "{:#?}", [1, 2]);
    check!(r, "[]", "{:#?}", [0i32; 0]);
    check!(r, "Point {\n    x: 1,\n    y: -2,\n}", "{:#?}", Point { x: 1, y: -2 });
    check!(r, "Unit", "{:#?}", Unit);
    check!(r, "Pair(\n    1,\n    \"a\",\n)", "{:#?}", Pair(1, "a"));
    check!(r, "Some(\n    [\n        1,\n        2,\n    ],\n)", "{:#?}", Some([1, 2]));
    check!(r, "Some(\n    Point {\n        x: 1,\n        y: 2,\n    },\n)", "{:#?}", Some(Point { x: 1, y: 2 }));
    check!(r, "Some(\n        1,\n)", "{:#5?}", Some(1));
    check!(r, "Some(\n    0xff,\n)", "{:#x?}", Some(255));
    check!(r, "Some(\n    \"a\\nb\",\n)", "{:#?}", Some("a\nb"));
    check!(r, "{\n    1: \"a\",\n    2: \"b\",\n}", "{:#?}", Built::Map);
    check!(r, "{\n    \"k\": [\n        1,\n        2,\n    ],\n}", "{:#?}", Built::MapNested);
    check!(r, "{\n    1,\n    2,\n}", "{:#?}", Built::Set);
    check!(r, "{\n    1,\n    ..\n}", "{:#?}", Built::SetRest);
    check!(r, "[\n    1,\n    2,\n    ..\n]", "{:#?}", Built::ListRest);
    check!(r, "[..]", "{:#?}", Built::ListRestEmpty);
    check!(r, "S {\n    a: 1,\n    ..\n}", "{:#?}", Built::StructRest);
    check!(r, "S { .. }", "{:#?}", Built::StructRestEmpty);
    check!(r, "T(\n    1,\n    ..\n)", "{:#?}", Built::TupleRest);
    check!(r, "T(..)", "{:#?}", Built::TupleRestEmpty);
    check!(r, "{\n    1: 2,\n    ..\n}", "{:#?}", Built::MapRest);
    // Arguments: positional and named reuse, escaped braces.
    check!(r, "b a b", "{1} {0} {1}", "a", "b");
    check!(r, "{x}", "{{x}}");
    check!(r, "v=3 v=3", "v={v} v={v}", v = 3);
    // Float parsing: correctly rounded, f32 rounded once.
    check!(r, "4340000000000001", "{:x}", "9007199254740993.0000000001".parse::<f64>().unwrap().to_bits());
    check!(r, "3f800001", "{:x}", "1.00000005960464477550".parse::<f32>().unwrap().to_bits());
    check!(r, "3fb999999999999a", "{:x}", "0.1".parse::<f64>().unwrap().to_bits());
    check!(r, "44b52d02c7e14af6", "{:x}", "1e23".parse::<f64>().unwrap().to_bits());
    check!(r, "1", "{:x}", "4.9e-324".parse::<f64>().unwrap().to_bits());
    check!(r, "fffffffffffff", "{:x}", "2.2250738585072011e-308".parse::<f64>().unwrap().to_bits());
    check!(r, "7ff0000000000000", "{:x}", "1e400".parse::<f64>().unwrap().to_bits());
    check!(r, "0", "{:x}", "1e-400".parse::<f64>().unwrap().to_bits());
    check!(r, "3dcccccd", "{:x}", "0.1".parse::<f32>().unwrap().to_bits());
    check!(r, "1.5s", "{:?}", Duration::from_millis(1500));
    check!(r, "1s", "{:?}", Duration::from_secs(1));
    check!(r, "1ms", "{:?}", Duration::from_millis(1));
    check!(r, "1µs", "{:?}", Duration::from_micros(1));
    check!(r, "1ns", "{:?}", Duration::from_nanos(1));
    check!(r, "0ns", "{:?}", Duration::from_nanos(0));
    check!(r, "1.000001ms", "{:?}", Duration::from_nanos(1_000_001));
    check!(r, "1.23s", "{:.2?}", Duration::from_millis(1234));
    check!(r, "2s", "{:.0?}", Duration::from_millis(1500));
    check!(r, "1.500s", "{:.3?}", Duration::from_millis(1500));
    check!(r, "+1.5s", "{:+?}", Duration::from_millis(1500));
    check!(r, "1.5s  |", "{:6?}|", Duration::from_millis(1500));
    check!(r, "'\\u{80}'", "{:?}", '\u{80}');
    check!(r, "\"\\u{200b}\"", "{:?}", "\u{200b}");
    check!(r, "\"\\u{301}\"", "{:?}", "\u{301}");
    check!(r, "'\"'", "{:?}", '"');
    check!(r, "\"'\"", "{:?}", "'");
    check!(r, "'\\''", "{:?}", '\'');
    check!(r, "\"\\u{7f}\"", "{:?}", "\u{7f}");
    check!(r, "\"é\"", "{:?}", "é");

    { // lw2-a: exact length of integer ranges (Range and RangeInclusive size_hint / len)
        check!(r, "(200, Some(200))", "{:?}", (-100i8..100).size_hint());
        check!(r, "255", "{}", (i8::MIN..i8::MAX).len());
        check!(r, "(0, Some(0))", "{:?}", (5u32..3).size_hint());
        check!(r, "(18446744073709551615, Some(18446744073709551615))", "{:?}", (i64::MIN..i64::MAX).size_hint());
        check!(r, "(18446744073709551615, None)", "{:?}", (0u128..u128::MAX).size_hint());
        check!(r, "5", "{}", (1u8..=5).len());
        check!(r, "(256, Some(256))", "{:?}", (0u8..=255).size_hint());
        check!(r, "256", "{}", (-128i8..=127).len());
        check!(r, "(0, Some(0))", "{:?}", (3i16..=2).size_hint());
        check!(r, "(18446744073709551615, None)", "{:?}", (0u64..=u64::MAX).size_hint());
        let mut done = 1u8..=2;
        let _ = done.next();
        let _ = done.next();
        check!(r, "(0, Some(0))", "{:?}", done.size_hint());
        let mut back = 1u8..=3;
        let _ = back.next_back();
        check!(r, "2", "{}", back.len());
    }
    { // lw2-b: range contains honours exhaustion, is_empty, inherent contains on every range
        check!(r, "true", "{}", (1..=3).contains(&3));
        let mut drained = 1..=3;
        while drained.next().is_some() {}
        check!(r, "false", "{}", drained.contains(&3));
        check!(r, "false", "{}", drained.contains(&1));
        check!(r, "true", "{}", drained.is_empty());
        let mut single = 1..=1;
        let _ = single.next();
        check!(r, "false", "{}", single.contains(&1));
        check!(r, "true", "{}", single.is_empty());
        check!(r, "true", "{}", (3..=1).is_empty());
        check!(r, "true", "{}", (1..1).is_empty());
        check!(r, "false", "{}", (1..2).is_empty());
        check!(r, "true", "{}", (0.0..1.0).contains(&0.5));
        check!(r, "false", "{}", (0.0..1.0).contains(&f64::NAN));
        check!(r, "true", "{}", (..5).contains(&4));
        check!(r, "true", "{}", (..=5).contains(&5));
        check!(r, "false", "{}", (5..).contains(&4));
        check!(r, "false", "{}", (1..3).contains(&3));
    }
    { // lw2-c: str::trim* strip Unicode White_Space, str::trim_ascii* strip ASCII whitespace only
        check!(r, "[a b]", "[{}]", "\u{a0} a b\u{3000}".trim());
        check!(r, "[x ]", "[{}]", "\x0B\x0Cx ".trim_start());
        check!(r, "[x]", "[{}]", "x\u{85}\u{2028}".trim_end());
        check!(r, "[]", "[{}]", " \t\n".trim());
        check!(r, "[日本]", "[{}]", "日本 \u{2003}".trim_end());
        check!(r, "[\u{200b}x]", "[{}]", "\u{200b}x".trim());
        check!(r, "[y]", "[{}]", "\u{1680}\u{205f}y\u{202f}".trim());
        check!(r, "[\u{a0}x]", "[{}]", "\u{a0}x ".trim_ascii());
        check!(r, "[x\t]", "[{}]", " \x0Cx\t".trim_ascii_start());
        check!(r, "[x]", "[{}]", "x \n".trim_ascii_end());
    }
    { // lw2-d: char::is_ascii_whitespace is the WhatWG set, without vertical tab
        check!(r, "false", "{}", '\x0B'.is_ascii_whitespace());
        check!(r, "true", "{}", '\x0C'.is_ascii_whitespace());
        check!(r, "true", "{}", ' '.is_ascii_whitespace());
        check!(r, "true", "{}", '\t'.is_ascii_whitespace());
        check!(r, "true", "{}", '\n'.is_ascii_whitespace());
        check!(r, "true", "{}", '\r'.is_ascii_whitespace());
        check!(r, "false", "{}", '\u{a0}'.is_ascii_whitespace());
        check!(r, "false", "{}", 'a'.is_ascii_whitespace());
        check!(r, "false", "{}", b'\x0B'.is_ascii_whitespace());
    }
    { // lw2-e: iter::Skip size_hint, len, next_back, nth
        let a = [1i32, 2, 3, 4];
        check!(r, "3", "{}", a.iter().skip(1).len());
        check!(r, "0", "{}", a.iter().skip(10).len());
        check!(r, "(2, Some(2))", "{:?}", a.iter().skip(2).size_hint());
        check!(r, "432", "{}", a.iter().skip(1).rev().fold(0, |acc, &x| acc * 10 + x));
        check!(r, "None", "{:?}", a.iter().skip(4).next_back());
        check!(r, "Some(3)", "{:?}", a.iter().skip(1).nth(1));
        check!(r, "None", "{:?}", a.iter().skip(1).nth(5));
        let mut s = a.iter().skip(1);
        check!(r, "Some(4)", "{:?}", s.next_back());
        check!(r, "Some(2)", "{:?}", s.next());
        check!(r, "Some(3)", "{:?}", s.next_back());
        check!(r, "None", "{:?}", s.next());
        check!(r, "(18446744073709551612, None)", "{:?}", (0..).skip(3).size_hint());
    }
    { // lw2-f: Peekable size_hint, len, next_back, peek_mut, next_if_eq, count, last, nth
        let mut p = [1, 2, 3].iter().peekable();
        let _ = p.peek();
        check!(r, "(3, Some(3))", "{:?}", p.size_hint());
        check!(r, "3", "{}", p.len());
        check!(r, "Some(3)", "{:?}", p.next_back());
        check!(r, "Some(2)", "{:?}", p.next_back());
        check!(r, "Some(1)", "{:?}", p.next_back());
        check!(r, "None", "{:?}", p.next_back());

        let mut q = [1, 2].into_iter().peekable();
        if let Some(x) = q.peek_mut() { *x = 10; }
        check!(r, "Some(10)", "{:?}", q.next());

        let mut s = [1, 2, 3].into_iter().peekable();
        check!(r, "Some(1)", "{:?}", s.next_if_eq(&1));
        check!(r, "None", "{:?}", s.next_if_eq(&5));
        check!(r, "Some(2)", "{:?}", s.next());

        let mut c = [4, 5, 6].iter().peekable();
        let _ = c.peek();
        check!(r, "3", "{}", c.count());
        let mut l = [4, 5, 6].iter().peekable();
        let _ = l.peek();
        check!(r, "Some(6)", "{:?}", l.last());
        let mut n = [7].iter().peekable();
        let _ = n.peek();
        check!(r, "Some(7)", "{:?}", n.nth(0));

        let mut e = [0u8; 0].iter().peekable();
        check!(r, "None", "{:?}", e.peek());
        check!(r, "(0, Some(0))", "{:?}", e.size_hint());
    }

    { // lw2-g: FlatMap from the back, FlatMap and Flatten size_hint
        check!(r, "21201110", "{}", [1, 2].iter().flat_map(|&x| x * 10..x * 10 + 2).rev().fold(0, |a, x| a * 100 + x));
        let mut f = [1, 2, 3].iter().flat_map(|&x| x * 10..x * 10 + 2);
        check!(r, "Some(10)", "{:?}", f.next());
        check!(r, "Some(31)", "{:?}", f.next_back());
        check!(r, "Some(30)", "{:?}", f.next_back());
        check!(r, "Some(11)", "{:?}", f.next());
        check!(r, "Some(20)", "{:?}", f.next());
        check!(r, "Some(21)", "{:?}", f.next_back());
        check!(r, "None", "{:?}", f.next());
        check!(r, "None", "{:?}", f.next_back());
        check!(r, "(0, None)", "{:?}", [1u8].iter().flat_map(|&x| 0..x).size_hint());
        let mut h = [1, 2].iter().flat_map(|&x| 0..x);
        h.next();
        h.next();
        check!(r, "(1, Some(1))", "{:?}", h.size_hint());
        check!(r, "6510", "{}", [0..2, 5..7].iter().cloned().flatten().rev().fold(0, |a, x| a * 10 + x));
        let mut fl = [0..3].into_iter().flatten();
        fl.next();
        check!(r, "(2, Some(2))", "{:?}", fl.size_hint());
    }
    { // lw2-h: StepBy and Chain size_hint, StepBy len
        check!(r, "(4, Some(4))", "{:?}", (0..10).step_by(3).size_hint());
        check!(r, "4", "{}", (0..10).step_by(3).len());
        check!(r, "3", "{}", (0..9).step_by(3).len());
        check!(r, "0", "{}", (0..0).step_by(2).len());
        let mut s = (0..10).step_by(3);
        s.next();
        check!(r, "(3, Some(3))", "{:?}", s.size_hint());
        check!(r, "1", "{}", (0..1).step_by(5).len());
        check!(r, "(5, Some(5))", "{:?}", (0..3).chain(5..7).size_hint());
        check!(r, "(18446744073709551615, None)", "{:?}", (0..3).chain(0u32..).size_hint());
        let mut c = (0..3).chain(5..7);
        c.next();
        c.next();
        c.next();
        check!(r, "(2, Some(2))", "{:?}", c.size_hint());
        check!(r, "(0, Some(0))", "{:?}", (0..0).chain(0..0).size_hint());
    }
    { // lw4-a: char API: ASCII classes, from_digit, UTF-16, escape_unicode/escape_default, make_ascii_*, constants, str escapes
        check!(r, "true false", "{} {}", 'a'.is_ascii_lowercase(), 'A'.is_ascii_lowercase());
        check!(r, "true false", "{} {}", '~'.is_ascii_graphic(), ' '.is_ascii_graphic());
        check!(r, "true false", "{} {}", '\x7f'.is_ascii_control(), '\u{80}'.is_ascii_control());
        check!(r, "true false", "{} {}", '7'.is_ascii_octdigit(), '8'.is_ascii_octdigit());
        check!(r, "Some('b')", "{:?}", char::from_digit(11, 16));
        check!(r, "None", "{:?}", char::from_digit(10, 10));
        check!(r, "Some('z')", "{:?}", core::char::from_digit(35, 36));
        check!(r, "1 2", "{} {}", 'ß'.len_utf16(), '\u{1D11E}'.len_utf16());
        let mut units = [0u16; 2];
        let encoded = '\u{1D11E}'.encode_utf16(&mut units);
        check!(r, "2 d834 dd1e", "{} {:x} {:x}", encoded.len(), encoded[0], encoded[1]);
        check!(r, "[\\n]", "[{}]", '\n'.escape_default());
        check!(r, "[\\\"]", "[{}]", '"'.escape_default());
        check!(r, "[\\']", "[{}]", '\''.escape_default());
        check!(r, "[\\u{7f}]", "[{}]", '\u{7f}'.escape_default());
        check!(r, "[\\u{e9}]", "[{}]", 'é'.escape_default());
        check!(r, "[a]", "[{}]", 'a'.escape_default());
        check!(r, "2", "{}", '\n'.escape_default().len());
        check!(r, "[\\u{61}]", "[{}]", 'a'.escape_unicode());
        check!(r, "[\\u{0}]", "[{}]", '\0'.escape_unicode());
        check!(r, "[\\u{10ffff}]", "[{}]", '\u{10ffff}'.escape_unicode());
        check!(r, "6", "{}", 'a'.escape_unicode().len());
        let mut q = 'q';
        q.make_ascii_uppercase();
        let mut e = 'é';
        e.make_ascii_uppercase();
        check!(r, "Q é", "{} {}", q, e);
        check!(r, "65533 0", "{} {}", char::REPLACEMENT_CHARACTER as u32, char::MIN as u32);
        check!(r, "[\\u{61}\\u{62}\\u{a}]", "[{}]", "ab\n".escape_unicode());
        check!(r, "[a\\'b\\\"\\n]", "[{}]", "a'b\"\n".escape_debug());
        check!(r, "[\\u{301}x\u{301}]", "[{}]", "\u{301}x\u{301}".escape_debug());
    }
    { // lw4-b: str API rsplit_terminator, split_at_checked/split_at_mut, from_utf8_mut, make_ascii_*, floor/ceil_char_boundary, encode_utf16
        fn join<'a, I: Iterator<Item = &'a str>>(it: I) -> Buf {
            let mut b = Buf::new();
            let mut first = true;
            for piece in it {
                if !first { let _ = b.write_str("|"); }
                first = false;
                let _ = b.write_str(piece);
            }
            b
        }
        let j = join("A.B.".rsplit_terminator('.'));
        check!(r, "B|A", "{}", j.as_str());
        let j = join("A..B..".rsplit_terminator("."));
        check!(r, "|B||A", "{}", j.as_str());
        let j = join("x".rsplit_terminator(','));
        check!(r, "x", "{}", j.as_str());
        check!(r, "0", "{}", "".rsplit_terminator('.').count());
        check!(r, "None", "{:?}", "héllo".split_at_checked(2));
        check!(r, "Some((\"hé\", \"llo\"))", "{:?}", "héllo".split_at_checked(3));
        check!(r, "None", "{:?}", "héllo".split_at_checked(9));
        let mut a = *b"Hello";
        match core::str::from_utf8_mut(&mut a) {
            Ok(s) => { s.make_ascii_lowercase(); check!(r, "hello", "{}", s); }
            Err(_) => check!(r, "hello", "{}", "<invalid UTF-8>"),
        }
        let mut e = [b'h', 0xc3, 0xa9, b'l', b'l', b'o'];
        match core::str::from_utf8_mut(&mut e) {
            Ok(s) => { s.make_ascii_uppercase(); check!(r, "HéLLO", "{}", s); }
            Err(_) => check!(r, "HéLLO", "{}", "<invalid UTF-8>"),
        }
        let mut c = *b"hello";
        match core::str::from_utf8_mut(&mut c) {
            Ok(s) => {
                let (left, _) = s.split_at_mut(2);
                left.make_ascii_uppercase();
                check!(r, "HEllo", "{}", s);
            }
            Err(_) => check!(r, "HEllo", "{}", "<invalid UTF-8>"),
        }
        check!(r, "true", "{}", core::str::from_utf8_mut(&mut [0xffu8]).is_err());
        check!(r, "1", "{}", "héllo".floor_char_boundary(2));
        check!(r, "3", "{}", "héllo".ceil_char_boundary(2));
        check!(r, "6", "{}", "héllo".floor_char_boundary(99));
        check!(r, "6", "{}", "héllo".ceil_char_boundary(99));
        check!(r, "0", "{}", "héllo".ceil_char_boundary(0));
        let mut h = Buf::new();
        let mut first = true;
        for u in "a\u{1D11E}".encode_utf16() {
            if !first { let _ = h.write_str(" "); }
            first = false;
            let _ = write!(h, "{:x}", u);
        }
        check!(r, "61 d834 dd1e", "{}", h.as_str());
        check!(r, "3", "{}", "a\u{1D11E}".encode_utf16().count());
    }
    { // lw4-c: slice split family, is_sorted*, fill_with, first_chunk/last_chunk/split_last_chunk
        fn join<'a, I: Iterator<Item = &'a [i32]>>(pieces: I) -> Buf {
            let mut b = Buf::new();
            let mut first = true;
            for piece in pieces {
                if !first { let _ = b.write_str(" "); }
                first = false;
                let _ = write!(b, "{:?}", piece);
            }
            b
        }
        check!(r, "true", "{}", [1, 2, 3].is_sorted());
        check!(r, "false", "{}", [1, 3, 2].is_sorted());
        check!(r, "true", "{}", [0u8; 0].is_sorted());
        check!(r, "true", "{}", [3, 2, 1].is_sorted_by(|a, b| a >= b));
        check!(r, "false", "{}", ["ccc", "a", "bb"].is_sorted_by_key(|s| s.len()));
        let v = [10, 0, 20, 0, 30];
        check!(r, "[30] [20] [10]", "{}", join(v.rsplit(|x| *x == 0)).as_str());
        check!(r, "[10] [20, 0, 30]", "{}", join(v.splitn(2, |x| *x == 0)).as_str());
        check!(r, "[30] [10, 0, 20]", "{}", join(v.rsplitn(2, |x| *x == 0)).as_str());
        check!(r, "0", "{}", v.splitn(0, |x| *x == 0).count());
        check!(r, "[10, 0] [20, 0] [30]", "{}", join(v.split_inclusive(|x| *x == 0)).as_str());
        check!(r, "[0] [0]", "{}", join([0, 0].split_inclusive(|x| *x == 0)).as_str());
        check!(r, "[30] [20, 0] [10, 0]", "{}", join(v.split_inclusive(|x| *x == 0).rev()).as_str());
        let mut w = v;
        for piece in w.split_mut(|x| *x == 0) {
            if let Some(first) = piece.first_mut() { *first += 1; }
        }
        check!(r, "[11, 0, 21, 0, 31]", "{:?}", w);
        let mut counter = 0;
        let mut filled = [0; 4];
        filled.fill_with(|| { counter += 1; counter });
        check!(r, "[1, 2, 3, 4]", "{:?}", filled);
        let a = [1, 2, 3];
        check!(r, "Some([1, 2])", "{:?}", a.first_chunk::<2>());
        check!(r, "Some([2, 3])", "{:?}", a.last_chunk::<2>());
        check!(r, "None", "{:?}", a.first_chunk::<4>());
        check!(r, "Some(([1, 2], [3]))", "{:?}", a.split_last_chunk::<1>());
    }
    { // lw4-d: select_nth / disjoint / ascii
        let mut v = [5, 1, 4, 2, 3];
        let (lo, mid, hi) = v.select_nth_unstable(2);
        let mut parted = true;
        for x in lo.iter() { if *x > 3 { parted = false; } }
        for x in hi.iter() { if *x < 3 { parted = false; } }
        check!(r, "3 true 2 2", "{} {} {} {}", *mid, parted, lo.len(), hi.len());
        let mut w = [1, 9, 3];
        let (_, top, _) = w.select_nth_unstable_by(0, |a, b| b.cmp(a));
        check!(r, "9", "{}", *top);
        let mut k = [1, 2, 3, 4, 5];
        let (_, least, _) = k.select_nth_unstable_by_key(4, |x: &i32| -x);
        check!(r, "1", "{}", *least);

        let mut a = [0i32; 200];
        let mut seed: u32 = 12345;
        let mut i = 0;
        while i < 200 {
            seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
            a[i] = ((seed >> 16) % 1000) as i32;
            i += 1;
        }
        let mut sorted = a;
        sorted.sort_unstable_by(|x, y| x.cmp(y));
        let mut same = true;
        for &at in [0usize, 57, 199].iter() {
            let mut c = a;
            let (before, nth, after) = c.select_nth_unstable(at);
            let pivot = *nth;
            if pivot != sorted[at] || before.len() != at { same = false; }
            for x in before.iter() { if *x > pivot { same = false; } }
            for x in after.iter() { if *x < pivot { same = false; } }
        }
        check!(r, "true", "{}", same);

        let mut d = [1, 2, 3, 4];
        if let Ok([x, y]) = d.get_disjoint_mut([0, 2]) {
            let t = *x;
            *x = *y;
            *y = t;
        }
        check!(r, "[3, 2, 1, 4]", "{:?}", d);
        match d.get_disjoint_mut([1, 1]) {
            Err(e) => check!(r, "Err(OverlappingIndices) there were overlapping indices", "Err({:?}) {}", e, e),
            Ok(_) => check!(r, "Err(OverlappingIndices) there were overlapping indices", "Ok"),
        }
        match d.get_disjoint_mut([4]) {
            Err(e) => check!(r, "Err(IndexOutOfBounds) an index is out of bounds", "Err({:?}) {}", e, e),
            Ok(_) => check!(r, "Err(IndexOutOfBounds) an index is out of bounds", "Ok"),
        }

        let mut h = *b"Hello, World";
        h.make_ascii_uppercase();
        check!(r, "HELLO, WORLD", "{}", match core::str::from_utf8(&h) { Ok(s) => s, Err(_) => "<invalid UTF-8>" });
        let mut g = [0u8; 3];
        g.copy_from_slice("ÀB".as_bytes());
        g.make_ascii_lowercase();
        check!(r, "Àb", "{}", match core::str::from_utf8(&g) { Ok(s) => s, Err(_) => "<invalid UTF-8>" });
        check!(r, "true false", "{} {}", b"Ferris".eq_ignore_ascii_case(b"FERRIS"), b"a".eq_ignore_ascii_case(b"ab"));

        let raw = b"0\t\r\n'\"\\\x9d";
        check!(r, "0\\t\\r\\n\\'\\\"\\\\\\x9d", "{}", raw.escape_ascii());
        check!(r, "17", "{}", raw.escape_ascii().count());
        let mut back = raw.escape_ascii();
        check!(r, "Some(100) Some(57) Some(48)", "{:?} {:?} {:?}", back.next_back(), back.next_back(), back.next());
        check!(r, "EscapeAscii { .. }", "{:?}", raw.escape_ascii());
    }
    { // lw4-f: cell API (Cell, RefCell, Ref, RefMut, OnceCell)
        let a = core::cell::Cell::new(1);
        let b = core::cell::Cell::new(2);
        a.swap(&b);
        check!(r, "(2, 1)", "{:?}", (a.get(), b.get()));
        a.swap(&a);
        check!(r, "2", "{}", a.get());
        let c = core::cell::Cell::new(4);
        c.update(|v| v * 10);
        check!(r, "40", "{}", c.get());
        check!(r, "7", "{}", core::cell::Cell::new(7).into_inner());
        let mut x = 1;
        core::cell::Cell::from_mut(&mut x).set(6);
        check!(r, "6", "{}", x);
        let mut arr = [1, 2];
        core::cell::Cell::from_mut(&mut arr[..]).as_slice_of_cells()[1].set(9);
        check!(r, "[1, 9]", "{:?}", arr);
        let mut arr2 = [3, 4];
        core::cell::Cell::from_mut(&mut arr2).as_array_of_cells()[0].set(5);
        check!(r, "[5, 4]", "{:?}", arr2);
        check!(r, "true", "{}", core::cell::Cell::new(1) < core::cell::Cell::new(2));
        check!(r, "true", "{}", core::cell::Cell::new(3) == core::cell::Cell::new(3));
        check!(r, "true", "{}", core::cell::Cell::new(5).cmp(&core::cell::Cell::new(2)) == core::cmp::Ordering::Greater);
        check!(r, "Cell { value: 5 }", "{:?}", core::cell::Cell::new(5));
        check!(r, "8", "{}", core::cell::Cell::new(8).clone().get());
        check!(r, "3", "{}", core::cell::Cell::from(3).get());

        let rc = core::cell::RefCell::new(1);
        check!(r, "1", "{}", rc.replace_with(|v| *v + 1));
        check!(r, "2", "{}", *rc.borrow());
        let r1 = core::cell::RefCell::new(1);
        let r2 = core::cell::RefCell::new(2);
        r1.swap(&r2);
        let swapped = (*r1.borrow(), *r2.borrow());
        check!(r, "(2, 1)", "{:?}", swapped);
        check!(r, "true", "{}", core::cell::RefCell::new(1) < core::cell::RefCell::new(2));
        check!(r, "true", "{}", core::cell::RefCell::new(4) == core::cell::RefCell::from(4));

        let p = core::cell::RefCell::new((3, 4));
        {
            let g = p.borrow();
            match p.try_borrow_mut() {
                Ok(_) => { check!(r, "already borrowed", "{}", "Ok"); }
                Err(e) => { check!(r, "already borrowed", "{}", e); }
            }
            core::mem::drop(g);
        }
        {
            let m = p.borrow_mut();
            match p.try_borrow() {
                Ok(_) => { check!(r, "already mutably borrowed", "{}", "Ok"); }
                Err(e) => { check!(r, "already mutably borrowed", "{}", e); }
            }
            core::mem::drop(m);
        }
        check!(r, "3", "{}", *core::cell::Ref::map(p.borrow(), |q| &q.0));
        {
            let g1 = p.borrow();
            let g2 = core::cell::Ref::clone(&g1);
            core::mem::drop(g1);
            check!(r, "true", "{}", p.try_borrow_mut().is_err());
            check!(r, "(3, 4)", "{:?}", *g2);
            core::mem::drop(g2);
            check!(r, "true", "{}", p.try_borrow_mut().is_ok());
        }
        {
            let (mut h0, mut h1) = core::cell::RefMut::map_split(p.borrow_mut(), |q| (&mut q.0, &mut q.1));
            *h0 = 10;
            *h1 = 20;
            core::mem::drop(h0);
            check!(r, "true", "{}", p.try_borrow().is_err());
            check!(r, "true", "{}", p.try_borrow_mut().is_err());
            core::mem::drop(h1);
            check!(r, "true", "{}", p.try_borrow().is_ok());
            check!(r, "true", "{}", p.try_borrow_mut().is_ok());
        }
        check!(r, "(10, 20)", "{:?}", *p.borrow());
        check!(r, "true", "{}", core::cell::Ref::filter_map(p.borrow(), |q| if q.0 > 0 { Some(&q.0) } else { None }).is_ok());
        check!(r, "true", "{}", core::cell::Ref::filter_map(p.borrow(), |q| if q.0 < 0 { Some(&q.0) } else { None }).is_err());
        check!(r, "true", "{}", p.try_borrow_mut().is_ok());
        {
            let mut m = core::cell::RefMut::map(p.borrow_mut(), |q| &mut q.1);
            *m += 1;
            check!(r, "21", "{}", m);
            check!(r, "21", "{:?}", m);
        }
        check!(r, "21", "{}", p.borrow().1);
        let s = core::cell::RefCell::new(7);
        check!(r, "7", "{}", s.borrow());

        let mut o: core::cell::OnceCell<i32> = core::cell::OnceCell::new();
        check!(r, "None", "{:?}", o.get_mut());
        check!(r, "OnceCell(<uninit>)", "{:?}", o);
        let _ = o.set(3);
        check!(r, "Some(3)", "{:?}", o.get_mut());
        check!(r, "OnceCell(3)", "{:?}", o);
        check!(r, "true", "{}", o.clone() == o);
        check!(r, "Some(3)", "{:?}", o.take());
        check!(r, "None", "{:?}", o.get());
        check!(r, "Some(4)", "{:?}", core::cell::OnceCell::from(4).into_inner());
    }
    { // lw4-g: hash impls for 1-tuples, 9..12-tuples, Ordering, Bound, ranges, Wrapping, ManuallyDrop, Pin, fmt::Error, Layout
        struct Rec { b: [u8; 64], n: usize }
        impl core::hash::Hasher for Rec {
            fn finish(&self) -> u64 { self.n as u64 }
            fn write(&mut self, bytes: &[u8]) {
                for &x in bytes {
                    if self.n < 64 { self.b[self.n] = x; self.n += 1; }
                }
            }
        }
        fn bytes<T: core::hash::Hash + ?Sized>(t: &T) -> (usize, [u8; 64]) {
            let mut rec = Rec { b: [0; 64], n: 0 };
            t.hash(&mut rec);
            (rec.n, rec.b)
        }
        macro_rules! hb {
            ($want:expr, $v:expr) => {{
                let (n, b) = bytes(&$v);
                check!(r, $want, "{:?}", &b[..n]);
            }};
        }
        hb!("[7]", (7u8,));
        hb!("[1, 2, 3, 4, 5, 6, 7, 8, 9]", (1u8, 2u8, 3u8, 4u8, 5u8, 6u8, 7u8, 8u8, 9u8));
        hb!("[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]",
            (1u8, 2u8, 3u8, 4u8, 5u8, 6u8, 7u8, 8u8, 9u8, 10u8, 11u8, 12u8));
        hb!("[255]", core::cmp::Ordering::Less);
        hb!("[1]", core::cmp::Ordering::Greater);
        hb!("[0, 0, 0, 0, 0, 0, 0, 0, 7]", core::ops::Bound::Included(7u8));
        hb!("[2, 0, 0, 0, 0, 0, 0, 0]", core::ops::Bound::<u8>::Unbounded);
        hb!("[1, 3]", 1u8..3u8);
        hb!("[5]", 5u8..);
        hb!("[4]", ..4u8);
        hb!("[4]", ..=4u8);
        hb!("[]", ..);
        hb!("[9]", core::num::Wrapping(9u8));
        hb!("[3]", core::mem::ManuallyDrop::new(3u8));
        hb!("[6]", core::pin::Pin::new(&6u8));
        hb!("[]", core::fmt::Error);
        hb!("[4, 0, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0]", core::alloc::Layout::new::<u32>());
    }
    { // lanes-w3 23: Utf8Error Display (Rust wording)
        match core::str::from_utf8(&[0x61, 0xff]) {
            Err(e) => check!(r, "invalid utf-8 sequence of 1 bytes from index 1", "{}", e),
            Ok(_) => check!(r, "invalid utf-8 sequence of 1 bytes from index 1", "{}", "Ok"),
        }
        match core::str::from_utf8(&[0xe2, 0x28, 0xa1]) {
            Err(e) => check!(r, "invalid utf-8 sequence of 1 bytes from index 0", "{}", e),
            Ok(_) => check!(r, "invalid utf-8 sequence of 1 bytes from index 0", "{}", "Ok"),
        }
        match core::str::from_utf8(&[0x61, 0xe2, 0x82]) {
            Err(e) => check!(r, "incomplete utf-8 byte sequence from index 1", "{}", e),
            Ok(_) => check!(r, "incomplete utf-8 byte sequence from index 1", "{}", "Ok"),
        }
    }
    { // LANES-W4 ref-neg: unary operators on references to primitives
        let a = 5i32;
        let b = -2.5f64;
        let c = 7i8;
        let d = 0.0f32;
        check!(r, "-5", "{}", -&a);
        check!(r, "2.5", "{}", -&b);
        check!(r, "-7", "{}", -&c);
        check!(r, "-0", "{}", -&d);
        check!(r, "false", "{}", !&true);
        check!(r, "255", "{}", !&0u8);
    }
    { // lw5-d: array API: each_ref/each_mut, by-value IntoIter traits, as_flattened, slice equality, tuple conversions
        let a = [1, 2, 3];
        let refs = a.each_ref();
        check!(r, "6", "{}", *refs[0] + *refs[1] + *refs[2]);
        let mut m = [1, 2, 3];
        for x in m.each_mut() { *x += 1; }
        check!(r, "[2, 3, 4]", "{:?}", m);
        check!(r, "[2, 3, 4]", "{:?}", m.as_mut_slice());

        let mut rev = Buf::new();
        for x in [1, 2, 3].into_iter().rev() { let _ = write!(rev, "{} ", x); }
        check!(r, "3 2 1 ", "{}", rev.as_str());

        let mut it = [1, 2, 3].into_iter();
        let _ = it.next();
        check!(r, "2", "{}", it.len());
        check!(r, "[2, 3]", "{:?}", it.as_slice());

        let mut it = [1, 2, 3].into_iter();
        let _ = it.next_back();
        check!(r, "IntoIter([1, 2])", "{:?}", it);

        let mut it = [1, 2, 3].into_iter();
        let _ = it.next();
        let mut copy = it.clone();
        check!(r, "Some(2) Some(3) None", "{:?} {:?} {:?}", copy.next(), copy.next(), copy.next());
        check!(r, "Some(2) Some(3) None", "{:?} {:?} {:?}", it.next(), it.next(), it.next());

        check!(r, "None", "{:?}", core::array::IntoIter::<u8, 3>::empty().next());

        check!(r, "[1, 2, 3, 4]", "{:?}", [[1, 2], [3, 4]][..].as_flattened());
        let mut grid = [[1, 2], [3, 4]];
        grid[..].as_flattened_mut()[2] = 9;
        check!(r, "[[1, 2], [9, 4]]", "{:?}", grid);

        check!(r, "true false true", "{} {} {}", [1, 2] == [1, 2][..], [1, 2][..] == [1, 3], [1, 2] == &[1, 2][..]);
        let mut s = [1, 2, 3];
        check!(r, "false true", "{} {}", [1, 2] == s[..], [1, 2, 3] == &mut s[..]);

        let arr: [i32; 3] = core::convert::From::from((1, 2, 3));
        let pair: (i32, i32) = core::convert::From::from([4, 5]);
        check!(r, "[1, 2, 3] (4, 5)", "{:?} {:?}", arr, pair);

        let mut v = [7, 8, 9];
        let got: Result<[i32; 2], core::array::TryFromSliceError> = core::convert::TryFrom::try_from(&mut v[..2]);
        check!(r, "Ok([7, 8])", "{:?}", got);
        let bad: Result<[i32; 2], core::array::TryFromSliceError> = core::convert::TryFrom::try_from(&mut v[..]);
        check!(r, "true", "{}", bad.is_err());

        struct D<'a>(&'a core::cell::Cell<u32>);
        impl core::ops::Drop for D<'_> {
            fn drop(&mut self) { self.0.set(self.0.get() + 1); }
        }
        let drops = core::cell::Cell::new(0u32);
        {
            let mut it = [D(&drops), D(&drops), D(&drops), D(&drops)].into_iter();
            {
                let _first = it.next();
                let _last = it.next_back();
            }
            check!(r, "2", "{}", drops.get());
        }
        check!(r, "4", "{}", drops.get());
    }
    { // lw5-e: atomics fetch_xor/fetch_nand/fetch_not, AtomicI8/I16, from_ptr, From/Default/Debug/into_inner, Ordering Debug
        let o = core::sync::atomic::Ordering::SeqCst;
        let a = core::sync::atomic::AtomicU32::new(0b1100);
        check!(r, "12 6", "{} {}", a.fetch_xor(0b1010, o), a.load(o));
        check!(r, "6 4294967289", "{} {}", a.fetch_nand(0b0110, o), a.load(o));
        let b = core::sync::atomic::AtomicBool::new(true);
        check!(r, "true false", "{} {}", b.fetch_xor(true, o), b.load(o));
        check!(r, "false true", "{} {}", b.fetch_nand(false, o), b.load(o));
        check!(r, "true false", "{} {}", b.fetch_not(o), b.load(o));
        check!(r, "false false", "{} {}", b.fetch_nand(true, core::sync::atomic::Ordering::Relaxed), b.load(o));
        let i8a = core::sync::atomic::AtomicI8::new(-5);
        check!(r, "-5 -2", "{} {}", i8a.fetch_add(3, o), i8a.load(o));
        let i8b = core::sync::atomic::AtomicI8::new(-1);
        check!(r, "-1 -16", "{} {}", i8b.fetch_xor(0x0F, core::sync::atomic::Ordering::AcqRel), i8b.load(o));
        let i16a = core::sync::atomic::AtomicI16::new(7);
        check!(r, "7 9", "{} {}", i16a.fetch_max(9, o), i16a.load(o));
        check!(r, "9 3", "{} {}", i16a.fetch_min(3, o), i16a.load(o));
        check!(r, "-3", "{}", core::sync::atomic::AtomicI8::from(-3).into_inner());
        check!(r, "0", "{:?}", core::sync::atomic::AtomicI16::default());
        let mut x: u64 = 1;
        let ax = unsafe { core::sync::atomic::AtomicU64::from_ptr(&mut x as *mut u64) };
        ax.store(5, o);
        check!(r, "5", "{}", x);
        let mut flag = false;
        let af = unsafe { core::sync::atomic::AtomicBool::from_ptr(&mut flag as *mut bool) };
        af.store(true, o);
        check!(r, "true", "{}", flag);
        let mut byte: u8 = 7;
        let mut slot: *mut u8 = core::ptr::null_mut::<u8>();
        let ap = unsafe { core::sync::atomic::AtomicPtr::from_ptr(&mut slot as *mut *mut u8) };
        ap.store(&mut byte as *mut u8, o);
        check!(r, "7", "{}", unsafe { *slot });
        check!(r, "true", "{:?}", core::sync::atomic::AtomicBool::new(true));
        check!(r, "0x0", "{:?}", core::sync::atomic::AtomicPtr::<u8>::default());
        check!(r, "0x0", "{:?}", core::sync::atomic::AtomicPtr::<u8>::from(core::ptr::null_mut::<u8>()).into_inner());
        check!(r, "AcqRel", "{:?}", core::sync::atomic::Ordering::AcqRel);
        check!(r, "Relaxed Release Acquire SeqCst", "{:?} {:?} {:?} {:?}",
            core::sync::atomic::Ordering::Relaxed, core::sync::atomic::Ordering::Release,
            core::sync::atomic::Ordering::Acquire, core::sync::atomic::Ordering::SeqCst);
        check!(r, "3", "{}", core::sync::atomic::AtomicU16::from(3).into_inner());
        check!(r, "false", "{}", core::sync::atomic::AtomicBool::default().into_inner());
        check!(r, "true", "{}", core::sync::atomic::AtomicBool::from(true).into_inner());
    }
    { // lw5-f: integer API: mixed-sign arithmetic, isqrt, midpoint, cast_signed/unsigned, unbounded shifts, parse error equality
        check!(r, "1", "{}", u32::MAX.wrapping_add_signed(2));
        check!(r, "None", "{:?}", 5u32.checked_add_signed(-6));
        check!(r, "0", "{}", 5u8.saturating_add_signed(-9));
        check!(r, "255", "{}", 250u8.saturating_add_signed(10));
        check!(r, "(4, true)", "{:?}", 250u8.overflowing_add_signed(10));
        check!(r, "(255, true)", "{:?}", 0u8.overflowing_add_signed(-1));
        check!(r, "(3, false)", "{:?}", 5u8.overflowing_add_signed(-2));
        check!(r, "Some(5)", "{:?}", 3u8.checked_sub_signed(-2));
        check!(r, "None", "{:?}", 3u8.checked_sub_signed(4));
        check!(r, "0", "{}", 3u8.saturating_sub_signed(4));
        check!(r, "255", "{}", 250u8.saturating_sub_signed(-10));
        check!(r, "255", "{}", 0u8.wrapping_sub_signed(1));
        check!(r, "(1, false)", "{:?}", 0u32.overflowing_sub_signed(-1));
        check!(r, "Some(8)", "{:?}", 5i32.checked_add_unsigned(3));
        check!(r, "None", "{:?}", 100i8.checked_add_unsigned(28));
        check!(r, "127", "{}", i8::MAX.saturating_add_unsigned(200));
        check!(r, "-56", "{}", 0i8.wrapping_add_unsigned(200));
        check!(r, "(72, false)", "{:?}", (-128i8).overflowing_add_unsigned(200));
        check!(r, "(127, true)", "{:?}", (-128i8).overflowing_sub_unsigned(1));
        check!(r, "Some(-10)", "{:?}", 10i32.checked_sub_unsigned(20));
        check!(r, "-128", "{}", 0i8.saturating_sub_unsigned(200));
        check!(r, "127", "{}", 0i8.wrapping_sub_unsigned(129));
        check!(r, "10", "{}", 100u8.isqrt());
        check!(r, "0", "{}", 0u32.isqrt());
        check!(r, "15", "{}", 255u8.isqrt());
        check!(r, "9", "{}", 99i32.isqrt());
        check!(r, "4294967295", "{}", u64::MAX.isqrt());
        check!(r, "18446744073709551615", "{}", u128::MAX.isqrt());
        check!(r, "11", "{}", i8::MAX.isqrt());
        check!(r, "None", "{:?}", (-1i32).checked_isqrt());
        check!(r, "Some(3)", "{:?}", 15i64.checked_isqrt());
        check!(r, "6", "{}", 5u8.midpoint(8));
        check!(r, "254", "{}", u8::MAX.midpoint(253));
        check!(r, "4294967295", "{}", u32::MAX.midpoint(u32::MAX));
        check!(r, "-1", "{}", (-3i32).midpoint(0));
        check!(r, "-2", "{}", (-4i32).midpoint(0));
        check!(r, "1", "{}", 1i32.midpoint(2));
        check!(r, "-67", "{}", (-7i8).midpoint(i8::MIN));
        check!(r, "2147483647", "{}", i32::MAX.midpoint(i32::MAX));
        check!(r, "0", "{}", i64::MIN.midpoint(i64::MAX));
        check!(r, "2.5", "{}", 1.0f64.midpoint(4.0));
        check!(r, "1.5", "{}", 1.0f32.midpoint(2.0));
        check!(r, "true", "{}", f64::MAX.midpoint(f64::MAX) == f64::MAX);
        check!(r, "true", "{}", f64::NAN.midpoint(1.0).is_nan());
        check!(r, "-56", "{}", 200u8.cast_signed());
        check!(r, "4294967295", "{}", (-1i32).cast_unsigned());
        check!(r, "0", "{}", 1u32.unbounded_shl(32));
        check!(r, "2147483648", "{}", 1u32.unbounded_shl(31));
        check!(r, "0", "{}", 255u8.unbounded_shr(8));
        check!(r, "-1", "{}", (-8i32).unbounded_shr(40));
        check!(r, "0", "{}", 8i32.unbounded_shr(40));
        check!(r, "-4", "{}", (-8i32).unbounded_shr(1));
        check!(r, "0", "{}", (-1i64).unbounded_shl(64));
        check!(r, "true", "{}", "123".parse::<i32>() == Ok(123));
        check!(r, "true", "{}", "12x".parse::<i32>() == "x".parse::<i32>());
        check!(r, "false", "{}", "".parse::<u8>().unwrap_err() == "x".parse::<u8>().unwrap_err());
        let overflow = "300".parse::<u8>().unwrap_err();
        check!(r, "PosOverflow", "{:?}", overflow.kind());
        check!(r, "NegOverflow", "{:?}", "-200".parse::<i8>().unwrap_err().kind());
        check!(r, "Empty", "{:?}", "".parse::<i8>().unwrap_err().kind());
        check!(r, "true", "{}", u8::try_from(300i32).unwrap_err() == u8::try_from(-1i32).unwrap_err());
        check!(r, "true", "{}", u8::try_from(5i32) == Ok(5));
    }
    let mut summary = Buf::new();
    let _ = write!(summary, "core-fmt-tests: {} passed, {} failed\n", r.passed, r.failed);
    out(summary.as_str());
    if r.failed == 0 { 0 } else { 1 }
}
