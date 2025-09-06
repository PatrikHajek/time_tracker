use session::{Aggregator, Label, Session, SessionFile};

use crate::date_time::DateTime;
use std::{env, error::Error, fs, io, path::PathBuf, process::Command};

mod date_time;
mod session;
#[cfg(test)]
mod test_utils;

const CONFIG_PATH: &str = "~/.timetracker.toml";
const CONFIG_SESSIONS_PATH: &str = "sessions_path";

#[derive(PartialEq, Debug)]
pub struct Config {
    action: Action,
    sessions_path: PathBuf,
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
enum Action {
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
                    date: DateTime::now().modify_by_relative_input(&args[0])?,
                },
                _ => return Err("too many arguments")?,
            },
            "stop" => match args.len() {
                0 => Action::Stop {
                    date: DateTime::now(),
                },
                1 => Action::Stop {
                    date: DateTime::now().modify_by_relative_input(&args[0])?,
                },
                _ => return Err("too many arguments")?,
            },
            "mark" => match args.len() {
                0 => Action::Mark {
                    date: DateTime::now(),
                },
                1 => Action::Mark {
                    date: DateTime::now().modify_by_relative_input(&args[0])?,
                },
                _ => return Err("too many arguments")?,
            },
            "remark" => match args.len() {
                0 => Action::Remark {
                    date: DateTime::now(),
                },
                1 => Action::Remark {
                    date: DateTime::now().modify_by_relative_input(&args[0])?,
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

// TODO: put all these functions inside Action?
pub fn run(config: Config) -> Result<(), Box<dyn Error>> {
    let result = match config.action {
        Action::Start { .. } => start(&config),
        Action::Stop { .. } => stop(&config),
        Action::Mark { .. } => mark(&config),
        Action::Remark { .. } => remark(&config),
        Action::Path => path(&config),
        Action::View => view(&config),
        Action::Label { .. } => label(&config),
        Action::Unlabel { .. } => unlabel(&config),
        Action::Write { .. } => write(&config),
        Action::Version => Ok(version()),
    };
    Ok(result?)
}

fn start(config: &Config) -> Result<(), Box<dyn Error>> {
    if let Some(session) = Session::get_last(&config)? {
        if session.is_active() {
            return Err("another session is already active")?;
        }
    }
    let Action::Start { date } = &config.action else {
        panic!("wrong action, expected Action::Start");
    };

    let session = Session::new(&config, &date);
    let SessionFile { path, contents } = session.to_file()?;
    if fs::exists(&path)? {
        return Err("this session file is already created")?;
    };
    fs::write(&path, &contents).map_err(|_| "session directory doesn't exist")?;
    println!("Started: {}", &date.to_formatted_time());
    Ok(())
}

fn stop(config: &Config) -> Result<(), Box<dyn Error>> {
    let Some(mut session) = Session::get_last(&config)? else {
        return Err("no active session found")?;
    };
    let Action::Stop { date } = &config.action else {
        panic!("wrong action, expected Action::Stop");
    };

    session.stop(&date)?;
    session.save()?;
    println!(
        "Stopped: {}\n{}",
        &date.to_formatted_time(),
        &Aggregator::build(&config)?
            .view()
            .lines()
            .skip(1)
            .fold(String::new(), |acc, val| acc + val + "\n")
            .trim_end_matches("\n")
    );
    Ok(())
}

fn mark(config: &Config) -> Result<(), Box<dyn Error>> {
    let Some(mut session) = Session::get_last(&config)? else {
        return Err("no active session found")?;
    };
    let Action::Mark { date } = &config.action else {
        panic!("wrong action, expected Action::Mark");
    };

    session.mark(&date)?;
    session.save()?;
    println!("Marked: {}", &date.to_formatted_time());
    Ok(())
}

fn remark(config: &Config) -> Result<(), Box<dyn Error>> {
    let Some(mut session) = Session::get_last(&config)? else {
        return Err("no active session found")?;
    };
    let Action::Remark { date } = &config.action else {
        panic!("wrong action, expected Action::Remark");
    };

    session.remark(&date);
    session.save()?;
    println!("Remarked to: {}", &date.to_formatted_time());
    Ok(())
}

fn path(config: &Config) -> Result<(), Box<dyn Error>> {
    let Some(session) = Session::get_last(&config)? else {
        return Err("no active session found")?;
    };
    print!("{}", session.path.to_str().ok_or("failed to convert path")?);
    Ok(())
}

fn view(config: &Config) -> Result<(), Box<dyn Error>> {
    let aggregator = Aggregator::build(&config)?;
    println!("{}", aggregator.view());
    Ok(())
}

fn label(config: &Config) -> Result<(), Box<dyn Error>> {
    let Some(mut session) = Session::get_last(&config)? else {
        // TODO: message should be more like: "no session found", change all other occurrences
        return Err("no active session found")?;
    };
    let Action::Label { label } = &config.action else {
        panic!("wrong action, expected Action::Label");
    };
    let was_added = session.label(&label);
    session.save()?;
    if was_added {
        println!("Added label: {label:?}");
    } else {
        println!("Label `{label:?}` already present");
    }
    Ok(())
}

fn unlabel(config: &Config) -> Result<(), Box<dyn Error>> {
    let Some(mut session) = Session::get_last(&config)? else {
        return Err("no active session found")?;
    };
    let Action::Unlabel { label } = &config.action else {
        panic!("wrong action, expected Action::Unlabel");
    };
    let was_removed = session.unlabel(&label);
    session.save()?;
    if was_removed {
        println!("Removed label: {label:?}");
    } else {
        println!("Label `{label:?}` not present")
    }
    Ok(())
}

fn write(config: &Config) -> Result<(), Box<dyn Error>> {
    let Some(mut session) = Session::get_last(&config)? else {
        return Err("no active session found")?;
    };
    let Action::Write { text } = &config.action else {
        panic!("wrong action, expected Action::Write");
    };
    let has_failed = session.write(&text).is_err();
    if has_failed {
        println!("Current mark already contains some text, do you want to overwrite it? (y/n)");
        let mut buf = String::new();
        io::stdin().read_line(&mut buf)?;
        if buf == "\n" || buf == "y\n" {
            session
                .marks
                .last_mut()
                .expect("session must always have at least one mark")
                .erase();
            session.write(&text).expect("content is erased");
        } else {
            println!("Action cancelled");
            return Ok(());
        }
    }
    session.save()?;
    println!("Wrote:\n{text}");
    Ok(())
}

fn version() {
    let version = env!("CARGO_PKG_VERSION");
    println!("v{version}");
}

fn resolve_path(path: &str) -> Result<PathBuf, &'static str> {
    let path = path.trim();
    if path.starts_with("~") {
        if !path.starts_with("~/") {
            return Err("invalid path, `~` is not followed by `/`");
        }
        let Ok(home) = env::var("HOME") else {
            return Err("failed to interpret environment variable HOME");
        };
        let path = PathBuf::from(home).join(&path[2..]);
        return Ok(path);
    }
    Ok(PathBuf::from(path))
}

// TODO: move to Aggregator
fn read_sessions_dir(config: &Config) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let mut dir = fs::read_dir(&config.sessions_path)
        .map_err(|_err| "session directory doesn't exist")?
        .map(|res| res.map(|v| v.path()))
        .collect::<Result<Vec<_>, io::Error>>()?;
    dir.sort();
    Ok(dir)
}

fn get_git_branch_name() -> Result<String, Box<dyn Error>> {
    let output = Command::new("git")
        .args(["branch", "--show-current"])
        .output()?;
    if output.status.success() {
        let name = String::from_utf8(output.stdout)?.trim().to_owned();
        if name.is_empty() {
            return Err("branch name is empty")?;
        }
        Ok(name)
    } else {
        let error_message = String::from_utf8(output.stderr)?;
        Err(format!("Failed to get git branch name: {error_message}"))?
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
