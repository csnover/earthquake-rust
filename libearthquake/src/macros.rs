#[macro_export]
macro_rules! assert_sample {
    ($test:expr, $($arg:tt)+) => (
        if !$test {
            $crate::panic_sample!($($arg)+)
        }
    )
}

#[macro_export]
macro_rules! bail_sample {
    ($msg:expr) => ({
        ::anyhow::bail!("{}. Please send this file for analysis.", $msg)
    });
    ($msg:expr,) => ({
        $crate::bail_sample!($msg)
    });
    ($fmt:expr, $($arg:tt)+) => ({
        $crate::bail_sample!(format_args!($fmt, $($arg)+))
    });
}

#[macro_export]
macro_rules! ensure_sample {
    ($test:expr, $msg:expr) => ({
        ::anyhow::ensure!($test, "{}. Please send this file for analysis.", $msg)
    });
    ($test:expr, $msg:expr,) => ({
        $crate::ensure_sample!($test, $msg)
    });
    ($test:expr, $fmt:expr, $($arg:tt)+) => ({
        $crate::ensure_sample!($test, format_args!($fmt, $($arg)+))
    });
}

#[macro_export]
macro_rules! panic_sample {
    ($msg:expr) => ({
        panic!("{}. Please send this file for analysis.", $msg)
    });
    ($msg:expr,) => ({
        $crate::panic_sample!($msg)
    });
    ($fmt:expr, $($arg:tt)+) => ({
        $crate::panic_sample!(format_args!($fmt, $($arg)+))
    });
}
