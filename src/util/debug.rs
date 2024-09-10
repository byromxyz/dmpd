pub static DEBUG: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => {
        if $crate::util::debug::DEBUG.load(std::sync::atomic::Ordering::Relaxed) {
            println!("[DEBUG] {}", format_args!($($arg)*));
        }
    };
}
