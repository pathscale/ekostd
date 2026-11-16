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

    let mut summary = Buf::new();
    let _ = write!(summary, "core-fmt-tests: {} passed, {} failed\n", r.passed, r.failed);
    out(summary.as_str());
    if r.failed == 0 { 0 } else { 1 }
}
