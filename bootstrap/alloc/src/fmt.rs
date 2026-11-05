pub use core::fmt::*;

struct StringWriter<'a>(&'a mut crate::string::String);

impl Write for StringWriter<'_> {
    fn write_str(&mut self, value: &str) -> Result {
        self.0.push_str(value);
        Ok(())
    }

    fn write_char(&mut self, value: char) -> Result {
        self.0.push(value);
        Ok(())
    }
}

pub fn format(args: core::fmt::Arguments<'_>) -> crate::string::String {
    if let Some(value) = args.as_str() {
        return crate::string::String::from(value);
    }
    let mut output = crate::string::String::new();
    let _ = core::fmt::write(&mut StringWriter(&mut output), args);
    output
}

#[macro_export]
macro_rules! format {
    ($($arg:tt)*) => {{
        alloc::fmt::format(format_args!($($arg)*))
    }};
}
