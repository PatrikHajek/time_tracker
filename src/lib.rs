#[derive(PartialEq, Debug)]
pub struct Config {
    pub action: Action,
}

impl Config {
    pub fn build(args: &[String]) -> Result<Config, String> {
        if args.len() < 2 {
            return Err(String::from("not enough arguments"));
        }

        let action = Action::build(&args[1])?;
        Ok(Config { action })
    }
}

#[derive(PartialEq, Debug)]
pub enum Action {
    Start,
    Stop,
    Mark,
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
                action: Action::Start
            }
        );
    }
}
