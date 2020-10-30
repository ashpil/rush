/// Alias of println! but only present in debug builds.
macro_rules! debug_println {
    ($($t:tt)*) => {
        #[cfg(debug_assertions)] // Only include when not built with `--release` flag
        println!($($t)*);
    }
}
