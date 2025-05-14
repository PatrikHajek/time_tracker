// TODO: auto-create session directory if missing

use std::{
    error::Error,
    fs, io,
    path::{Path, PathBuf},
    str::FromStr,
};

const SESSION_HEADING_PREFIX: &str = "# ";
const MARKS_HEADING: &str = "## Marks";
const MARK_HEADING_PREFIX: &str = "### ";

fn get_contents(date: &str) -> String {
    format!(
        "\
{SESSION_HEADING_PREFIX}{date}

{MARKS_HEADING}
"
    )
}

#[derive(PartialEq, Debug)]
pub struct Config {
    pub action: Action,
    pub sessions_path: PathBuf,
}

impl Config {
    pub fn build(args: &[String]) -> Result<Config, String> {
        if args.len() < 2 {
            return Err(String::from("not enough arguments"));
        }

        let action = Action::build(&args[1])?;
        Ok(Config {
            action,
            sessions_path: PathBuf::from("./sessions"),
        })
    }
}

#[derive(PartialEq, Debug)]
pub enum Action {
    Start,
    Stop,
    Mark,
    // Set,
    // View
}

impl Action {
    fn build(name: &str) -> Result<Action, String> {
        let out = match name {
            "start" => Action::Start,
            "stop" => Action::Stop,
            "mark" => Action::Mark,
            name => return Err(format!("unrecognized command `{name}`")),
        };
        Ok(out)
    }
}

struct SessionFile {
    contents: String,
}
impl SessionFile {
    fn build(contents: String) -> Result<SessionFile, &'static str> {
        let contents = contents.trim();
        if !contents.starts_with(SESSION_HEADING_PREFIX) || !contents.contains(MARKS_HEADING) {
            return Err("couldn't parse session file");
        }
        Ok(SessionFile {
            contents: contents.to_string(),
        })
    }

    fn get_heading_with_contents(heading: &str, contents: &str) -> String {
        let heading_level = SessionFile::get_heading_level(&heading);
        let mut is_within = false;
        let mut text = String::new();
        for line in contents.lines() {
            if line.starts_with("#") && SessionFile::get_heading_level(&line) <= heading_level {
                is_within = false;
            }
            if line.starts_with(&heading) {
                is_within = true;
            }
            if is_within {
                text += &line.trim();
                text += "\n";
            }
        }

        text
    }

    fn get_heading_level(heading: &str) -> u8 {
        let mut level = 0u8;
        for char in heading.trim().chars() {
            if char != '#' {
                break;
            }
            level += 1;
        }
        level
    }
}

struct Session {
    is_active: bool,
    start: chrono::DateTime<chrono::Local>,
    marks: Vec<Mark>,
}

impl Session {
    fn get_active(config: &Config) -> Result<Session, Box<dyn Error>> {
        let dir = fs::read_dir(&config.sessions_path)
            .map_err(|_err| "session directory doesn't exist")?
            .map(|res| res.map(|v| v.path()))
            .collect::<Result<Vec<_>, io::Error>>()?;
        if dir.len() == 0 {
            return Err("there is no active session")?;
        }
        let contents = fs::read_to_string(&dir[dir.len() - 1])?;
        Session::parse(contents)
    }

    fn parse(contents: String) -> Result<Session, Box<dyn Error>> {
        let file = SessionFile::build(contents)?;
        let start: chrono::DateTime<chrono::Local> = file
            .contents
            .lines()
            .next()
            .map(|val| chrono::DateTime::from_str(&val[SESSION_HEADING_PREFIX.len()..val.len()]))
            .ok_or("couldn't extract date from heading")??;

        let mut marks: Vec<Mark> = Vec::new();
        let marks_contents = SessionFile::get_heading_with_contents(MARKS_HEADING, &file.contents);
        for line in marks_contents.lines() {
            if line.starts_with(MARK_HEADING_PREFIX) {
                let contents = SessionFile::get_heading_with_contents(&line, &marks_contents);
                let date = contents
                    .lines()
                    .next()
                    .map(|val| {
                        chrono::DateTime::from_str(&val[MARK_HEADING_PREFIX.len()..val.len()])
                    })
                    .ok_or("couldn't parse mark heading")??;
                marks.push(Mark { date, contents });
            }
        }

        Ok(Session {
            is_active: true,
            start,
            marks,
        })
    }

    fn save(&self) -> Result<(), io::Error> {
        todo!()
    }

    fn mark(&mut self) {
        let dt = DateTime::now();
        let mark = Mark {
            date: dt.date,
            contents: format!("{MARK_HEADING_PREFIX}{}", dt.formatted),
        };
        self.marks.push(mark);
    }
}

struct Mark {
    date: chrono::DateTime<chrono::Local>,
    contents: String,
}

pub fn run(config: Config) -> Result<(), Box<dyn Error>> {
    let result = match config.action {
        Action::Start => start(&config),
        Action::Stop => stop(),
        Action::Mark => mark(&config),
    };
    Ok(result?)
}

fn start(config: &Config) -> Result<(), Box<dyn Error>> {
    let StartData { path, contents } = start_get_data(config);
    if fs::exists(&path)? {
        return Err("this session file is already created")?;
    };
    fs::write(&path, &contents).map_err(|_| "session directory doesn't exist")?;
    Ok(())
}

struct StartData {
    path: PathBuf,
    contents: String,
}
// TODO: move into Session
fn start_get_data(config: &Config) -> StartData {
    let dt = DateTime::now();
    let contents = get_contents(&dt.formatted);
    let path = Path::join(&config.sessions_path, format!("{}.md", dt.formatted));
    StartData { path, contents }
}

fn stop() -> Result<(), Box<dyn Error>> {
    todo!();
}

fn mark(config: &Config) -> Result<(), Box<dyn Error>> {
    let mut session = Session::get_active(&config)?;
    session.mark();
    todo!();
    // Ok(())
}

struct DateTime {
    date: chrono::DateTime<chrono::Local>,
    formatted: String,
}
impl DateTime {
    fn now() -> DateTime {
        let date = chrono::Local::now();
        DateTime {
            date,
            formatted: DateTime::format(date),
        }
    }

    fn format(date: chrono::DateTime<chrono::Local>) -> String {
        date.format("%FT%T%:z").to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_build_works() {
        let args = &[String::from("time_tracker"), String::from("start")];
        let result = Config::build(args).unwrap();
        assert_eq!(
            result,
            Config {
                action: Action::Start,
                sessions_path: PathBuf::from("./sessions")
            }
        );
    }

    #[test]
    fn start_get_data_works() {
        let config = Config {
            action: Action::Start,
            sessions_path: PathBuf::from("."),
        };
        let StartData { path, contents } = start_get_data(&config);
        let path = path.to_str().unwrap();
        let date = &path[2..path.len() - 3];
        assert_eq!(path, format!("./{}.md", date));
        assert!(contents.contains(&format!("# {}", date)));
    }

    #[test]
    fn action_build_works() {}

    #[test]
    fn session_file_build_works() {}

    #[test]
    fn session_file_get_heading_with_contents_works() {}

    #[test]
    fn session_file_get_heading_level_works() {}

    #[test]
    fn session_parse_works() {}

    #[test]
    fn session_mark() {}
}
