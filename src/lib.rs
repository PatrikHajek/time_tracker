use std::{error::Error, fs};

fn get_contents(date: &str) -> String {
    format!(
        "\
# {date}
"
    )
}

#[derive(PartialEq, Debug)]
pub struct Config {
    pub action: Action,
    pub session_path: String,
}

impl Config {
    pub fn build(args: &[String]) -> Result<Config, String> {
        if args.len() < 2 {
            return Err(String::from("not enough arguments"));
        }

        let action = Action::build(&args[1])?;
        Ok(Config {
            action,
            session_path: String::from("./sessions"),
        })
    }
}

#[derive(PartialEq, Debug)]
pub enum Action {
    Start,
    Stop,
    // Mark,
    // Set,
    // View
}

impl Action {
    fn build(name: &str) -> Result<Action, String> {
        let out = match name {
            "start" => Action::Start,
            "stop" => Action::Stop,
            name => return Err(format!("unrecognized command `{name}`")),
        };
        Ok(out)
    }
}

pub fn run(config: Config) -> Result<(), Box<dyn Error>> {
    let result = match config.action {
        Action::Start => start(&config),
        Action::Stop => stop(),
    };
    Ok(result?)
}

fn start(config: &Config) -> Result<(), Box<dyn Error>> {
    let date = chrono::Local::now().format("%FT%T%:z");
    let contents = get_contents(&date.to_string());
    let path = format!("{}/{}.md", config.session_path, date);
    if fs::exists(&path)? {
        return Err("this session file is already created")?;
    };
    fs::write(&path, &contents)?;
    Ok(())
}

fn stop() -> Result<(), Box<dyn Error>> {
    todo!();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_config() {
        let args = &[String::from("time_tracker"), String::from("start")];
        let result = Config::build(args).unwrap();
        assert_eq!(
            result,
            Config {
                action: Action::Start,
                session_path: String::from("./sessions")
            }
        );
    }

    #[test]
    fn start_works() -> Result<(), Box<dyn Error>> {
        let path = "./temp/start_works";
        fs::remove_dir_all(&path[..6]).unwrap_or(());
        fs::create_dir_all(path)?;
        let config = Config {
            action: Action::Start,
            session_path: String::from(path),
        };
        start(&config)?;
        let dir = fs::read_dir(path)?;
        assert_eq!(dir.count(), 1);
        fs::remove_dir_all(&path[..6])?;
        Ok(())
    }
}
