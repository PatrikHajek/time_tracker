use std::{env, error::Error};

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    time_tracker::run(&args)?;
    // if let Err(e) = time_tracker::run(config) {
    //     eprintln!("Application error: {e}");
    //     process::exit(1);
    // };
    Ok(())
}
