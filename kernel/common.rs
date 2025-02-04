use core::fmt::{self, Arguments, Write};

pub fn print_args(args: Arguments) {
    let mut writer = Writer;
    writer
        .write_fmt(args)
        .expect("Something when wrong writing args");
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::common::print_args(format_args!($($arg)*)));
}

struct Writer;

impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        crate::arch::console_write(s);
        Ok(())
    }
}
