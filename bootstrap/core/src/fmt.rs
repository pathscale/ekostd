//! Allocation-free formatting for the bootstrap runtime.
pub use crate::macros::builtin::Debug;
pub struct Error;
impl Copy for Error {}
impl Clone for Error { fn clone(&self) -> Self { *self } }
pub type Result = crate::result::Result<(), Error>;

macro_rules! attempt {
    ($e:expr) => { match $e { Ok(value) => value, Err(error) => return Err(error) } };
}
#[rustc_diagnostic_item = "FmtWrite"]
pub trait Write {
    fn write_str(&mut self, s: &str) -> Result;
    fn write_char(&mut self, c: char) -> Result {
        let mut bytes = [0; 4];
        self.write_str(c.encode_utf8(&mut bytes))
    }
    fn write_fmt(&mut self, args: Arguments<'_>) -> Result {
        struct Adapter<'a, T: ?Sized + 'a>(&'a mut T);
        impl<T: Write + ?Sized> Write for Adapter<'_, T> {
            fn write_str(&mut self, s: &str) -> Result { self.0.write_str(s) }
            fn write_fmt(&mut self, args: Arguments<'_>) -> Result { write(self, args) }
        }
        write(&mut Adapter(self), args)
    }
}
impl<W: Write + ?Sized> Write for &mut W {
    fn write_str(&mut self, s: &str) -> Result { (**self).write_str(s) }
}
/// Placeholder options, encoded as in rustc's format_args lowering: the low 21 bits of `flags` are the fill
/// character, the rest are the flag bits below.
struct Options { flags: u32, width: u16, precision: u16 }
impl Copy for Options {}
impl Clone for Options { fn clone(&self) -> Self { *self } }
mod flags {
    pub const SIGN_PLUS: u32 = 1 << 21;
    pub const SIGN_MINUS: u32 = 1 << 22;
    pub const ALTERNATE: u32 = 1 << 23;
    pub const ZERO_PAD: u32 = 1 << 24;
    pub const DEBUG_LOWER_HEX: u32 = 1 << 25;
    pub const DEBUG_UPPER_HEX: u32 = 1 << 26;
    pub const WIDTH: u32 = 1 << 27;
    pub const PRECISION: u32 = 1 << 28;
    pub const ALIGN_SHIFT: u32 = 29;
    pub const ALIGN_BITS: u32 = 3 << 29;
    pub const ALIGN_RIGHT: u32 = 1 << 29;
    pub const ALIGN_UNKNOWN: u32 = 3 << 29;
    pub const FILL: u32 = (1 << 21) - 1;
    /// The flag value of a placeholder without options.
    pub const DEFAULT: u32 = ' ' as u32 | ALIGN_UNKNOWN;
}
pub struct Formatter<'a> { writer: &'a mut dyn Write, options: Options }
impl<'a> Formatter<'a> {
    /// Whether the `#` flag was given.
    pub fn alternate(&self) -> bool { self.options.flags & flags::ALTERNATE != 0 }
    pub fn write_str(&mut self, s: &str) -> Result { self.writer.write_str(s) }
    pub fn write_char(&mut self, c: char) -> Result { self.writer.write_char(c) }
    pub fn write_fmt(&mut self, args: Arguments<'_>) -> Result { write(self.writer, args) }
    pub fn debug_map(&mut self) -> DebugMap<'_, 'a> {
        let failed = self.write_str("{").is_err();
        DebugMap { fmt: self, failed, fields: false, has_key: false }
    }
    pub fn debug_set(&mut self) -> DebugSet<'_, 'a> {
        let failed = self.write_str("{").is_err();
        DebugSet { fmt: self, failed, fields: false }
    }
    pub fn debug_list(&mut self) -> DebugList<'_, 'a> {
        let failed = self.write_str("[").is_err();
        DebugList { fmt: self, failed, fields: false }
    }
    pub fn debug_tuple_field1_finish<'b>(&'b mut self, name: &str, value1: &dyn Debug) -> Result {
        attempt!(self.write_str(name));
        attempt!(self.write_str("("));
        attempt!(value1.fmt(self));
        self.write_str(")")
    }
    pub fn debug_tuple_field2_finish<'b>(&'b mut self, name: &str, value1: &dyn Debug, value2: &dyn Debug) -> Result {
        attempt!(self.write_str(name));
        attempt!(self.write_str("("));
        attempt!(value1.fmt(self));
        attempt!(self.write_str(", "));
        attempt!(value2.fmt(self));
        self.write_str(")")
    }
    pub fn debug_tuple_field3_finish<'b>(&'b mut self, name: &str, value1: &dyn Debug, value2: &dyn Debug, value3: &dyn Debug) -> Result {
        self.debug_tuple_fields_finish(name, &[value1, value2, value3])
    }
    pub fn debug_tuple_field4_finish<'b>(&'b mut self, name: &str, value1: &dyn Debug, value2: &dyn Debug, value3: &dyn Debug, value4: &dyn Debug) -> Result {
        self.debug_tuple_fields_finish(name, &[value1, value2, value3, value4])
    }
    pub fn debug_tuple_field5_finish<'b>(&'b mut self, name: &str, value1: &dyn Debug, value2: &dyn Debug, value3: &dyn Debug, value4: &dyn Debug, value5: &dyn Debug) -> Result {
        self.debug_tuple_fields_finish(name, &[value1, value2, value3, value4, value5])
    }
    pub fn debug_tuple_fields_finish<'b>(&'b mut self, name: &str, values: &[&dyn Debug]) -> Result {
        attempt!(self.write_str(name));
        attempt!(self.write_str("("));
        let mut i = 0;
        while i < values.len() {
            if i != 0 { attempt!(self.write_str(", ")); }
            attempt!(values[i].fmt(self));
            i += 1;
        }
        self.write_str(")")
    }
    pub fn debug_struct_field1_finish<'b>(&'b mut self, name: &str, name1: &str, value1: &dyn Debug) -> Result {
        self.debug_struct_fields_finish(name, &[name1], &[value1])
    }
    pub fn debug_struct_field2_finish<'b>(&'b mut self, name: &str, name1: &str, value1: &dyn Debug, name2: &str, value2: &dyn Debug) -> Result {
        self.debug_struct_fields_finish(name, &[name1, name2], &[value1, value2])
    }
    pub fn debug_struct_field3_finish<'b>(&'b mut self, name: &str, name1: &str, value1: &dyn Debug, name2: &str, value2: &dyn Debug, name3: &str, value3: &dyn Debug) -> Result {
        self.debug_struct_fields_finish(name, &[name1, name2, name3], &[value1, value2, value3])
    }
    pub fn debug_struct_field4_finish<'b>(&'b mut self, name: &str, name1: &str, value1: &dyn Debug, name2: &str, value2: &dyn Debug, name3: &str, value3: &dyn Debug, name4: &str, value4: &dyn Debug) -> Result {
        self.debug_struct_fields_finish(name, &[name1, name2, name3, name4], &[value1, value2, value3, value4])
    }
    pub fn debug_struct_field5_finish<'b>(&'b mut self, name: &str, name1: &str, value1: &dyn Debug, name2: &str, value2: &dyn Debug, name3: &str, value3: &dyn Debug, name4: &str, value4: &dyn Debug, name5: &str, value5: &dyn Debug) -> Result {
        self.debug_struct_fields_finish(name, &[name1, name2, name3, name4, name5], &[value1, value2, value3, value4, value5])
    }
    pub fn debug_struct_fields_finish<'b>(&'b mut self, name: &str, names: &[&str], values: &[&dyn Debug]) -> Result {
        attempt!(self.write_str(name));
        attempt!(self.write_str(" { "));
        let mut i = 0;
        while i < names.len() && i < values.len() {
            if i != 0 { attempt!(self.write_str(", ")); }
            attempt!(self.write_str(names[i]));
            attempt!(self.write_str(": "));
            attempt!(values[i].fmt(self));
            i += 1;
        }
        self.write_str(" }")
    }
}
pub struct DebugList<'b, 'a: 'b> { fmt: &'b mut Formatter<'a>, failed: bool, fields: bool }
impl<'b, 'a: 'b> DebugList<'b, 'a> {
    pub fn entry(&mut self, entry: &dyn Debug) -> &mut Self {
        if !self.failed {
            let prefix = if self.fields { ", " } else { "" };
            self.failed = self.fmt.write_str(prefix).is_err() || entry.fmt(self.fmt).is_err();
        }
        self.fields = true;
        self
    }
    pub fn entries<D: Debug, I: crate::iter::IntoIterator<Item = D>>(&mut self, entries: I) -> &mut Self {
        for entry in entries {
            self.entry(&entry);
        }
        self
    }
    pub fn finish_non_exhaustive(&mut self) -> Result {
        if self.failed { return Err(Error); }
        self.fmt.write_str(if self.fields { ", ..]" } else { "..]" })
    }
    pub fn finish(&mut self) -> Result {
        if self.failed { return Err(Error); }
        self.fmt.write_str("]")
    }
}

pub struct DebugSet<'b, 'a: 'b> { fmt: &'b mut Formatter<'a>, failed: bool, fields: bool }
impl<'b, 'a: 'b> DebugSet<'b, 'a> {
    pub fn entry(&mut self, entry: &dyn Debug) -> &mut Self {
        if !self.failed {
            let prefix = if self.fields { ", " } else { "" };
            self.failed = self.fmt.write_str(prefix).is_err() || entry.fmt(self.fmt).is_err();
        }
        self.fields = true;
        self
    }
    pub fn entries<D: Debug, I: crate::iter::IntoIterator<Item = D>>(&mut self, entries: I) -> &mut Self {
        for entry in entries {
            self.entry(&entry);
        }
        self
    }
    pub fn finish_non_exhaustive(&mut self) -> Result {
        if self.failed { return Err(Error); }
        self.fmt.write_str(if self.fields { ", ..}" } else { "..}" })
    }
    pub fn finish(&mut self) -> Result {
        if self.failed { return Err(Error); }
        self.fmt.write_str("}")
    }
}

pub struct DebugMap<'b, 'a: 'b> { fmt: &'b mut Formatter<'a>, failed: bool, fields: bool, has_key: bool }
impl<'b, 'a: 'b> DebugMap<'b, 'a> {
    pub fn key(&mut self, key: &dyn Debug) -> &mut Self {
        if !self.failed {
            let prefix = if self.fields { ", " } else { "" };
            self.failed = self.fmt.write_str(prefix).is_err() || key.fmt(self.fmt).is_err();
        }
        self.fields = true;
        self.has_key = true;
        self
    }
    pub fn value(&mut self, value: &dyn Debug) -> &mut Self {
        if !self.failed {
            self.failed = self.fmt.write_str(": ").is_err() || value.fmt(self.fmt).is_err();
        }
        self.has_key = false;
        self
    }
    pub fn entry(&mut self, key: &dyn Debug, value: &dyn Debug) -> &mut Self {
        self.key(key).value(value)
    }
    pub fn entries<K: Debug, V: Debug, I: crate::iter::IntoIterator<Item = (K, V)>>(&mut self, entries: I) -> &mut Self {
        for (k, v) in entries {
            self.entry(&k, &v);
        }
        self
    }
    pub fn finish_non_exhaustive(&mut self) -> Result {
        if self.failed { return Err(Error); }
        self.fmt.write_str(if self.fields { ", ..}" } else { "..}" })
    }
    pub fn finish(&mut self) -> Result {
        if self.failed { return Err(Error); }
        self.fmt.write_str("}")
    }
}
#[rustc_diagnostic_item = "Display"]
pub trait Display { fn fmt(&self, f: &mut Formatter<'_>) -> Result; }
#[rustc_diagnostic_item = "Debug"]
pub trait Debug { fn fmt(&self, f: &mut Formatter<'_>) -> Result; }
pub trait LowerHex { fn fmt(&self, f: &mut Formatter<'_>) -> Result; }
pub trait UpperHex { fn fmt(&self, f: &mut Formatter<'_>) -> Result; }
pub trait Binary { fn fmt(&self, f: &mut Formatter<'_>) -> Result; }
pub trait Octal { fn fmt(&self, f: &mut Formatter<'_>) -> Result; }
pub trait LowerExp { fn fmt(&self, f: &mut Formatter<'_>) -> Result; }
pub trait UpperExp { fn fmt(&self, f: &mut Formatter<'_>) -> Result; }
pub trait Pointer { fn fmt(&self, f: &mut Formatter<'_>) -> Result; }

impl Debug for () {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result { f.write_str("()") }
}
impl<A: Debug, B: Debug> Debug for (A, B) {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        attempt!(f.write_str("("));
        attempt!(Debug::fmt(&self.0, f));
        attempt!(f.write_str(", "));
        attempt!(Debug::fmt(&self.1, f));
        f.write_str(")")
    }
}
impl<T: Debug> Debug for [T] {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        attempt!(f.write_str("["));
        let mut i = 0;
        while i < self.len() {
            if i != 0 { attempt!(f.write_str(", ")); }
            attempt!(Debug::fmt(&self[i], f));
            i += 1;
        }
        f.write_str("]")
    }
}
impl<T: Debug, const N: usize> Debug for [T; N] {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        Debug::fmt(&self[..], f)
    }
}
impl<T: Debug> Debug for Option<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            None => f.write_str("None"),
            Some(value) => {
                attempt!(f.write_str("Some("));
                attempt!(Debug::fmt(value, f));
                f.write_str(")")
            }
        }
    }
}
impl<T: Debug, E: Debug> Debug for crate::result::Result<T, E> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Ok(value) => {
                attempt!(f.write_str("Ok("));
                attempt!(Debug::fmt(value, f));
            }
            Err(error) => {
                attempt!(f.write_str("Err("));
                attempt!(Debug::fmt(error, f));
            }
        }
        f.write_str(")")
    }
}

pub mod rt {
    use super::*;
    #[lang = "format_argument"]
    pub struct Argument<'a> { kind: Kind<'a> }
    enum Kind<'a> {
        Display(&'a dyn Display), Debug(&'a dyn Debug),
        LowerHex(&'a dyn LowerHex), UpperHex(&'a dyn UpperHex),
        Binary(&'a dyn Binary), Octal(&'a dyn Octal),
        LowerExp(&'a dyn LowerExp), UpperExp(&'a dyn UpperExp),
        Pointer(&'a dyn Pointer), Count(usize),
    }
    impl<'a> Argument<'a> {
        pub const fn new_display<T: Display>(v: &'a T) -> Self { Self { kind: Kind::Display(v) } }
        pub const fn new_debug<T: Debug>(v: &'a T) -> Self { Self { kind: Kind::Debug(v) } }
        pub const fn new_lower_hex<T: LowerHex>(v: &'a T) -> Self { Self { kind: Kind::LowerHex(v) } }
        pub const fn new_upper_hex<T: UpperHex>(v: &'a T) -> Self { Self { kind: Kind::UpperHex(v) } }
        pub const fn new_binary<T: Binary>(v: &'a T) -> Self { Self { kind: Kind::Binary(v) } }
        pub const fn new_octal<T: Octal>(v: &'a T) -> Self { Self { kind: Kind::Octal(v) } }
        pub const fn new_lower_exp<T: LowerExp>(v: &'a T) -> Self { Self { kind: Kind::LowerExp(v) } }
        pub const fn new_upper_exp<T: UpperExp>(v: &'a T) -> Self { Self { kind: Kind::UpperExp(v) } }
        pub const fn new_pointer<T: Pointer>(v: &'a T) -> Self { Self { kind: Kind::Pointer(v) } }
        pub const fn from_usize(v: &'a usize) -> Self { Self { kind: Kind::Count(*v) } }
        pub(super) fn as_u16(&self) -> Option<u16> {
            match &self.kind { Kind::Count(n) => Some(*n as u16), _ => None }
        }
        pub(super) fn format(&self, f: &mut Formatter<'_>) -> Result {
            match &self.kind {
                Kind::Display(v) => Display::fmt(*v, f), Kind::Debug(v) => Debug::fmt(*v, f),
                Kind::LowerHex(v) => LowerHex::fmt(*v, f), Kind::UpperHex(v) => UpperHex::fmt(*v, f),
                Kind::Binary(v) => Binary::fmt(*v, f), Kind::Octal(v) => Octal::fmt(*v, f),
                Kind::LowerExp(v) => LowerExp::fmt(*v, f), Kind::UpperExp(v) => UpperExp::fmt(*v, f),
                Kind::Pointer(v) => Pointer::fmt(*v, f), Kind::Count(_) => Err(Error),
            }
        }
    }
}
pub type ArgumentV1<'a> = rt::Argument<'a>;
#[lang = "format_arguments"]
pub struct Arguments<'a> {
    template: &'a [u8],
    args: &'a [rt::Argument<'a>],
    literal: Option<&'a str>,
}
impl Copy for Arguments<'_> {}
impl Clone for Arguments<'_> { fn clone(&self) -> Self { *self } }
#[rustc_diagnostic_item = "FmtArgumentsNew"]
impl<'a> Arguments<'a> {
    /// The compiler must supply a valid encoded template and matching arguments.
    pub const unsafe fn new<const N: usize, const M: usize>(
        template: &'a [u8; N], args: &'a [rt::Argument<'a>; M],
    ) -> Self { Self { template, args, literal: None } }
    pub const fn from_str(s: &'static str) -> Self {
        Self { template: &[], args: &[], literal: Some(s) }
    }
    pub fn from_str_nonconst(s: &'static str) -> Self { Self::from_str(s) }
    pub const fn as_str(&self) -> Option<&'a str> { self.literal }
}
impl Display for Arguments<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result { f.write_fmt(*self) }
}
impl Debug for Arguments<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result { Display::fmt(self, f) }
}
pub fn write(output: &mut dyn Write, args: Arguments<'_>) -> Result {
    if let Some(s) = args.literal { return output.write_str(s); }
    let mut offset = 0;
    let mut next = 0;
    let bytes = args.template;
    while offset < bytes.len() {
        let tag = bytes[offset];
        offset += 1;
        if tag == 0 { return Ok(()); }
        if tag <= 128 {
            let mut len = tag as usize;
            if tag == 128 {
                if bytes.len() - offset < 2 { return Err(Error); }
                len = bytes[offset] as usize | ((bytes[offset + 1] as usize) << 8);
                offset += 2;
            }
            if len > bytes.len() - offset { return Err(Error); }
            // Compiler templates contain validated UTF-8 literal pieces.
            let piece = unsafe { crate::slice::from_raw_parts(bytes.as_ptr().add(offset), len) };
            attempt!(output.write_str(unsafe { crate::str::from_utf8_unchecked(piece) }));
            offset += len;
        } else {
            // The encoding is rustc's: 0xC0 | bit 0 flags (u32), bit 1 width (u16), bit 2 precision (u16),
            // bit 3 explicit argument index (u16), bit 4/5 width/precision taken from a count argument.
            if tag < 192 { return Err(Error); }
            let mut options = Options { flags: flags::DEFAULT, width: 0, precision: 0 };
            let need = (if tag & 1 != 0 { 4 } else { 0 }) + (if tag & 2 != 0 { 2 } else { 0 })
                + (if tag & 4 != 0 { 2 } else { 0 }) + (if tag & 8 != 0 { 2 } else { 0 });
            if bytes.len() - offset < need { return Err(Error); }
            if tag & 1 != 0 {
                options.flags = u32::from_le_bytes([bytes[offset], bytes[offset + 1], bytes[offset + 2], bytes[offset + 3]]);
                offset += 4;
            }
            if tag & 2 != 0 { options.width = bytes[offset] as u16 | ((bytes[offset + 1] as u16) << 8); offset += 2; }
            if tag & 4 != 0 { options.precision = bytes[offset] as u16 | ((bytes[offset + 1] as u16) << 8); offset += 2; }
            let index = if tag & 8 != 0 {
                let index = bytes[offset] as usize | ((bytes[offset + 1] as usize) << 8);
                offset += 2;
                index
            } else { let index = next; next += 1; index };
            if tag & 16 != 0 {
                let Some(count) = args.args.get(options.width as usize) else { return Err(Error) };
                let Some(width) = count.as_u16() else { return Err(Error) };
                options.width = width;
            }
            if tag & 32 != 0 {
                let Some(count) = args.args.get(options.precision as usize) else { return Err(Error) };
                let Some(precision) = count.as_u16() else { return Err(Error) };
                options.precision = precision;
            }
            if index >= args.args.len() { return Err(Error); }
            attempt!(args.args[index].format(&mut Formatter { writer: output, options }));
        }
    }
    Err(Error)
}
fn number(value: u128, radix: u128, upper: bool, f: &mut Formatter<'_>) -> Result {
    let mut bytes = [0u8; 128];
    f.write_str(digits(value, radix, upper, &mut bytes))
}
/// The digits of `value` in `radix`, most significant first, written at the end of `bytes`.
fn digits(mut value: u128, radix: u128, upper: bool, bytes: &mut [u8; 128]) -> &str {
    let mut start = 128;
    loop {
        let digit = (value % radix) as u8;
        start -= 1;
        bytes[start] = if digit < 10 { b'0' + digit }
            else if upper { b'A' + digit - 10 } else { b'a' + digit - 10 };
        value /= radix;
        if value == 0 { break; }
    }
    let s = unsafe { crate::slice::from_raw_parts(bytes.as_ptr().add(start), 128 - start) };
    unsafe { crate::str::from_utf8_unchecked(s) }
}
fn escaped(c: char, quote: char, f: &mut Formatter<'_>) -> Result {
    match c {
        '\n' => f.write_str("\\n"), '\r' => f.write_str("\\r"),
        '\t' => f.write_str("\\t"), '\0' => f.write_str("\\0"),
        '\\' => f.write_str("\\\\"),
        _ => {
            if c == quote { attempt!(f.write_char('\\')); return f.write_char(c); }
            if (c as u32) < 32 || (c as u32) == 127 {
                attempt!(f.write_str("\\u{"));
                attempt!(number(c as u128, 16, false, f));
                f.write_str("}")
            } else { f.write_char(c) }
        }
    }
}
impl Display for bool {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result { f.pad(if *self { "true" } else { "false" }) }
}
impl Debug for bool { fn fmt(&self, f: &mut Formatter<'_>) -> Result { Display::fmt(self, f) } }
impl Display for char {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        if f.options.flags & (flags::WIDTH | flags::PRECISION) == 0 { return f.write_char(*self); }
        let mut bytes = [0u8; 4];
        f.pad(self.encode_utf8(&mut bytes))
    }
}
impl Debug for char {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        attempt!(f.write_char('\'')); attempt!(escaped(*self, '\'', f)); f.write_char('\'')
    }
}
impl Display for str { fn fmt(&self, f: &mut Formatter<'_>) -> Result { f.pad(self) } }
impl Debug for str {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        attempt!(f.write_char('"'));
        // Decode UTF-8 directly so bootstrap formatting needs no iterator traits.
        let bytes = self.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            let b = bytes[i]; i += 1;
            let (mut code, count) = if b < 128 { (b as u32, 0) }
                else if b < 224 { ((b & 31) as u32, 1) }
                else if b < 240 { ((b & 15) as u32, 2) } else { ((b & 7) as u32, 3) };
            let mut n = 0;
            while n < count { code = (code << 6) | (bytes[i] & 63) as u32; i += 1; n += 1; }
            let c = unsafe { crate::intrinsics::transmute::<u32, char>(code) };
            attempt!(escaped(c, '"', f));
        }
        f.write_char('"')
    }
}
macro_rules! reference {
    ($trait:ident) => {
        impl<T: $trait + ?Sized> $trait for &T {
            fn fmt(&self, f: &mut Formatter<'_>) -> Result { $trait::fmt(*self, f) }
        }
        impl<T: $trait + ?Sized> $trait for &mut T {
            fn fmt(&self, f: &mut Formatter<'_>) -> Result { $trait::fmt(&**self, f) }
        }
    };
}
reference!(Display); reference!(Debug); reference!(LowerHex); reference!(UpperHex);
reference!(Binary); reference!(Octal); reference!(LowerExp); reference!(UpperExp);
macro_rules! integer {
    ($t:ty, $u:ty, $signed:expr) => {
        impl Display for $t {
            #[allow(unused_comparisons)]
            fn fmt(&self, f: &mut Formatter<'_>) -> Result {
                let negative = $signed && *self < 0;
                let magnitude = if negative { (!(*self as $u)).wrapping_add(1) } else { *self as $u };
                let mut buf = [0u8; 128];
                f.pad_integral(!negative, "", digits(magnitude as u128, 10, false, &mut buf))
            }
        }
        impl Debug for $t {
            fn fmt(&self, f: &mut Formatter<'_>) -> Result {
                if f.debug_lower_hex() { LowerHex::fmt(self, f) }
                else if f.debug_upper_hex() { UpperHex::fmt(self, f) }
                else { Display::fmt(self, f) }
            }
        }
        impl LowerHex for $t {
            fn fmt(&self, f: &mut Formatter<'_>) -> Result {
                let mut buf = [0u8; 128]; f.pad_integral(true, "0x", digits(*self as $u as u128, 16, false, &mut buf))
            }
        }
        impl UpperHex for $t {
            fn fmt(&self, f: &mut Formatter<'_>) -> Result {
                let mut buf = [0u8; 128]; f.pad_integral(true, "0x", digits(*self as $u as u128, 16, true, &mut buf))
            }
        }
        impl Binary for $t {
            fn fmt(&self, f: &mut Formatter<'_>) -> Result {
                let mut buf = [0u8; 128]; f.pad_integral(true, "0b", digits(*self as $u as u128, 2, false, &mut buf))
            }
        }
        impl Octal for $t {
            fn fmt(&self, f: &mut Formatter<'_>) -> Result {
                let mut buf = [0u8; 128]; f.pad_integral(true, "0o", digits(*self as $u as u128, 8, false, &mut buf))
            }
        }
    };
}
integer!(i8, u8, true); integer!(u8, u8, false);
integer!(i16, u16, true); integer!(u16, u16, false);
integer!(i32, u32, true); integer!(u32, u32, false);
integer!(i64, u64, true); integer!(u64, u64, false);
integer!(i128, u128, true); integer!(u128, u128, false);
integer!(isize, usize, true); integer!(usize, usize, false);
impl<T: ?Sized> Pointer for *const T {
    /// As Rust: always `0x`-prefixed lower hex; `#` zero-extends to the full pointer width.
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let old = f.options;
        if f.alternate() {
            f.options.flags |= flags::ZERO_PAD;
            if f.width().is_none() {
                f.options.flags |= flags::WIDTH;
                f.options.width = (crate::mem::size_of::<usize>() * 2) as u16 + 2;
            }
        }
        f.options.flags |= flags::ALTERNATE;
        let result = LowerHex::fmt(&(*self as *const () as usize), f);
        f.options = old;
        result
    }
}
impl<T: ?Sized> Pointer for *mut T {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result { Pointer::fmt(&(*self as *const T), f) }
}
impl<T: ?Sized> Debug for *const T {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result { Pointer::fmt(self, f) }
}
impl<T: ?Sized> Debug for *mut T {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result { Pointer::fmt(self, f) }
}
impl Debug for Error { fn fmt(&self, f: &mut Formatter<'_>) -> Result { f.write_str("Error") } }
impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result { f.write_str("an error occurred when formatting an argument") }
}

impl<T: ?Sized> Display for *const T {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result { Pointer::fmt(self, f) }
}
impl<T: ?Sized> Display for *mut T {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result { Pointer::fmt(self, f) }
}
impl<T: ?Sized> Pointer for &T {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result { Pointer::fmt(&(*self as *const T), f) }
}


/// Builder behind `Formatter::debug_struct`. Writes `Name { a: 1, b: 2 }`, or `Name` with no fields.
pub struct DebugStruct<'b, 'a: 'b> { fmt: &'b mut Formatter<'a>, failed: bool, fields: bool }
impl<'b, 'a: 'b> DebugStruct<'b, 'a> {
    pub fn field(&mut self, name: &str, value: &dyn Debug) -> &mut Self {
        if !self.failed {
            let first = !self.fields;
            self.failed = self.entry(first, name, value).is_err();
        }
        self.fields = true;
        self
    }
    fn entry(&mut self, first: bool, name: &str, value: &dyn Debug) -> Result {
        attempt!(self.fmt.write_str(if first { " { " } else { ", " }));
        attempt!(self.fmt.write_str(name));
        attempt!(self.fmt.write_str(": "));
        value.fmt(self.fmt)
    }
    pub fn finish_non_exhaustive(&mut self) -> Result {
        if self.failed { return Err(Error); }
        self.fmt.write_str(if self.fields { ", .. }" } else { " { .. }" })
    }
    pub fn finish(&mut self) -> Result {
        if self.failed { return Err(Error); }
        if self.fields { self.fmt.write_str(" }") } else { Ok(()) }
    }
}

/// Builder behind `Formatter::debug_tuple`. Writes `Name(1, 2)`, or `Name` with no fields.
pub struct DebugTuple<'b, 'a: 'b> { fmt: &'b mut Formatter<'a>, failed: bool, fields: bool }
impl<'b, 'a: 'b> DebugTuple<'b, 'a> {
    pub fn field(&mut self, value: &dyn Debug) -> &mut Self {
        if !self.failed {
            let first = !self.fields;
            self.failed = self.entry(first, value).is_err();
        }
        self.fields = true;
        self
    }
    fn entry(&mut self, first: bool, value: &dyn Debug) -> Result {
        attempt!(self.fmt.write_str(if first { "(" } else { ", " }));
        value.fmt(self.fmt)
    }
    pub fn finish_non_exhaustive(&mut self) -> Result {
        if self.failed { return Err(Error); }
        self.fmt.write_str(if self.fields { ", ..)" } else { "(..)" })
    }
    pub fn finish(&mut self) -> Result {
        if self.failed { return Err(Error); }
        if self.fields { self.fmt.write_str(")") } else { Ok(()) }
    }
}

impl<'a> Formatter<'a> {
    pub fn debug_struct<'b>(&'b mut self, name: &str) -> DebugStruct<'b, 'a> {
        let failed = self.write_str(name).is_err();
        DebugStruct { fmt: self, failed, fields: false }
    }
    pub fn debug_tuple<'b>(&'b mut self, name: &str) -> DebugTuple<'b, 'a> {
        let failed = self.write_str(name).is_err();
        DebugTuple { fmt: self, failed, fields: false }
    }
}

impl<T: ?Sized> Pointer for crate::ptr::NonNull<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result { Pointer::fmt(&(self.as_ptr() as *const T), f) }
}
impl<T: ?Sized> Debug for crate::ptr::NonNull<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result { Pointer::fmt(self, f) }
}
impl<T: crate::marker::Copy + Debug> Debug for crate::cell::Cell<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result { f.debug_struct("Cell").field("value", &self.get()).finish() }
}
impl<T: ?Sized> Debug for crate::marker::PhantomData<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result { f.write_str("PhantomData") }
}
impl Debug for crate::alloc::Layout {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.debug_struct("Layout").field("size", &self.size()).field("align", &self.align()).finish()
    }
}

impl<A: Debug, B: Debug, C: Debug> Debug for (A, B, C) {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        attempt!(f.write_str("("));
        attempt!(Debug::fmt(&self.0, f));
        attempt!(f.write_str(", "));
        attempt!(Debug::fmt(&self.1, f));
        attempt!(f.write_str(", "));
        attempt!(Debug::fmt(&self.2, f));
        f.write_str(")")
    }
}
impl Debug for ! {
    fn fmt(&self, _f: &mut Formatter<'_>) -> Result { match *self {} }
}
const DEC_DIGITS: &[u8] = b"0123456789";

fn fmt_float_f64(val: f64, is_f32: bool, exp_char: u8, debug: bool, f: &mut Formatter<'_>) -> Result {
    if let Some(precision) = f.precision() { return float_exact(val, exp_char, precision, f); }
    if val.is_nan() {
        return f.pad_number("", "NaN", 0, "");
    }
    let mut buf = [0u8; 64];
    let mut pos: usize = 0;
    if val.is_sign_negative() {
        buf[pos] = b'-';
        pos += 1;
    }
    let mut x = val.abs();
    if x.is_infinite() {
        buf[pos] = b'i';
        buf[pos + 1] = b'n';
        buf[pos + 2] = b'f';
        pos += 3;
        return finish_float(f, &buf[..pos]);
    }
    if x == 0.0 {
        buf[pos] = b'0';
        pos += 1;
        if exp_char != 0 {
            buf[pos] = exp_char;
            buf[pos + 1] = b'0';
            pos += 2;
        } else if debug {
            buf[pos] = b'.';
            buf[pos + 1] = b'0';
            pos += 2;
        }
        return finish_float(f, &buf[..pos]);
    }

    let mut exp10: i32 = 0;
    let mut step = 0usize;
    while step < 24 {
        if x < 1e16 {
            break;
        }
        x /= 1e16;
        exp10 += 16;
        step += 1;
    }
    let mut step2 = 0usize;
    while step2 < 24 {
        if x >= 1e-15 {
            break;
        }
        x *= 1e16;
        exp10 -= 16;
        step2 += 1;
    }
    let mut step3 = 0usize;
    while step3 < 20 {
        if x < 10.0 {
            break;
        }
        x /= 10.0;
        exp10 += 1;
        step3 += 1;
    }
    let mut step4 = 0usize;
    while step4 < 20 {
        if x >= 1.0 {
            break;
        }
        x *= 10.0;
        exp10 -= 1;
        step4 += 1;
    }

    let ndigits: usize = if is_f32 { 7 } else { 15 };
    let mut digits = [0usize; 16];
    let mut i = 0usize;
    while i <= ndigits {
        let mut d: usize = 0;
        while d < 9 {
            if x < ((d + 1) as f64) {
                break;
            }
            d += 1;
        }
        digits[i] = d;
        x = (x - (d as f64)) * 10.0;
        i += 1;
    }

    if digits[ndigits] >= 5 {
        let mut carry: usize = 1;
        let mut j = ndigits;
        while j > 0 {
            if carry == 0 {
                break;
            }
            j -= 1;
            let sum = digits[j] + carry;
            if sum >= 10 {
                digits[j] = sum - 10;
                carry = 1;
            } else {
                digits[j] = sum;
                carry = 0;
            }
        }
        if carry > 0 {
            digits[0] = 1;
            let mut k = 1usize;
            while k < ndigits {
                digits[k] = 0;
                k += 1;
            }
            exp10 += 1;
        }
    }

    let mut len = ndigits;
    while len > 1 {
        if digits[len - 1] != 0 {
            break;
        }
        len -= 1;
    }

    let use_exp = exp_char != 0 || exp10 < -4 || exp10 >= (ndigits as i32);
    if use_exp {
        let ec = if exp_char != 0 { exp_char } else { b'e' };
        buf[pos] = DEC_DIGITS[digits[0]];
        pos += 1;
        if len > 1 {
            buf[pos] = b'.';
            pos += 1;
            let mut k = 1usize;
            while k < len {
                buf[pos] = DEC_DIGITS[digits[k]];
                pos += 1;
                k += 1;
            }
        }
        buf[pos] = ec;
        pos += 1;
        let mut e_abs: usize = if exp10 < 0 {
            buf[pos] = b'-';
            pos += 1;
            (-exp10) as usize
        } else {
            exp10 as usize
        };
        let mut e_buf = [0u8; 8];
        let mut e_len: usize = 0;
        let mut es = 0usize;
        while es < 8 {
            e_buf[e_len] = DEC_DIGITS[e_abs % 10];
            e_len += 1;
            e_abs /= 10;
            if e_abs == 0 {
                break;
            }
            es += 1;
        }
        while e_len > 0 {
            e_len -= 1;
            buf[pos] = e_buf[e_len];
            pos += 1;
        }
    } else if exp10 < 0 {
        buf[pos] = b'0';
        pos += 1;
        buf[pos] = b'.';
        pos += 1;
        let mut z = (-exp10 - 1) as usize;
        while z > 0 {
            buf[pos] = b'0';
            pos += 1;
            z -= 1;
        }
        let mut k = 0;
        while k < len {
            buf[pos] = DEC_DIGITS[digits[k]];
            pos += 1;
            k += 1;
        }
    } else {
        let int_digits = (exp10 as usize) + 1;
        if int_digits >= len {
            let mut k = 0;
            while k < len {
                buf[pos] = DEC_DIGITS[digits[k]];
                pos += 1;
                k += 1;
            }
            let mut z = int_digits - len;
            while z > 0 {
                buf[pos] = b'0';
                pos += 1;
                z -= 1;
            }
            if debug {
                buf[pos] = b'.';
                pos += 1;
                buf[pos] = b'0';
                pos += 1;
            }
        } else {
            let mut k = 0;
            while k < int_digits {
                buf[pos] = DEC_DIGITS[digits[k]];
                pos += 1;
                k += 1;
            }
            buf[pos] = b'.';
            pos += 1;
            while k < len {
                buf[pos] = DEC_DIGITS[digits[k]];
                pos += 1;
                k += 1;
            }
        }
    }

    finish_float(f, &buf[..pos])
}

impl Display for f32 {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result { fmt_float_f64(*self as f64, true, 0, false, f) }
}
impl Debug for f32 {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result { fmt_float_f64(*self as f64, true, 0, true, f) }
}
impl LowerExp for f32 {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result { fmt_float_f64(*self as f64, true, b'e', false, f) }
}
impl UpperExp for f32 {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result { fmt_float_f64(*self as f64, true, b'E', false, f) }
}

impl Display for f64 {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result { fmt_float_f64(*self, false, 0, false, f) }
}
impl Debug for f64 {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result { fmt_float_f64(*self, false, 0, true, f) }
}
impl LowerExp for f64 {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result { fmt_float_f64(*self, false, b'e', false, f) }
}
impl UpperExp for f64 {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result { fmt_float_f64(*self, false, b'E', false, f) }
}

macro_rules! fnptr_fmt_impls {
    (@one $($arg:ident),*) => {
        impl<Ret, $($arg),*> Pointer for fn($($arg),*) -> Ret {
            fn fmt(&self, f: &mut Formatter<'_>) -> Result {
                Pointer::fmt(&(*self as *const ()), f)
            }
        }
        impl<Ret, $($arg),*> Debug for fn($($arg),*) -> Ret {
            fn fmt(&self, f: &mut Formatter<'_>) -> Result {
                Pointer::fmt(self, f)
            }
        }
        impl<Ret, $($arg),*> Pointer for unsafe fn($($arg),*) -> Ret {
            fn fmt(&self, f: &mut Formatter<'_>) -> Result {
                Pointer::fmt(&(*self as *const ()), f)
            }
        }
        impl<Ret, $($arg),*> Debug for unsafe fn($($arg),*) -> Ret {
            fn fmt(&self, f: &mut Formatter<'_>) -> Result {
                Pointer::fmt(self, f)
            }
        }
        impl<Ret, $($arg),*> Pointer for extern "C" fn($($arg),*) -> Ret {
            fn fmt(&self, f: &mut Formatter<'_>) -> Result {
                Pointer::fmt(&(*self as *const ()), f)
            }
        }
        impl<Ret, $($arg),*> Debug for extern "C" fn($($arg),*) -> Ret {
            fn fmt(&self, f: &mut Formatter<'_>) -> Result {
                Pointer::fmt(self, f)
            }
        }
        impl<Ret, $($arg),*> Pointer for unsafe extern "C" fn($($arg),*) -> Ret {
            fn fmt(&self, f: &mut Formatter<'_>) -> Result {
                Pointer::fmt(&(*self as *const ()), f)
            }
        }
        impl<Ret, $($arg),*> Debug for unsafe extern "C" fn($($arg),*) -> Ret {
            fn fmt(&self, f: &mut Formatter<'_>) -> Result {
                Pointer::fmt(self, f)
            }
        }
    };
    () => {
        fnptr_fmt_impls!(@one);
    };
    ($first:ident $(, $rest:ident)*) => {
        fnptr_fmt_impls!(@one $first $(, $rest)*);
        fnptr_fmt_impls!($($rest),*);
    };
}
fnptr_fmt_impls!(A, B, C, D, E, F, G, H, I, J, K, L);

impl<T: ?Sized> Debug for crate::cell::UnsafeCell<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result { f.debug_struct("UnsafeCell").finish_non_exhaustive() }
}
impl<T> Debug for crate::mem::MaybeUninit<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result { f.pad(crate::any::type_name::<Self>()) }
}

impl<T: Debug> Debug for crate::slice::Iter<'_, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result { f.debug_tuple("Iter").field(&self.as_slice()).finish() }
}
impl<I: Debug> Debug for crate::iter::Enumerate<I> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result { f.debug_struct("Enumerate").field("iter", &self.iter).field("count", &self.count).finish() }
}
impl<T: ?Sized + Debug> Debug for crate::cell::RefCell<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let mut d = f.debug_struct("RefCell");
        match self.try_borrow() {
            Ok(borrow) => { d.field("value", &borrow); }
            Err(_) => { d.field("value", &format_args!("<borrowed>")); }
        }
        d.finish()
    }
}

impl<T: ?Sized + Debug> Debug for crate::cell::Ref<'_, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result { Debug::fmt(&**self, f) }
}

macro_rules! tuple_debug {
    ($(($($T:ident $i:tt),+))*) => {$(
        impl<$($T: Debug),+> Debug for ($($T,)+) {
            fn fmt(&self, f: &mut Formatter<'_>) -> Result {
                let mut tuple = f.debug_tuple("");
                $(tuple.field(&self.$i);)+
                tuple.finish()
            }
        }
    )*};
}
tuple_debug!((A 0, B 1, C 2, D 3) (A 0, B 1, C 2, D 3, E 4) (A 0, B 1, C 2, D 3, E 4, F 5)
    (A 0, B 1, C 2, D 3, E 4, F 5, G 6) (A 0, B 1, C 2, D 3, E 4, F 5, G 6, H 7)
    (A 0, B 1, C 2, D 3, E 4, F 5, G 6, H 7, I 8) (A 0, B 1, C 2, D 3, E 4, F 5, G 6, H 7, I 8, J 9)
    (A 0, B 1, C 2, D 3, E 4, F 5, G 6, H 7, I 8, J 9, K 10) (A 0, B 1, C 2, D 3, E 4, F 5, G 6, H 7, I 8, J 9, K 10, L 11));

/// Alignment requested by a placeholder: `<`, `>` or `^`.
pub enum Alignment { Left, Right, Center }
impl Copy for Alignment {}
impl Clone for Alignment { fn clone(&self) -> Self { *self } }

/// Character count of UTF-8 text: every byte that is not a continuation byte starts a character.
fn char_count(s: &str) -> usize {
    let bytes = s.as_bytes();
    let mut count = 0;
    let mut i = 0;
    while i < bytes.len() { if bytes[i] & 0xC0 != 0x80 { count += 1; } i += 1; }
    count
}

impl<'a> Formatter<'a> {
    /// The fill character (space unless the placeholder names one).
    pub fn fill(&self) -> char { unsafe { crate::intrinsics::transmute::<u32, char>(self.options.flags & flags::FILL) } }
    pub fn align(&self) -> Option<Alignment> {
        match (self.options.flags & flags::ALIGN_BITS) >> flags::ALIGN_SHIFT {
            0 => Some(Alignment::Left), 1 => Some(Alignment::Right), 2 => Some(Alignment::Center), _ => None,
        }
    }
    pub fn width(&self) -> Option<usize> {
        if self.options.flags & flags::WIDTH != 0 { Some(self.options.width as usize) } else { None }
    }
    pub fn precision(&self) -> Option<usize> {
        if self.options.flags & flags::PRECISION != 0 { Some(self.options.precision as usize) } else { None }
    }
    pub fn sign_plus(&self) -> bool { self.options.flags & flags::SIGN_PLUS != 0 }
    pub fn sign_minus(&self) -> bool { self.options.flags & flags::SIGN_MINUS != 0 }
    pub fn sign_aware_zero_pad(&self) -> bool { self.options.flags & flags::ZERO_PAD != 0 }
    fn debug_lower_hex(&self) -> bool { self.options.flags & flags::DEBUG_LOWER_HEX != 0 }
    fn debug_upper_hex(&self) -> bool { self.options.flags & flags::DEBUG_UPPER_HEX != 0 }
    fn min_width(&self) -> usize { match self.width() { Some(width) => width, None => 0 } }

    /// Writes the fill owed before a value of `count` missing columns and returns the fill and count owed after it.
    fn padding(&mut self, count: usize, default: Alignment) -> crate::result::Result<(char, usize), Error> {
        let fill = self.fill();
        let align = match self.align() { Some(align) => align, None => default };
        let before = match align { Alignment::Left => 0, Alignment::Right => count, Alignment::Center => count / 2 };
        let mut i = 0;
        while i < before { attempt!(self.writer.write_char(fill)); i += 1; }
        Ok((fill, count - before))
    }
    fn post_padding(&mut self, (fill, count): (char, usize)) -> Result {
        let mut i = 0;
        while i < count { attempt!(self.writer.write_char(fill)); i += 1; }
        Ok(())
    }
    /// Sign-aware zero padding: fill `0`, aligned right.
    fn set_zero_fill(&mut self) {
        self.options.flags = (self.options.flags & !(flags::FILL | flags::ALIGN_BITS)) | '0' as u32 | flags::ALIGN_RIGHT;
    }

    /// Writes an integer's digits `buf` with its sign, the `#` prefix, width, fill, alignment and `0`, as Rust.
    pub fn pad_integral(&mut self, is_nonnegative: bool, prefix: &str, buf: &str) -> Result {
        let mut width = buf.len();
        let sign = if !is_nonnegative { width += 1; "-" } else if self.sign_plus() { width += 1; "+" } else { "" };
        let prefix = if self.alternate() { width += char_count(prefix); prefix } else { "" };
        let min = self.min_width();
        if width >= min {
            attempt!(self.writer.write_str(sign));
            attempt!(self.writer.write_str(prefix));
            self.writer.write_str(buf)
        } else if self.sign_aware_zero_pad() {
            let old = self.options;
            self.set_zero_fill();
            attempt!(self.writer.write_str(sign));
            attempt!(self.writer.write_str(prefix));
            let post = attempt!(self.padding(min - width, Alignment::Right));
            attempt!(self.writer.write_str(buf));
            attempt!(self.post_padding(post));
            self.options = old;
            Ok(())
        } else {
            let post = attempt!(self.padding(min - width, Alignment::Right));
            attempt!(self.writer.write_str(sign));
            attempt!(self.writer.write_str(prefix));
            attempt!(self.writer.write_str(buf));
            self.post_padding(post)
        }
    }

    /// Writes `s` truncated to the precision (in characters) and padded to the width, left aligned by default, as Rust.
    pub fn pad(&mut self, s: &str) -> Result {
        if self.options.flags & (flags::WIDTH | flags::PRECISION) == 0 { return self.writer.write_str(s); }
        let (s, count) = match self.precision() {
            Some(max) => {
                let bytes = s.as_bytes();
                let mut end = 0;
                let mut count = 0;
                while end < bytes.len() && count < max {
                    let lead = bytes[end];
                    end += if lead < 0x80 { 1 } else if lead < 0xE0 { 2 } else if lead < 0xF0 { 3 } else { 4 };
                    count += 1;
                }
                (unsafe { crate::str::from_utf8_unchecked(&bytes[..end]) }, count)
            }
            None => (s, char_count(s)),
        };
        let min = self.min_width();
        if count < min {
            let post = attempt!(self.padding(min - count, Alignment::Left));
            attempt!(self.writer.write_str(s));
            self.post_padding(post)
        } else {
            self.writer.write_str(s)
        }
    }

    /// Writes a formatted number `sign`, `body`, `zeros` zero digits and `suffix` (all ASCII), padded as Rust pads floats.
    fn pad_number(&mut self, sign: &str, body: &str, zeros: usize, suffix: &str) -> Result {
        let len = sign.len() + body.len() + zeros + suffix.len();
        let min = self.min_width();
        if len >= min {
            attempt!(self.writer.write_str(sign));
            return self.write_number_rest(body, zeros, suffix);
        }
        if self.sign_aware_zero_pad() {
            let old = self.options;
            attempt!(self.writer.write_str(sign));
            self.set_zero_fill();
            let post = attempt!(self.padding(min - len, Alignment::Right));
            attempt!(self.write_number_rest(body, zeros, suffix));
            attempt!(self.post_padding(post));
            self.options = old;
            Ok(())
        } else {
            let post = attempt!(self.padding(min - len, Alignment::Right));
            attempt!(self.writer.write_str(sign));
            attempt!(self.write_number_rest(body, zeros, suffix));
            self.post_padding(post)
        }
    }
    fn write_number_rest(&mut self, body: &str, mut zeros: usize, suffix: &str) -> Result {
        attempt!(self.writer.write_str(body));
        const ZEROS: &str = "0000000000000000000000000000000000000000000000000000000000000000";
        while zeros > 0 {
            let n = if zeros < ZEROS.len() { zeros } else { ZEROS.len() };
            attempt!(self.writer.write_str(unsafe { crate::str::from_utf8_unchecked(&ZEROS.as_bytes()[..n]) }));
            zeros -= n;
        }
        self.writer.write_str(suffix)
    }
}

/// Pads float text produced by `fmt_float_f64` (a leading `-` is its sign).
fn finish_float(f: &mut Formatter<'_>, bytes: &[u8]) -> Result {
    let (sign, body) = if !bytes.is_empty() && bytes[0] == b'-' { ("-", &bytes[1..]) }
        else if f.sign_plus() { ("+", bytes) } else { ("", bytes) };
    f.pad_number(sign, unsafe { crate::str::from_utf8_unchecked(body) }, 0, "")
}

/// Fixed-size unsigned integer, little-endian 32-bit limbs; 1152 bits hold every finite f64 scaled as below.
struct Big { limbs: [u32; 36] }
impl Big {
    fn from_u64(value: u64) -> Self {
        let mut limbs = [0u32; 36];
        limbs[0] = value as u32;
        limbs[1] = (value >> 32) as u32;
        Big { limbs }
    }
    fn is_zero(&self) -> bool {
        let mut i = 0;
        while i < 36 { if self.limbs[i] != 0 { return false; } i += 1; }
        true
    }
    fn shl(&mut self, n: usize) {
        let words = n / 32;
        let bits = (n % 32) as u32;
        let mut i = 36;
        while i > 0 {
            i -= 1;
            let mut value = 0u32;
            if i >= words {
                value = self.limbs[i - words] << bits;
                if bits != 0 && i > words { value |= self.limbs[i - words - 1] >> (32 - bits); }
            }
            self.limbs[i] = value;
        }
    }
    fn shr(&mut self, n: usize) {
        let words = n / 32;
        let bits = (n % 32) as u32;
        let mut i = 0;
        while i < 36 {
            let mut value = 0u32;
            if i + words < 36 {
                value = self.limbs[i + words] >> bits;
                if bits != 0 && i + words + 1 < 36 { value |= self.limbs[i + words + 1] << (32 - bits); }
            }
            self.limbs[i] = value;
            i += 1;
        }
    }
    /// Keeps only the low `n` bits.
    fn mask_low(&mut self, n: usize) {
        let words = n / 32;
        let bits = (n % 32) as u32;
        let mut i = words;
        while i < 36 {
            if i == words && bits != 0 { self.limbs[i] &= (1u32 << bits) - 1; } else { self.limbs[i] = 0; }
            i += 1;
        }
    }
    fn mul_small(&mut self, factor: u32) {
        let mut carry = 0u64;
        let mut i = 0;
        while i < 36 {
            let value = self.limbs[i] as u64 * factor as u64 + carry;
            self.limbs[i] = value as u32;
            carry = value >> 32;
            i += 1;
        }
    }
    /// Returns `self >> n` (known to be below 2^32) and keeps the low `n` bits.
    fn take_high(&mut self, n: usize) -> u32 {
        let words = n / 32;
        let bits = n % 32;
        let low = self.limbs[words] as u64;
        let high = if words + 1 < 36 { self.limbs[words + 1] as u64 } else { 0 };
        let value = ((low | (high << 32)) >> bits) as u32;
        self.mask_low(n);
        value
    }
    fn div_small(&mut self, divisor: u32) -> u32 {
        let mut remainder = 0u64;
        let mut i = 36;
        while i > 0 {
            i -= 1;
            let value = (remainder << 32) | self.limbs[i] as u64;
            self.limbs[i] = (value / divisor as u64) as u32;
            remainder = value % divisor as u64;
        }
        remainder as u32
    }
    /// Decimal digits (ASCII, most significant first, `0` for zero) into `out`; returns their count.
    fn write_decimal(mut self, out: &mut [u8]) -> usize {
        let mut reversed = [0u8; 360];
        let mut len = 0;
        loop {
            let mut chunk = self.div_small(1_000_000_000);
            let last = self.is_zero();
            let mut k = 0;
            while k < 9 && (!last || chunk != 0 || k == 0) {
                reversed[len] = b'0' + (chunk % 10) as u8;
                chunk /= 10;
                len += 1;
                k += 1;
            }
            if last { break; }
        }
        let mut i = 0;
        while i < len { out[i] = reversed[len - 1 - i]; i += 1; }
        len
    }
}

/// Adds one unit in the last place of the ASCII digits `digits[..len]` (skipping `.`); returns true when it carried
/// out of the first digit (all digits were 9 and are now 0).
fn round_up(digits: &mut [u8], len: usize) -> bool {
    let mut i = len;
    while i > 0 {
        i -= 1;
        if digits[i] == b'.' { continue; }
        if digits[i] == b'9' { digits[i] = b'0'; } else { digits[i] += 1; return false; }
    }
    true
}

/// Whether discarding `rest` (ASCII digits) after a kept digit `last` rounds up: above half, or exactly half with
/// an odd `last` (ties to even, as Rust's exact float formatting).
fn rounds_up(rest: &[u8], last: u8) -> bool {
    if rest.is_empty() || rest[0] < b'5' { return false; }
    if rest[0] > b'5' { return true; }
    let mut i = 1;
    while i < rest.len() { if rest[i] != b'0' { return true; } i += 1; }
    (last - b'0') % 2 == 1
}

/// `{:.N}` and `{:.Ne}` of a float: the exact binary value expanded to decimal, then rounded ties-to-even.
fn float_exact(val: f64, exp_char: u8, precision: usize, f: &mut Formatter<'_>) -> Result {
    if val.is_nan() { return f.pad_number("", "NaN", 0, ""); }
    let sign = if val.is_sign_negative() { "-" } else if f.sign_plus() { "+" } else { "" };
    if val.is_infinite() { return f.pad_number(sign, "inf", 0, ""); }
    let bits = val.to_bits();
    let biased = ((bits >> 52) & 0x7ff) as i32;
    let fraction = bits & ((1u64 << 52) - 1);
    let (mantissa, exponent) = if biased == 0 { (fraction, -1074) } else { (fraction | (1u64 << 52), biased - 1075) };
    // int: integer-part digits; frac: every fractional digit (a binary fraction terminates in decimal).
    let mut int = [0u8; 320];
    let mut frac = [0u8; 1080];
    let mut frac_len = 0usize;
    let mut big = Big::from_u64(mantissa);
    let int_len = if exponent >= 0 {
        big.shl(exponent as usize);
        big.write_decimal(&mut int)
    } else {
        let shift = (-exponent) as usize;
        let mut whole = Big { limbs: big.limbs };
        whole.shr(shift);
        big.mask_low(shift);
        while !big.is_zero() {
            big.mul_small(10);
            frac[frac_len] = b'0' + big.take_high(shift) as u8;
            frac_len += 1;
        }
        whole.write_decimal(&mut int)
    };
    // out[0] is room for a carry digit.
    let mut out = [0u8; 1408];
    if exp_char == 0 {
        let kept = if precision < frac_len { precision } else { frac_len };
        let mut len = 1;
        let mut i = 0;
        while i < int_len { out[len] = int[i]; len += 1; i += 1; }
        if precision > 0 { out[len] = b'.'; len += 1; }
        i = 0;
        while i < kept { out[len] = frac[i]; len += 1; i += 1; }
        let last = if kept > 0 { frac[kept - 1] } else { int[int_len - 1] };
        let mut start = 1;
        if rounds_up(&frac[kept..frac_len], last) && round_up(&mut out[1..], len - 1) { out[0] = b'1'; start = 0; }
        let body = unsafe { crate::str::from_utf8_unchecked(&out[start..len]) };
        return f.pad_number(sign, body, precision - kept, "");
    }
    // Scientific: significant digits from the first non-zero digit.
    let mut sig = [0u8; 1400];
    let mut sig_len = 0;
    let mut exp10: i32;
    if !(int_len == 1 && int[0] == b'0') {
        exp10 = int_len as i32 - 1;
        let mut i = 0;
        while i < int_len { sig[sig_len] = int[i]; sig_len += 1; i += 1; }
        i = 0;
        while i < frac_len { sig[sig_len] = frac[i]; sig_len += 1; i += 1; }
    } else {
        let mut first = 0;
        while first < frac_len && frac[first] == b'0' { first += 1; }
        exp10 = if first < frac_len { -(first as i32) - 1 } else { 0 };
        let mut i = first;
        while i < frac_len { sig[sig_len] = frac[i]; sig_len += 1; i += 1; }
    }
    if sig_len == 0 { sig[0] = b'0'; sig_len = 1; }
    let keep = precision + 1;
    let kept = if keep < sig_len { keep } else { sig_len };
    if rounds_up(&sig[kept..sig_len], sig[kept - 1]) && round_up(&mut sig, kept) {
        sig[0] = b'1';
        exp10 += 1;
    }
    let mut len = 0;
    out[len] = sig[0]; len += 1;
    if precision > 0 { out[len] = b'.'; len += 1; }
    let mut i = 1;
    while i < kept { out[len] = sig[i]; len += 1; i += 1; }
    let mut suffix = [0u8; 8];
    let mut suffix_len = 0;
    suffix[suffix_len] = exp_char; suffix_len += 1;
    if exp10 < 0 { suffix[suffix_len] = b'-'; suffix_len += 1; }
    let mut magnitude = [0u8; 128];
    let text = digits(if exp10 < 0 { (-exp10) as u128 } else { exp10 as u128 }, 10, false, &mut magnitude).as_bytes();
    i = 0;
    while i < text.len() { suffix[suffix_len] = text[i]; suffix_len += 1; i += 1; }
    let body = unsafe { crate::str::from_utf8_unchecked(&out[..len]) };
    let suffix = unsafe { crate::str::from_utf8_unchecked(&suffix[..suffix_len]) };
    f.pad_number(sign, body, keep - kept, suffix)
}
