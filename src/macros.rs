#[macro_export]
macro_rules! wdebug {
    ($config:expr) => {
        if $config.debug {
            print!("\n")
        }
    };
    ($config:expr, $fmt:expr) => {
        if $config.debug {
            print!(concat!($fmt, "\n"));
            log::debug!($fmt);
        }
    };
    ($config:expr, $fmt:expr, $($arg:tt)*) => {
            if $config.debug {
            print!(concat!($fmt, "\n"), $($arg)*);
            log::debug!($fmt, $($arg)*);
        }
    };
}

#[macro_export]
macro_rules! werror {
    ($fmt:expr) => {
        eprint!("{}", &style("Error: ").red().to_string());
        eprint!(concat!($fmt, "\n"));
        log::error!($fmt);
    };
    ($fmt:expr, $($arg:tt)*) => {
        eprint!("{}", &style("Error: ").red().to_string());
        eprint!(concat!($fmt, "\n"), $($arg)*);
        log::error!($fmt, $($arg)*);
    };
}

#[macro_export]
macro_rules! winfo {
    ($fmt:expr) => {
        print!(concat!($fmt, "\n"));
        log::info!($fmt);
    };
    ($fmt:expr, $($arg:tt)*) => {
        print!(concat!($fmt, "\n"), $($arg)*);
        log::info!($fmt, $($arg)*);
    };
}

#[macro_export]
macro_rules! wwarning {
    ($fmt:expr) => {
        print!("{}", &style("Warning: ").yellow().to_string());
        print!(concat!($fmt, "\n"));
        log::warn!($fmt);
    };
    ($fmt:expr, $($arg:tt)*) => {
        print!("{}", &style("Warning: ").yellow().to_string());
        print!(concat!($fmt, "\n"), $($arg)*);
        log::warn!($fmt, $($arg)*);
    };
}
