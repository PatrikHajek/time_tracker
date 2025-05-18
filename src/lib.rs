use std::{
    error::Error,
    fs, io,
    path::{Path, PathBuf},
    str::FromStr,
};

use chrono::Timelike;

const SESSION_HEADING_PREFIX: &str = "# ";
const MARKS_HEADING: &str = "## Marks";
const MARK_HEADING_PREFIX: &str = "### ";

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

#[derive(PartialEq, Debug)]
struct SessionFile {
    path: PathBuf,
    contents: String,
}
impl SessionFile {
    // TODO: add checks for path
    fn build(path: &PathBuf, contents: &str) -> Result<SessionFile, &'static str> {
        let contents = contents.trim();
        if !contents.starts_with(SESSION_HEADING_PREFIX) || !contents.contains(MARKS_HEADING) {
            return Err("couldn't parse session file");
        }
        Ok(SessionFile {
            path: path.clone(),
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

        text.trim().to_string()
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

    fn get_template(date: &str) -> String {
        format!(
            "\
            {SESSION_HEADING_PREFIX}{date}\n\
            \n\
            {MARKS_HEADING}\n\
            \n\
            {MARK_HEADING_PREFIX}{date}\
            "
        )
    }
}

#[derive(PartialEq, Debug)]
struct Session {
    path: PathBuf,
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
        let path = &dir[dir.len() - 1];
        let contents = fs::read_to_string(&path)?;
        let file = SessionFile::build(&path, &contents)?;
        Session::from_file(&file)
    }

    fn from_file(file: &SessionFile) -> Result<Session, Box<dyn Error>> {
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
                let mark = Mark::build(&contents)?;
                marks.push(mark);
            }
        }

        Ok(Session {
            path: file.path.clone(),
            is_active: true,
            start,
            marks,
        })
    }

    fn save(&self, config: &Config) -> Result<(), Box<dyn Error>> {
        let file = self.to_file(&config)?;
        fs::write(&file.path, &file.contents).map_err(|e| format!("coudln't save session: {e}"))?;
        Ok(())
    }

    fn to_file(&self, config: &Config) -> Result<SessionFile, &'static str> {
        let date = DateTime::format(&self.start);
        let mut contents = format!(
            "\
            {SESSION_HEADING_PREFIX}{date}\n\
            \n\
            {MARKS_HEADING}\n\
            \n\
            "
        );
        for mark in &self.marks {
            contents += &mark.to_string();
            contents += "\n\n";
        }

        let file_name = format!("{}.md", date);
        let path = config.sessions_path.join(&file_name);

        let file = SessionFile::build(&path, &contents)?;
        Ok(file)
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

#[derive(PartialEq, Debug)]
struct Mark {
    date: chrono::DateTime<chrono::Local>,
    contents: String,
}

impl Mark {
    fn build(contents: &str) -> Result<Mark, Box<dyn Error>> {
        let contents = contents.trim();
        let date = contents
            .lines()
            .next()
            .map(|val| chrono::DateTime::from_str(&val[MARK_HEADING_PREFIX.len()..val.len()]))
            .ok_or("couldn't parse mark heading")??;
        let contents_without_heading = contents
            .lines()
            .skip(1)
            .fold(String::new(), |acc, val| acc + "\n" + val)
            .trim()
            .to_string();
        Ok(Mark {
            date,
            contents: contents_without_heading,
        })
    }

    fn to_string(&self) -> String {
        let mut contents = format!("{MARK_HEADING_PREFIX}{}", DateTime::format(&self.date));
        let trimmed = self.contents.trim();
        if !trimmed.is_empty() {
            contents += "\n\n";
            contents += &trimmed;
        }
        contents
    }
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

// TODO: remove, this is SessionFile
struct StartData {
    path: PathBuf,
    contents: String,
}
// TODO: move into Session
fn start_get_data(config: &Config) -> StartData {
    let dt = DateTime::now();
    let contents = SessionFile::get_template(&dt.formatted);
    let path = Path::join(&config.sessions_path, format!("{}.md", dt.formatted));
    StartData { path, contents }
}

fn stop() -> Result<(), Box<dyn Error>> {
    todo!();
}

fn mark(config: &Config) -> Result<(), Box<dyn Error>> {
    let mut session = Session::get_active(&config)?;
    session.mark();
    session.save(&config)?;
    Ok(())
}

struct DateTime {
    date: chrono::DateTime<chrono::Local>,
    formatted: String,
}
impl DateTime {
    fn now() -> DateTime {
        let now = chrono::Local::now();
        let date =
            chrono::NaiveDateTime::new(now.date_naive(), now.time().with_nanosecond(0).unwrap())
                .and_local_timezone(now.timezone())
                .unwrap();
        DateTime {
            date,
            formatted: DateTime::format(&date),
        }
    }

    fn format(date: &chrono::DateTime<chrono::Local>) -> String {
        date.format("%FT%T%:z").to_string()
    }
}

#[cfg(test)]
mod tests {
    use chrono::Datelike;

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
    fn action_build_works() -> Result<(), Box<dyn Error>> {
        assert_eq!(Action::build("start")?, Action::Start);
        assert_eq!(Action::build("stop")?, Action::Stop);
        assert_eq!(Action::build("mark")?, Action::Mark);

        assert!(Action::build("some string").is_err());

        Ok(())
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
    fn session_file_build_works() {
        let path = PathBuf::new();
        let contents = SessionFile::get_template(&DateTime::now().formatted);
        let file = SessionFile::build(&path, &contents).unwrap();
        assert_eq!(&file.contents, &contents);
    }

    #[test]
    fn session_file_get_heading_with_contents_works() {
        let dt = &DateTime::now();
        let contents = SessionFile::get_template(&dt.formatted);
        let heading_contents = SessionFile::get_heading_with_contents(MARKS_HEADING, &contents);
        assert_eq!(
            heading_contents,
            format!(
                "\
                {MARKS_HEADING}\n\
                \n\
                {MARK_HEADING_PREFIX}{}\
                ",
                dt.formatted
            )
        );
    }

    #[test]
    fn session_file_get_heading_level_works() {
        assert_eq!(SessionFile::get_heading_level("# Heading"), 1);
        assert_eq!(SessionFile::get_heading_level("## Heading"), 2);
        assert_eq!(SessionFile::get_heading_level("##### Heading"), 5);
    }

    #[test]
    fn session_from_file_works() {
        let DateTime { date, formatted } = DateTime::now();
        let mark_first_dt = DateTime {
            date: date.with_hour(5).unwrap(),
            formatted: DateTime::format(&date.with_hour(5).unwrap()),
        };
        let mark_first = Mark {
            date: mark_first_dt.date,
            contents: String::new(),
        };

        let contents = format!(
            "\
                {SESSION_HEADING_PREFIX}{formatted}\n\
                \n\
                {MARKS_HEADING}\n\
                \n\
                {MARK_HEADING_PREFIX}{}\n\
                ",
            mark_first_dt.formatted
        );
        let file = SessionFile::build(&PathBuf::new(), &contents).unwrap();
        let session = Session {
            path: file.path.clone(),
            is_active: true,
            start: date,
            marks: vec![mark_first],
        };

        assert_eq!(Session::from_file(&file).unwrap(), session);
    }

    #[test]
    fn session_to_file_works() -> Result<(), Box<dyn Error>> {
        let dt = DateTime::now();
        let mark_first_dt = DateTime {
            date: dt.date.with_hour(5).unwrap(),
            formatted: DateTime::format(&dt.date.with_hour(5).unwrap()),
        };
        let mark_first = Mark {
            date: mark_first_dt.date,
            contents: String::new(),
        };
        let mark_second_dt = DateTime {
            date: mark_first_dt.date.with_minute(44).unwrap(),
            formatted: DateTime::format(&mark_first_dt.date.with_minute(44).unwrap()),
        };
        let mark_second = Mark {
            date: mark_second_dt.date,
            contents: String::from("I am the second mark!\nHi!\n"),
        };
        let config = Config {
            action: Action::Mark,
            sessions_path: PathBuf::from("sessions"),
        };
        let session = Session {
            path: config.sessions_path.join(format!("{}.md", dt.formatted)),
            is_active: true,
            start: dt.date,
            marks: vec![mark_first, mark_second],
        };
        let file = SessionFile::build(
            &session.path,
            &format!(
                "\
                    {SESSION_HEADING_PREFIX}{}\n\
                    \n\
                    {MARKS_HEADING}\n\
                    \n\
                    {MARK_HEADING_PREFIX}{}\n\
                    \n\
                    {MARK_HEADING_PREFIX}{}\n\
                    \n\
                    I am the second mark!\n\
                    Hi!\n\
                    ",
                dt.formatted, mark_first_dt.formatted, mark_second_dt.formatted
            ),
        )?;

        assert_eq!(session.to_file(&config)?, file);
        Ok(())
    }

    #[test]
    fn session_to_file_and_from_file() {
        let dt = DateTime::now();
        let mark_first = Mark {
            date: dt.date.with_hour(5).unwrap().with_minute(54).unwrap(),
            contents: String::from("feat/some-branch\n\nDid a few things"),
        };
        let mark_second = Mark {
            date: dt.date.with_hour(6).unwrap().with_minute(13).unwrap(),
            contents: String::from("feat/new-feature"),
        };
        let config = Config {
            action: Action::Mark,
            sessions_path: PathBuf::from("sessions"),
        };
        let session = Session {
            path: config.sessions_path.join(&format!("{}.md", dt.formatted)),
            is_active: true,
            start: dt.date,
            marks: vec![mark_first, mark_second],
        };
        let file = session.to_file(&config).unwrap();
        assert_eq!(Session::from_file(&file).unwrap(), session);
    }

    #[test]
    fn session_mark_works() {
        let dt = DateTime::now();
        let mut session = Session {
            path: PathBuf::new(),
            is_active: true,
            start: dt.date,
            marks: vec![],
        };
        let mark = Mark {
            date: dt.date,
            contents: format!("{MARK_HEADING_PREFIX}{}", dt.formatted),
        };
        session.mark();
        assert_eq!(session.marks.len(), 1);
        assert_eq!(session.marks[0], mark);
    }

    #[test]
    fn mark_build_works() {
        let dt = DateTime::now();
        let contents = format!(
            "\
                {MARK_HEADING_PREFIX}{}\n\
                \n\
                This is some content.\n\
                ",
            dt.formatted
        );
        let mark = Mark {
            date: dt.date,
            contents: String::from("This is some content."),
        };
        assert_eq!(Mark::build(&contents).unwrap(), mark);
    }

    #[test]
    fn mark_to_string_works() {
        let dt = DateTime::now();
        let mark = Mark {
            date: dt.date,
            contents: String::from("This is a content of a mark.\nHow are you?\n"),
        };
        let output = format!(
            "\
                {MARK_HEADING_PREFIX}{}\n\
                \n\
                This is a content of a mark.\n\
                How are you?\
                ",
            dt.formatted
        );
        assert_eq!(mark.to_string(), output);
    }

    #[test]
    fn date_time_now_works() {
        let DateTime { date, formatted } = DateTime::now();
        let now = chrono::Local::now();

        assert_eq!(date.year(), now.year());
        assert_eq!(date.month(), now.month());
        assert_eq!(date.day(), now.day());
        assert_eq!(date.hour(), now.hour());
        assert_eq!(date.minute(), now.minute());
        assert_eq!(date.second(), now.second());
        assert_eq!(date.nanosecond(), 0);
        assert_eq!(date.offset(), now.offset());

        assert_eq!(formatted, DateTime::format(&date));
    }
}
