use std::{env, error::Error, process};

use time_tracker::Config;

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    let config = Config::build(&args).unwrap_or_else(|err| {
        eprintln!("Problem parsing arguments: {err}");
        process::exit(1);
    });

    time_tracker::run(config)?;
    // if let Err(e) = time_tracker::run(config) {
    //     eprintln!("Application error: {e}");
    //     process::exit(1);
    // };
    Ok(())
}
