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
    pub sessions_path: String,
}

impl Config {
    pub fn build(args: &[String]) -> Result<Config, String> {
        if args.len() < 2 {
            return Err(String::from("not enough arguments"));
        }

        let action = Action::build(&args[1])?;
        Ok(Config {
            action,
            sessions_path: String::from("./sessions"),
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
    let StartData { path, contents } = start_get_data(config);
    if fs::exists(&path)? {
        return Err("this session file is already created")?;
    };
    fs::write(&path, &contents)?;
    Ok(())
}

struct StartData {
    path: String,
    contents: String,
}
fn start_get_data(config: &Config) -> StartData {
    let date = chrono::Local::now().format("%FT%T%:z").to_string();
    let contents = get_contents(&date);
    let path = format!("{}/{}.md", config.sessions_path, date);
    StartData { path, contents }
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
                sessions_path: String::from("./sessions")
            }
        );
    }

    #[test]
    fn start_get_data_works() {
        let config = Config {
            action: Action::Start,
            sessions_path: String::from("."),
        };
        let StartData { path, contents } = start_get_data(&config);
        let date = &path[2..path.len() - 3];
        assert_eq!(path, format!("./{}.md", date));
        assert!(contents.contains(&format!("# {}", date)));
    }
}
