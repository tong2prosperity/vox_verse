use slog::{o, Drain, Logger};
use once_cell::sync::Lazy;

use std::sync::Mutex;

pub static GLOBAL_LOGGER: Lazy<Logger> = Lazy::new(|| {
    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::FullFormat::new(decorator).build().fuse();
    let drain = Mutex::new(drain).fuse();
    Logger::root(drain, o!())
});


// 定义自己的宏
#[macro_export]
macro_rules! info {
    ($($arg:tt)+) => {
        slog::info!($crate::utils::log::GLOBAL_LOGGER, $($arg)+)
    }
}

#[macro_export]
macro_rules! debug {
    ($($arg:tt)+) => {
        slog::debug!($crate::utils::log::GLOBAL_LOGGER, $($arg)+)
    }
}

#[macro_export]
macro_rules! error {
    ($($arg:tt)+) => {
        slog::error!($crate::utils::log::GLOBAL_LOGGER, $($arg)+)
    }
}

#[macro_export]
macro_rules! warn {
    ($($arg:tt)+) => {
        slog::warn!($crate::utils::log::GLOBAL_LOGGER, $($arg)+)
    }
}