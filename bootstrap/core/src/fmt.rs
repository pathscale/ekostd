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
    pub const ALTERNATE: u32 = 1 << 23;
    pub const WIDTH: u32 = 1 << 27;
    pub const PRECISION: u32 = 1 << 28;
    pub const ALIGN_UNKNOWN: u32 = 3 << 29;
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
            // Width, fill, alignment, sign, zero-padding and `x?` are not implemented by the formatting
            // traits here: reject them rather than silently formatting incorrectly. `#` and precision are
            // carried to the impl, which may honour them.
            if options.flags & !(flags::ALTERNATE | flags::PRECISION) != flags::DEFAULT { return Err(Error); }
            attempt!(args.args[index].format(&mut Formatter { writer: output, options }));
        }
    }
    Err(Error)
}
fn number(mut value: u128, radix: u128, upper: bool, f: &mut Formatter<'_>) -> Result {
    let mut bytes = [0u8; 128];
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
    f.write_str(unsafe { crate::str::from_utf8_unchecked(s) })
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
    fn fmt(&self, f: &mut Formatter<'_>) -> Result { f.write_str(if *self { "true" } else { "false" }) }
}
impl Debug for bool { fn fmt(&self, f: &mut Formatter<'_>) -> Result { Display::fmt(self, f) } }
impl Display for char { fn fmt(&self, f: &mut Formatter<'_>) -> Result { f.write_char(*self) } }
impl Debug for char {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        attempt!(f.write_char('\'')); attempt!(escaped(*self, '\'', f)); f.write_char('\'')
    }
}
impl Display for str { fn fmt(&self, f: &mut Formatter<'_>) -> Result { f.write_str(self) } }
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
            fn fmt(&self, f: &mut Formatter<'_>) -> Result {
                let value = *self as $u as u128;
                if $signed && *self < 0 {
                    attempt!(f.write_str("-"));
                    number((!(*self as $u) + 1) as u128, 10, false, f)
                } else { number(value, 10, false, f) }
            }
        }
        impl Debug for $t { fn fmt(&self, f: &mut Formatter<'_>) -> Result { Display::fmt(self, f) } }
        impl LowerHex for $t {
            fn fmt(&self, f: &mut Formatter<'_>) -> Result { number(*self as $u as u128, 16, false, f) }
        }
        impl UpperHex for $t {
            fn fmt(&self, f: &mut Formatter<'_>) -> Result { number(*self as $u as u128, 16, true, f) }
        }
        impl Binary for $t {
            fn fmt(&self, f: &mut Formatter<'_>) -> Result { number(*self as $u as u128, 2, false, f) }
        }
        impl Octal for $t {
            fn fmt(&self, f: &mut Formatter<'_>) -> Result { number(*self as $u as u128, 8, false, f) }
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
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        attempt!(f.write_str("0x")); number(*self as *const () as usize as u128, 16, false, f)
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
    /// Writes `s`. Width, fill and precision are not supported by this formatter and are ignored.
    pub fn pad(&mut self, s: &str) -> Result { self.write_str(s) }
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
    if val.is_nan() {
        return f.write_str("NaN");
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
        return f.write_str(unsafe { crate::str::from_utf8_unchecked(&buf[..pos]) });
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
        return f.write_str(unsafe { crate::str::from_utf8_unchecked(&buf[..pos]) });
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

    f.write_str(unsafe { crate::str::from_utf8_unchecked(&buf[..pos]) })
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
