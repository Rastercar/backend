use std::fmt::Display;

pub fn exit_on_err<T, E>(result: Result<T, E>) -> T
where
    E: Display,
{
    match result {
        Ok(t) => t,
        Err(err) => {
            println!("[RMQ] fatal error, exiting: {}", err);
            std::process::exit(-1)
        }
    }
}
