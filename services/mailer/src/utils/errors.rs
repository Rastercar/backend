use std::fmt::Display;

pub trait ResultExt<T, E>
where
    E: Display,
{
    /// exits the process if result is `Err`, otherwise unwraps `Ok`
    ///
    /// this is desirable when a error should terminate the entire program
    /// instead of just panicking on the current thread.
    fn unwrap_or_exit_process(self) -> T;
}

impl<T, E> ResultExt<T, E> for Result<T, E>
where
    E: Display,
{
    fn unwrap_or_exit_process(self) -> T {
        match self {
            Ok(t) => t,
            Err(err) => {
                eprintln!("[RMQ] fatal error, exiting: {}", err);
                std::process::exit(-1)
            }
        }
    }
}
