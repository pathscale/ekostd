//! Allocation-free formatting for the bootstrap runtime.
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
pub struct Formatter<'a> { writer: &'a mut dyn Write }
impl<'a> Formatter<'a> {
    pub fn write_str(&mut self, s: &str) -> Result { self.writer.write_str(s) }
    pub fn write_char(&mut self, c: char) -> Result { self.writer.write_char(c) }
    pub fn write_fmt(&mut self, args: Arguments<'_>) -> Result { write(self.writer, args) }
    pub fn debug_map(&mut self) -> DebugList<'_, 'a> { DebugList { fmt: self } }
    pub fn debug_set(&mut self) -> DebugList<'_, 'a> { DebugList { fmt: self } }
    pub fn debug_list(&mut self) -> DebugList<'_, 'a> { DebugList { fmt: self } }
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
pub struct DebugList<'b, 'a: 'b> { fmt: &'b mut Formatter<'a> }
impl<'b, 'a: 'b> DebugList<'b, 'a> {
    pub fn entries<I>(&mut self, _entries: I) -> &mut Self { self }
    pub fn finish(&mut self) -> Result { self.fmt.write_str("{..}") }
}
#[rustc_diagnostic_item = "Display"]
pub trait Display { fn fmt(&self, f: &mut Formatter<'_>) -> Result; }
#[rustc_diagnostic_item = "Debug"]
pub trait Debug { fn fmt(&self, f: &mut Formatter<'_>) -> Result; }
pub trait LowerHex { fn fmt(&self, f: &mut Formatter<'_>) -> Result; }
pub trait UpperHex { fn fmt(&self, f: &mut Formatter<'_>) -> Result; }
pub trait Pointer { fn fmt(&self, f: &mut Formatter<'_>) -> Result; }

impl Debug for () {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result { f.write_str("()") }
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
        Pointer(&'a dyn Pointer), Count(usize),
    }
    impl<'a> Argument<'a> {
        pub const fn new_display<T: Display>(v: &'a T) -> Self { Self { kind: Kind::Display(v) } }
        pub const fn new_debug<T: Debug>(v: &'a T) -> Self { Self { kind: Kind::Debug(v) } }
        pub const fn new_lower_hex<T: LowerHex>(v: &'a T) -> Self { Self { kind: Kind::LowerHex(v) } }
        pub const fn new_upper_hex<T: UpperHex>(v: &'a T) -> Self { Self { kind: Kind::UpperHex(v) } }
        pub const fn new_pointer<T: Pointer>(v: &'a T) -> Self { Self { kind: Kind::Pointer(v) } }
        pub const fn from_usize(v: &'a usize) -> Self { Self { kind: Kind::Count(*v) } }
        pub(super) fn format(&self, f: &mut Formatter<'_>) -> Result {
            match &self.kind {
                Kind::Display(v) => Display::fmt(*v, f), Kind::Debug(v) => Debug::fmt(*v, f),
                Kind::LowerHex(v) => LowerHex::fmt(*v, f), Kind::UpperHex(v) => UpperHex::fmt(*v, f),
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
    pub unsafe fn new<const N: usize, const M: usize>(
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
            // Bootstrap supports default placeholders and explicit argument indices.
            // Reject unsupported options rather than silently formatting them incorrectly.
            if tag < 192 || tag & 0x37 != 0 { return Err(Error); }
            let index = if tag & 8 != 0 {
                if bytes.len() - offset < 2 { return Err(Error); }
                let index = bytes[offset] as usize | ((bytes[offset + 1] as usize) << 8);
                offset += 2;
                index
            } else { let index = next; next += 1; index };
            if index >= args.args.len() { return Err(Error); }
            attempt!(args.args[index].format(&mut Formatter { writer: output }));
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

