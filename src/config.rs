use crate::{date_time::DateTime, get_git_branch_name, resolve_path, session::Label};
use std::{error::Error, fs, io, path::PathBuf};

const CONFIG_PATH: &str = "~/.timetracker.toml";
const CONFIG_SESSIONS_PATH: &str = "sessions_path";

#[derive(PartialEq, Debug)]
pub struct Config {
    // TODO: Remove, instead pass the actual values stored in the Action to the functions.
    pub action: Action,
    pub sessions_path: PathBuf,
}

impl Config {
    pub fn build(args: &[String]) -> Result<Config, Box<dyn Error>> {
        let from_args = Config::from_args(&args)?;
        let path = resolve_path(CONFIG_PATH)?;
        let contents = match fs::read_to_string(&path) {
            Ok(val) => val,
            Err(err) => {
                if err.kind() == io::ErrorKind::NotFound {
                    let contents = format!("{CONFIG_SESSIONS_PATH}=''");
                    fs::write(&path, &contents)?;
                    return Err(format!(
                        "config file not found, created one at `{CONFIG_PATH}`"
                    ))?;
                }
                return Err(err)?;
            }
        };
        let from_file = Config::from_file(&contents)?;
        let config = Config {
            action: from_args.action,
            sessions_path: from_file.sessions_path,
        };
        Ok(config)
    }

    fn from_args(args: &[String]) -> Result<Config, Box<dyn Error>> {
        if args.len() < 2 {
            return Err("not enough arguments")?;
        }

        let action = Action::build(&args[1], &args[2..])?;
        let config = Config {
            action,
            sessions_path: PathBuf::from(""),
        };
        Ok(config)
    }

    fn from_file(contents: &str) -> Result<Config, String> {
        let contents = contents.trim();
        if !contents.starts_with(&format!("{CONFIG_SESSIONS_PATH}='")) || !contents.ends_with("'") {
            return Err(format!(
                "wrong config file format, please use `{CONFIG_SESSIONS_PATH}='<path>'`"
            ))?;
        }
        let sessions_path = &contents[15..contents.len() - 1];
        if sessions_path.is_empty() {
            return Err("wrong config, sessions_path is empty")?;
        }
        let sessions_path = resolve_path(&sessions_path)?;
        let config = Config {
            action: Action::Start {
                date: DateTime::now(),
            },
            sessions_path,
        };
        Ok(config)
    }
}

#[derive(PartialEq, Debug)]
pub enum Action {
    Start { date: DateTime },
    Stop { date: DateTime },
    Mark { date: DateTime },
    Remark { date: DateTime },
    Path,
    View,
    Label { label: Label },
    Unlabel { label: Label },
    Write { text: String },
    Version,
    // Set,
}

impl Action {
    fn build(name: &str, args: &[String]) -> Result<Action, Box<dyn Error>> {
        let out = match name {
            "start" => match args.len() {
                0 => Action::Start {
                    date: DateTime::now(),
                },
                1 => Action::Start {
                    date: DateTime::now().modify(&args[0])?,
                },
                _ => return Err("too many arguments")?,
            },
            "stop" => match args.len() {
                0 => Action::Stop {
                    date: DateTime::now(),
                },
                1 => Action::Stop {
                    date: DateTime::now().modify(&args[0])?,
                },
                _ => return Err("too many arguments")?,
            },
            "mark" => match args.len() {
                0 => Action::Mark {
                    date: DateTime::now(),
                },
                1 => Action::Mark {
                    date: DateTime::now().modify(&args[0])?,
                },
                _ => return Err("too many arguments")?,
            },
            "remark" => match args.len() {
                0 => Action::Remark {
                    date: DateTime::now(),
                },
                1 => Action::Remark {
                    date: DateTime::now().modify(&args[0])?,
                },
                _ => return Err("too many arguments")?,
            },
            "path" => {
                if args.len() != 0 {
                    return Err("too many arguments")?;
                }
                Action::Path
            }
            "view" => {
                if args.len() != 0 {
                    return Err("too many arguments")?;
                }
                Action::View
            }
            "label" | "unlabel" => {
                if args.len() == 0 {
                    return Err("no label specified")?;
                }
                // TODO: forbid adding Label::End?
                let label = Label::from_args(&args)?;
                match name {
                    "label" => Action::Label { label },
                    "unlabel" => Action::Unlabel { label },
                    x => panic!("unreachable Action::Label pattern {x}"),
                }
            }
            "write" => {
                if args.len() == 0 {
                    return Err("no text specified")?;
                } else if args.len() > 1 {
                    return Err("too many arguments")?;
                }
                let text = match args[0].trim() {
                    "-b" => get_git_branch_name()?,
                    text => text.to_owned(),
                };
                Action::Write { text }
            }
            "version" => {
                if args.len() != 0 {
                    return Err("too many arguments")?;
                }
                Action::Version
            }
            name => return Err(format!("unrecognized command `{name}`"))?,
        };
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_from_args_works() {
        let args = &[String::from("time_tracker"), String::from("start")];
        let config = Config {
            action: Action::Start {
                date: DateTime::now(),
            },
            sessions_path: PathBuf::from(""),
        };
        assert_eq!(Config::from_args(args).unwrap(), config);
    }

    #[test]
    fn config_from_file_works() {
        let path = "./notes/sessions";
        let contents = format!("{CONFIG_SESSIONS_PATH}='{path}'");
        let config = Config {
            action: Action::Start {
                date: DateTime::now(),
            },
            sessions_path: PathBuf::from(&path),
        };
        assert_eq!(Config::from_file(&contents).unwrap(), config);
    }

    #[test]
    fn config_from_file_fails_when_wrong_format() {
        assert!(Config::from_file("sessions_path=''")
            .unwrap_err()
            .contains("sessions_path is empty"));
    }

    #[test]
    fn action_build_works() -> Result<(), Box<dyn Error>> {
        assert_eq!(
            Action::build("start", &[])?,
            Action::Start {
                date: DateTime::now()
            }
        );
        assert_eq!(
            Action::build("start", &[String::from("0m")])?,
            Action::Start {
                date: DateTime::now()
            }
        );
        assert!(Action::build("start", &[String::from("0m"), String::from("hello")]).is_err());
        assert!(Action::build("start", &[String::from("hello")]).is_err());

        assert_eq!(
            Action::build("stop", &[])?,
            Action::Stop {
                date: DateTime::now()
            }
        );
        assert_eq!(
            Action::build("stop", &[String::from("0m")])?,
            Action::Stop {
                date: DateTime::now()
            }
        );
        assert!(Action::build("stop", &[String::from("0m"), String::from("hello")]).is_err());
        assert!(Action::build("stop", &[String::from("hello")]).is_err());

        assert_eq!(
            Action::build("mark", &[])?,
            Action::Mark {
                date: DateTime::now()
            }
        );
        assert_eq!(
            Action::build("mark", &[String::from("0m")])?,
            Action::Mark {
                date: DateTime::now()
            }
        );
        assert!(Action::build("mark", &[String::from("0m"), String::from("hello")]).is_err());
        assert!(Action::build("mark", &[String::from("hello")]).is_err());

        assert_eq!(
            Action::build("remark", &[])?,
            Action::Remark {
                date: DateTime::now()
            }
        );
        assert_eq!(
            Action::build("remark", &[String::from("0m")])?,
            Action::Remark {
                date: DateTime::now()
            }
        );
        assert!(Action::build("remark", &[String::from("0m"), String::from("hello")]).is_err());
        assert!(Action::build("remark", &[String::from("hello")]).is_err());

        assert_eq!(Action::build("path", &[])?, Action::Path);
        assert!(Action::build("path", &[String::from("hello")]).is_err());

        assert_eq!(Action::build("view", &[])?, Action::View);
        assert!(Action::build("view", &[String::from("hello")]).is_err());

        assert!(Action::build("label", &[]).is_err());
        assert!(Action::build("label", &[String::from("hello")]).is_err());
        assert!(Action::build("label", &[String::from("skip"), String::from("hello")]).is_err());
        assert_eq!(
            Action::build("label", &[String::from("skip")])?,
            Action::Label { label: Label::Skip }
        );

        assert!(Action::build("unlabel", &[]).is_err());
        assert!(Action::build("unlabel", &[String::from("hello")]).is_err());
        assert!(Action::build("unlabel", &[String::from("skip"), String::from("hello")]).is_err());
        assert_eq!(
            Action::build("unlabel", &[String::from("skip")])?,
            Action::Unlabel { label: Label::Skip }
        );

        assert!(Action::build("write", &[]).is_err());
        assert!(Action::build("write", &[String::from("hello"), String::from("bye")]).is_err());
        assert_eq!(
            Action::build("write", &[String::from("this is content")]).unwrap(),
            Action::Write {
                text: String::from("this is content")
            }
        );

        assert_eq!(Action::build("version", &[])?, Action::Version);
        assert!(Action::build("version", &[String::from("hello")]).is_err());

        assert!(Action::build("hello", &[]).is_err());

        Ok(())
    }
}
