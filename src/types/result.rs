use std::error::Error;

pub type Result<T, E = Box<dyn Error>> = std::result::Result<T, E>;
