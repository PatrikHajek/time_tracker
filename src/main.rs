use std::{env, error::Error};

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    time_tracker::run(&args)?;
    Ok(())
}
