use std::fmt::Debug;

pub trait ErrorLoggable {
    fn log_on_error(&self);
}

impl<T, E: Debug> ErrorLoggable for Result<T, E> {
    fn log_on_error(&self) {
        match self {
            Ok(_) => {}
            Err(e) => {
                eprintln!("Error occurred: {:?}", e);
            }
        }
    }
}
