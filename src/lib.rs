use std::{
    collections::HashSet,
    env,
    error::Error,
    fs, io,
    path::{Path, PathBuf},
    str::FromStr,
};

use chrono::{Datelike, Timelike};

const CONFIG_PATH: &str = "~/.timetracker.toml";
const CONFIG_SESSIONS_PATH: &str = "sessions_path";

const SESSION_HEADING_PREFIX: &str = "# ";
const SESSION_TITLE: &str = "Session";
const MARKS_HEADING: &str = "## Marks";
const MARK_HEADING_PREFIX: &str = "### ";
const LABEL_PREFIX: &str = "- ";
const LABEL_END: &str = "- end";
const LABEL_SKIP: &str = "- skip";
const LABEL_TAG: &str = "- tag";
const LABEL_TAG_SURROUND: &str = "`";

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

    fn from_args(args: &[String]) -> Result<Config, String> {
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
            action: Action::Start,
            sessions_path,
        };
        Ok(config)
    }
}

#[derive(PartialEq, Debug)]
enum Action {
    Start,
    Stop,
    Mark,
    Path,
    View,
    Label { label: Label },
    Unlabel { label: Label },
    Write { text: String },
    Remark,
    Version,
    // Set,
}

impl Action {
    fn build(name: &str, args: &[String]) -> Result<Action, String> {
        let out = match name {
            "start" => {
                if args.len() != 0 {
                    return Err("too many arguments")?;
                }
                Action::Start
            }
            "stop" => {
                if args.len() != 0 {
                    return Err("too many arguments")?;
                }
                Action::Stop
            }
            "mark" => {
                if args.len() != 0 {
                    return Err("too many arguments")?;
                }
                Action::Mark
            }
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
                Action::Write {
                    text: args[0].to_owned(),
                }
            }
            "remark" => {
                if args.len() != 0 {
                    return Err("too many arguments")?;
                }
                Action::Remark
            }
            "version" => {
                if args.len() != 0 {
                    return Err("too many arguments")?;
                }
                Action::Version
            }
            name => return Err(format!("unrecognized command `{name}`")),
        };
        Ok(out)
    }
}

struct Aggregator {
    sessions: Vec<Session>,
}

impl Aggregator {
    fn build(config: &Config) -> Result<Aggregator, Box<dyn Error>> {
        let sessions = read_sessions_dir(&config)?
            .iter()
            .map(|v| -> Result<Session, Box<dyn Error>> {
                let contents = fs::read_to_string(&v)?;
                let file = SessionFile::build(&v, &contents)?;
                let session = Session::from_file(&file)?;
                Ok(session)
            })
            .collect::<Result<Vec<_>, _>>()?;
        if sessions.is_empty() {
            return Err("session directory is empty")?;
        }
        let aggregator = Aggregator { sessions };
        Ok(aggregator)
    }

    /// Week - Session started in the previous week that ends in the current week is still counted to the
    /// previous week.
    fn view(&self) -> String {
        let session = self
            .sessions
            .last()
            .expect("must always have at least one session");
        assert!(session.marks.len() > 0);
        assert!(
            !(session.marks.len() == 1 && session.marks.last().unwrap().has_label(&Label::End))
        );
        let time = DateTime::get_time_hr_from_milli(session.get_time());
        let start = DateTime::format(&session.start());
        let start_of_week = DateTime::get_start_of_week();
        let week_time = self
            .sessions
            .iter()
            .filter(|v| v.start().timestamp_millis() - start_of_week.timestamp_millis() >= 0)
            .fold(0, |acc, val| acc + val.get_time());
        let week_time = DateTime::get_time_hr_from_milli(week_time);
        let mut str = String::new();
        if !session.is_active() {
            str += "No active session, last session:\n";
        }
        str += &format!(
            "\
            Time: {time}\n\
            Start: {start}\n\
            Week: {week_time}\
            "
        );
        str
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
}

#[derive(PartialEq, Debug, Clone)]
struct Session {
    path: PathBuf,
    marks: Vec<Mark>,
}

impl Session {
    fn new(config: &Config) -> Session {
        let dt = DateTime::now();
        let mark = Mark::new(&dt.date);
        Session {
            path: Path::join(&config.sessions_path, format!("{}.md", dt.formatted)),
            marks: vec![mark],
        }
    }

    fn get_last(config: &Config) -> Result<Option<Session>, Box<dyn Error>> {
        let dir = read_sessions_dir(&config)?;
        if dir.len() == 0 {
            return Ok(None);
        }
        let path = &dir[dir.len() - 1];
        let contents = fs::read_to_string(&path)?;
        let file = SessionFile::build(&path, &contents)?;
        let session = Session::from_file(&file)?;
        Ok(Some(session))
    }

    fn start(&self) -> chrono::DateTime<chrono::Local> {
        self.marks
            .first()
            .expect("session must have at least one mark")
            .date
    }

    #[allow(dead_code)]
    fn end(&self) -> chrono::DateTime<chrono::Local> {
        self.marks
            .last()
            .expect("session must have at least one mark")
            .date
    }

    fn is_active(&self) -> bool {
        !self
            .marks
            .last()
            .expect("must always have at least one mark")
            .has_label(&Label::End)
    }

    fn get_time(&self) -> u64 {
        let mut acc = 0;
        let mut mark_preceding = &self.marks[0];
        for mark in self.marks.iter().skip(1) {
            if !mark_preceding.has_label(&Label::Skip) {
                acc += DateTime::get_time(&mark_preceding.date, &mark.date);
            }
            mark_preceding = &mark;
        }
        if self.is_active() && !mark_preceding.has_label(&Label::Skip) {
            acc += DateTime::get_time(&mark_preceding.date, &DateTime::now().date);
        }
        acc
    }

    fn stop(&mut self) -> Result<(), &'static str> {
        if !self.is_active() {
            return Err("session already ended");
        }
        let dt = DateTime::now();
        let mut mark = Mark::new(&dt.date);
        mark.add_label(&Label::End);
        self.marks.push(mark);
        Ok(())
    }

    fn mark(&mut self) -> Result<(), &'static str> {
        if !self.is_active() {
            return Err("can't mark, session has already ended");
        }
        let dt = DateTime::now();
        let mark = Mark::new(&dt.date);
        self.marks.push(mark);
        Ok(())
    }

    fn label(&mut self, label: &Label) -> bool {
        self.marks
            .last_mut()
            .expect("session must always have at least one mark")
            .add_label(&label)
    }

    fn unlabel(&mut self, label: &Label) -> bool {
        self.marks
            .last_mut()
            .expect("session must always have at least one mark")
            .remove_label(&label)
    }

    /// Returns error if the content of the current mark is not empty.
    fn write(&mut self, text: &str) -> Result<(), ()> {
        let mark = self
            .marks
            .last_mut()
            .expect("session must always have at least one mark");
        if !mark.contents.is_empty() {
            return Err(());
        }
        mark.write(&text);
        Ok(())
    }

    fn remark(&mut self) {
        let mark = self
            .marks
            .last_mut()
            .expect("session must always have at least one mark");
        mark.date = DateTime::now().date;
    }

    fn save(&self) -> Result<(), Box<dyn Error>> {
        let file = self.to_file()?;
        fs::write(&file.path, &file.contents).map_err(|e| format!("coudln't save session: {e}"))?;
        Ok(())
    }

    // TODO: make this and all other from/to methods idiomatic using traits
    fn from_file(file: &SessionFile) -> Result<Session, Box<dyn Error>> {
        let mut marks: Vec<Mark> = Vec::new();
        let marks_contents = SessionFile::get_heading_with_contents(MARKS_HEADING, &file.contents);
        for line in marks_contents.lines() {
            if line.starts_with(MARK_HEADING_PREFIX) {
                let contents = SessionFile::get_heading_with_contents(&line, &marks_contents);
                let mark = Mark::from_string(&contents)?;
                marks.push(mark);
            }
        }
        if marks.is_empty() {
            return Err("there must be at least one mark for a session to be valid")?;
        }

        Ok(Session {
            path: file.path.clone(),
            marks,
        })
    }

    fn to_file(&self) -> Result<SessionFile, &'static str> {
        let mut contents = format!(
            "\
            {SESSION_HEADING_PREFIX}{SESSION_TITLE}\n\
            \n\
            {MARKS_HEADING}\n\
            \n\
            "
        );
        for mark in &self.marks {
            contents += &mark.to_string();
            contents += "\n\n";
        }

        let file = SessionFile::build(&self.path, &contents)?;
        Ok(file)
    }
}

#[derive(PartialEq, Debug, Clone)]
struct Mark {
    date: chrono::DateTime<chrono::Local>,
    labels: HashSet<Label>,
    contents: String,
}

impl Mark {
    fn new(date: &chrono::DateTime<chrono::Local>) -> Mark {
        Mark {
            date: date.clone(),
            labels: HashSet::new(),
            contents: String::new(),
        }
    }

    // TODO: remove all label manipulation methods and use the labels set directly?
    fn add_label(&mut self, label: &Label) -> bool {
        self.labels.insert(label.clone())
    }

    fn remove_label(&mut self, label: &Label) -> bool {
        self.labels.remove(&label)
    }

    fn has_label(&self, label: &Label) -> bool {
        self.labels.contains(&label)
    }

    /// Overwrites the content of this mark.
    fn write(&mut self, text: &str) {
        self.contents = text.to_owned();
    }

    fn erase(&mut self) {
        self.contents = String::new();
    }

    fn from_string(contents: &str) -> Result<Mark, Box<dyn Error>> {
        let contents = contents.trim();
        let date = contents
            .lines()
            .next()
            .map(|val| chrono::DateTime::from_str(&val[MARK_HEADING_PREFIX.len()..val.len()]))
            .ok_or("couldn't parse mark heading")??;
        let mut contents_without_heading = contents
            .lines()
            .skip(1)
            .fold(String::new(), |acc, val| acc + "\n" + val)
            .trim()
            .to_owned();
        let mut labels: HashSet<Label> = HashSet::new();
        if contents_without_heading.starts_with(LABEL_PREFIX) {
            for line in contents_without_heading.lines() {
                if !line.starts_with(LABEL_PREFIX) {
                    break;
                }
                labels.insert(Label::from_string(&line)?);
            }
            contents_without_heading = contents_without_heading
                .lines()
                .skip(labels.len())
                .collect::<Vec<&str>>()
                .join("\n")
                .trim()
                .to_owned();
        }
        Ok(Mark {
            date,
            labels,
            contents: contents_without_heading,
        })
    }

    fn to_string(&self) -> String {
        let mut contents = format!("{MARK_HEADING_PREFIX}{}", DateTime::format(&self.date));
        if !self.labels.is_empty() {
            contents += "\n";
            contents += &self
                .labels
                .iter()
                .fold(String::new(), |acc, val| acc + "\n" + &val.to_string());
        }
        let trimmed = self.contents.trim();
        if !trimmed.is_empty() {
            contents += "\n\n";
            contents += &trimmed;
        }
        contents
    }
}

#[derive(Eq, Hash, PartialEq, Debug, Clone)]
enum Label {
    End,
    Skip,
    Tag { text: String },
}

impl Label {
    fn from_args(args: &[String]) -> Result<Label, String> {
        if args.is_empty() {
            return Err("not enough arguments")?;
        }

        let name = LABEL_PREFIX.to_owned() + args[0].trim();
        if name == LABEL_END {
            if args.len() > 1 {
                return Err("too many arguments")?;
            }
            return Ok(Label::End);
        } else if name == LABEL_SKIP {
            if args.len() > 1 {
                return Err("too many arguments")?;
            }
            return Ok(Label::Skip);
        } else if name.starts_with(LABEL_TAG) {
            if args.len() < 2 {
                return Err("not enough arguments")?;
            } else if args.len() > 2 {
                return Err("too many arguments")?;
            }
            let tag = Label::Tag {
                text: args[1].clone(),
            };
            return Ok(tag);
        } else {
            return Err(format!("couldn't parse label from args '{:?}'", &args));
        }
    }

    fn from_string(text: &str) -> Result<Label, String> {
        let text = text.trim();
        if text == LABEL_END {
            return Ok(Label::End);
        } else if text == LABEL_SKIP {
            return Ok(Label::Skip);
        } else if text.starts_with(&format!("{LABEL_TAG} {LABEL_TAG_SURROUND}"))
            && text.ends_with(LABEL_TAG_SURROUND)
        {
            let start = format!("{LABEL_TAG} {LABEL_TAG_SURROUND}").len();
            let end = text.len() - LABEL_TAG_SURROUND.len();
            let tag_text = text[start..end].trim().to_owned();
            if tag_text.is_empty() {
                return Err("label tag cannot be empty")?;
            }
            let tag = Label::Tag { text: tag_text };
            return Ok(tag);
        } else {
            return Err(format!("couldn't parse label from string '{}'", text));
        }
    }

    fn to_string(&self) -> String {
        match self {
            Label::End => LABEL_END.to_owned(),
            Label::Skip => LABEL_SKIP.to_owned(),
            Label::Tag { text } => {
                // There shouldn't be a way to store empty string in Label::Tag.
                assert!(!text.is_empty());
                format!("{LABEL_TAG} {LABEL_TAG_SURROUND}{text}{LABEL_TAG_SURROUND}")
            }
        }
    }
}

// TODO: put all these functions inside Action?
pub fn run(config: Config) -> Result<(), Box<dyn Error>> {
    let result = match config.action {
        Action::Start => start(&config),
        Action::Stop => stop(&config),
        Action::Mark => mark(&config),
        Action::Path => path(&config),
        Action::View => view(&config),
        Action::Label { .. } => label(&config),
        Action::Unlabel { .. } => unlabel(&config),
        Action::Write { .. } => write(&config),
        Action::Remark => remark(&config),
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

    let session = Session::new(&config);
    let SessionFile { path, contents } = session.to_file()?;
    if fs::exists(&path)? {
        return Err("this session file is already created")?;
    };
    fs::write(&path, &contents).map_err(|_| "session directory doesn't exist")?;
    println!("Started: {}", DateTime::format(&session.start()));
    Ok(())
}

fn stop(config: &Config) -> Result<(), Box<dyn Error>> {
    let Some(mut session) = Session::get_last(&config)? else {
        return Err("no active session found")?;
    };
    session.stop()?;
    session.save()?;
    let mark = session.marks.last().expect("Last mark was just added");
    println!("Stopped: {}", DateTime::format(&mark.date));
    Ok(())
}

fn mark(config: &Config) -> Result<(), Box<dyn Error>> {
    let Some(mut session) = Session::get_last(&config)? else {
        return Err("no active session found")?;
    };
    session.mark()?;
    session.save()?;
    let mark = session.marks.last().expect("Last mark was just added");
    println!("Marked: {}", DateTime::format(&mark.date));
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
    println!("Text was successfully written to current mark");
    Ok(())
}

fn remark(config: &Config) -> Result<(), Box<dyn Error>> {
    let Some(mut session) = Session::get_last(&config)? else {
        return Err("no active session found")?;
    };
    session.remark();
    session.save()?;
    println!("Updated current mark's date to current date");
    Ok(())
}

fn version() {
    let version = env!("CARGO_PKG_VERSION");
    println!("v{version}");
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

    // TEST: that it works when the months change in the middle of the week.
    #[allow(dead_code)]
    fn get_start_of_week() -> chrono::DateTime<chrono::Local> {
        let date = DateTime::now().date;
        let days_since_monday: i64 = date.weekday().num_days_from_monday().into();
        let date: chrono::DateTime<chrono::Local> = chrono::DateTime::from_timestamp_millis(
            date.timestamp_millis() - days_since_monday * 24 * 60 * 60 * 1000,
        )
        .unwrap()
        .into();
        let date = date
            .with_time(chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap())
            .unwrap();
        date
    }

    // TODO: move to it's own struct or combine with std::time::Duration?
    fn get_time(
        start: &chrono::DateTime<chrono::Local>,
        end: &chrono::DateTime<chrono::Local>,
    ) -> u64 {
        let time = end.timestamp_millis() - start.timestamp_millis();
        time.try_into()
            .expect("start date must always be smaller than end date")
    }

    fn get_time_hr_from_milli(milli: u64) -> String {
        let mut timestamp = milli / 1000;
        const UNIT: u64 = 60;
        let seconds = timestamp % UNIT;
        timestamp /= UNIT;
        let minutes = timestamp % UNIT;
        timestamp /= UNIT;
        let hours = timestamp;
        format!("{hours}h {minutes}m {seconds}s")
    }
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

#[cfg(test)]
mod tests {
    use chrono::Datelike;

    use super::*;

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

    fn now_plus_secs(secs: i64) -> chrono::DateTime<chrono::Local> {
        let date = DateTime::now().date;
        chrono::DateTime::from_timestamp_millis(date.timestamp_millis() + secs * 1000)
            .unwrap()
            .into()
    }

    #[test]
    fn config_from_args_works() {
        let args = &[String::from("time_tracker"), String::from("start")];
        let config = Config {
            action: Action::Start,
            sessions_path: PathBuf::from(""),
        };
        assert_eq!(Config::from_args(args).unwrap(), config);
    }

    #[test]
    fn config_from_file_works() {
        let path = "./notes/sessions";
        let contents = format!("{CONFIG_SESSIONS_PATH}='{path}'");
        let config = Config {
            action: Action::Start,
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
        assert_eq!(Action::build("start", &[])?, Action::Start);
        assert!(Action::build("start", &[String::from("hello")]).is_err());

        assert_eq!(Action::build("stop", &[])?, Action::Stop);
        assert!(Action::build("stop", &[String::from("hello")]).is_err());

        assert_eq!(Action::build("mark", &[])?, Action::Mark);
        assert!(Action::build("mark", &[String::from("hello")]).is_err());

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

        assert_eq!(Action::build("remark", &[])?, Action::Remark);
        assert!(Action::build("remark", &[String::from("hello")]).is_err());

        assert!(Action::build("hello", &[]).is_err());

        Ok(())
    }

    #[test]
    fn aggregator_view_works() {
        let mark_start = Mark::new(&now_plus_secs(-2 * 60 * 60));
        let mark_end = Mark::new(&now_plus_secs(-30 * 60));
        let mut session_third = Session {
            path: PathBuf::from("sessions"),
            marks: vec![mark_start, mark_end.clone()],
        };
        let start = DateTime::format(&session_third.start());

        // Gets ignored because it's not in the current week.
        let mut session_first = Session {
            path: session_third.path.clone(),
            marks: vec![
                Mark::new(&now_plus_secs(-12 * 24 * 60 * 60)),
                Mark::new(&now_plus_secs(-9 * 24 * 60 * 60)),
            ],
        };
        session_first
            .marks
            .last_mut()
            .unwrap()
            .add_label(&Label::End);

        let mut session_second = Session {
            path: session_third.path.clone(),
            marks: vec![
                Mark::new(&now_plus_secs(-4 * 24 * 60 * 60)),
                Mark::new(&now_plus_secs(-3 * 24 * 60 * 60)),
            ],
        };
        session_second
            .marks
            .last_mut()
            .unwrap()
            .add_label(&Label::End);

        let aggregator = Aggregator {
            sessions: vec![
                session_first.clone(),
                session_second.clone(),
                session_third.clone(),
            ],
        };

        // Goes up to current time.
        assert_eq!(
            aggregator.view(),
            format!(
                "\
                Time: 2h 0m 0s\n\
                Start: {start}\n\
                Week: 26h 0m 0s\
                "
            )
        );

        session_third.marks.pop();
        session_third.stop().unwrap();
        let mark = &mut session_third.marks[1];
        mark.date = mark_end.date;
        let aggregator = Aggregator {
            sessions: vec![
                session_first.clone(),
                session_second.clone(),
                session_third.clone(),
            ],
        };
        assert_eq!(
            aggregator.view(),
            format!(
                "\
                No active session, last session:\n\
                Time: 1h 30m 0s\n\
                Start: {start}\n\
                Week: 25h 30m 0s\
                "
            )
        );
    }

    #[test]
    fn session_file_build_works() {
        let path = PathBuf::new();
        let contents = get_template(&DateTime::now().formatted);
        let file = SessionFile::build(&path, &contents).unwrap();
        assert_eq!(&file.contents, &contents);
    }

    #[test]
    fn session_file_get_heading_with_contents_works() {
        let dt = &DateTime::now();
        let contents = get_template(&dt.formatted);
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
    fn session_new_works() {
        let config = Config {
            action: Action::Start,
            sessions_path: PathBuf::from("."),
        };
        let dt = DateTime::now();
        let mark = Mark::new(&dt.date);
        let session = Session {
            path: config.sessions_path.join(format!("{}.md", dt.formatted)),
            marks: vec![mark],
        };
        assert_eq!(Session::new(&config), session);
    }

    #[test]
    fn session_start_works() {
        let config = Config {
            action: Action::Start,
            sessions_path: PathBuf::from("sessions"),
        };
        let mut session = Session::new(&config);
        let date = now_plus_secs(30);
        session.marks.push(Mark::new(&date));
        assert_eq!(session.start(), session.marks.first().unwrap().date);
    }

    #[test]
    fn session_end_works() {
        let config = Config {
            action: Action::Start,
            sessions_path: PathBuf::from("sessions"),
        };
        let mut session = Session::new(&config);
        assert_eq!(session.end(), session.marks.last().unwrap().date);
        session.stop().unwrap();
        session.marks.last_mut().unwrap().date = now_plus_secs(30);
        assert_eq!(session.end(), session.marks.last().unwrap().date);
    }

    #[test]
    fn session_is_active_works() {
        let config = Config {
            action: Action::Start,
            sessions_path: PathBuf::from("sessions"),
        };
        let mut session = Session::new(&config);
        assert!(session.is_active());
        assert!(!session.marks.last().unwrap().has_label(&Label::End));
        session.stop().unwrap();
        assert!(!session.is_active());
        assert!(session.marks.last().unwrap().has_label(&Label::End));
    }

    // It ignores `mark_first` and counts to current time, so `mark_second` is the final time.
    #[test]
    fn session_get_time_ignores_marks_if_they_have_label_skip() {
        let mut mark_first = Mark::new(&now_plus_secs(-3 * 60 * 60));
        mark_first.add_label(&Label::Skip);
        let mark_second = Mark::new(&now_plus_secs(-54 * 60 - 10)); // 54m 10s
        let mark_third = Mark::new(&now_plus_secs(-10 * 60));
        let session = Session {
            path: PathBuf::from("sessions"),
            marks: vec![mark_first, mark_second, mark_third],
        };
        assert_eq!(session.get_time(), (54 * 60 + 10) * 1000);
    }

    #[test]
    fn session_get_time_ignores_current_time_if_last_mark_has_label_skip() {
        let mark_first = Mark::new(&now_plus_secs(-3 * 60 * 60));
        let mut mark_second = Mark::new(&now_plus_secs(-1 * 60 * 60 - 33 * 60 - 20)); // 1h 33m 20s
        mark_second.add_label(&Label::Skip);
        let session = Session {
            path: PathBuf::from("sessions"),
            marks: vec![mark_first, mark_second],
        };
        assert_eq!(session.get_time(), (1 * 60 * 60 + 26 * 60 + 40) * 1000);
    }

    #[test]
    fn session_stop_works() {
        let config = Config {
            action: Action::Stop,
            sessions_path: PathBuf::from("sessions"),
        };
        let mut session = Session::new(&config);
        let mut clone = session.clone();
        session.stop().unwrap();
        let dt = DateTime::now();
        let mut mark = Mark::new(&dt.date);
        mark.add_label(&Label::End);
        clone.marks.push(mark);
        assert_eq!(session, clone);
    }

    #[test]
    fn cannot_stop_when_session_ended() {
        let config = Config {
            action: Action::Stop,
            sessions_path: PathBuf::from("sessions"),
        };
        let mut session = Session::new(&config);
        session.stop().unwrap();
        let clone = session.clone();
        assert!(session.stop().is_err());
        assert_eq!(session, clone);
    }

    #[test]
    fn session_mark_works() {
        let dt = DateTime::now();
        let config = Config {
            action: Action::Start,
            sessions_path: PathBuf::from("sessions"),
        };
        let mut session = Session::new(&config);
        let mark = Mark::new(&dt.date);
        session.mark().unwrap();
        assert_eq!(session.marks.len(), 2);
        assert_eq!(session.marks[1], mark);
    }

    #[test]
    fn session_mark_preserves_integrity_of_previous_content() {
        let dt = DateTime::now();
        let mark_first = Mark::new(&dt.date.with_hour(14).unwrap());
        let mark_second = Mark::new(&mark_first.date.with_minute(47).unwrap());
        let mut session = Session {
            path: PathBuf::from(format!("./sessions/{}.md", dt.formatted)),
            marks: vec![mark_first, mark_second],
        };
        let mut clone = session.clone();
        session.mark().unwrap();
        assert_eq!(session.marks.len(), 3);
        clone.marks.push(session.marks[2].clone());
        assert_eq!(session, clone);
    }

    #[test]
    fn cannot_mark_when_session_ended() {
        let config = Config {
            action: Action::Mark,
            sessions_path: PathBuf::from("sessions"),
        };
        let mut session = Session::new(&config);
        session.stop().unwrap();
        let clone = session.clone();
        assert!(session.mark().is_err());
        assert_eq!(session, clone);
    }

    #[test]
    fn session_label_works() {
        let config = Config {
            action: Action::Label { label: Label::Skip },
            sessions_path: PathBuf::from("sessions"),
        };
        let mut session = Session::new(&config);
        // To have at least 2 marks.
        session.mark().unwrap();
        let mut clone = session.clone();
        session.label(&Label::Skip);
        clone.marks.last_mut().unwrap().add_label(&Label::Skip);
        assert_eq!(session, clone);
    }

    #[test]
    fn session_unlabel_works() {
        let config = Config {
            action: Action::Label { label: Label::Skip },
            sessions_path: PathBuf::from("sessions"),
        };
        let mut session = Session::new(&config);
        // To have at least 2 marks.
        session.mark().unwrap();
        let clone = session.clone();
        session.label(&Label::Skip);
        session.unlabel(&Label::Skip);
        assert_eq!(session, clone);
    }

    #[test]
    fn session_write_works() {
        let config = Config {
            action: Action::Write {
                text: String::new(),
            },
            sessions_path: PathBuf::from("sessions"),
        };
        let mut session = Session::new(&config);
        // To have at least 2 marks.
        session.mark().unwrap();
        let mut clone = session.clone();
        session.write("hello").unwrap();
        clone.marks.iter_mut().last().unwrap().write("hello");
        assert_eq!(session, clone);
    }

    #[test]
    fn session_write_errors_when_there_is_content() {
        let config = Config {
            action: Action::Start,
            sessions_path: PathBuf::from("sessions"),
        };
        let mut session = Session::new(&config);
        session.write("Some content.").unwrap();
        assert!(session.write("Some other content.").is_err());
    }

    #[test]
    fn session_remark_works() {
        let config = Config {
            action: Action::Start,
            sessions_path: PathBuf::from("sessions"),
        };
        let mut session = Session::new(&config);
        session.mark().unwrap();
        let clone = session.clone();
        session.marks.last_mut().unwrap().date = now_plus_secs(30);
        assert_ne!(session, clone);
        session.remark();
        assert_eq!(session, clone);
    }

    #[test]
    fn session_from_file_works() {
        let DateTime { date, .. } = DateTime::now();
        let mark_first_dt = DateTime {
            date: date.with_hour(5).unwrap(),
            formatted: DateTime::format(&date.with_hour(5).unwrap()),
            // FIX: same time for both marks breaks reading of them - doesn't read labels and keeps
            // the whole content as is in the file
            // date: date.with_hour(5).unwrap().with_minute(23).unwrap(),
            // formatted: DateTime::format(&date.with_hour(5).unwrap().with_minute(23).unwrap()),
        };
        let mark_first = Mark::new(&mark_first_dt.date);
        let mark_second_dt = DateTime {
            date: mark_first.date.with_minute(23).unwrap(),
            formatted: DateTime::format(&mark_first.date.with_minute(23).unwrap()),
        };
        let mut mark_second = Mark::new(&mark_second_dt.date);
        mark_second.add_label(&Label::End);

        let contents = format!(
            "\
                {SESSION_HEADING_PREFIX}{SESSION_TITLE}\n\
                \n\
                {MARKS_HEADING}\n\
                \n\
                {MARK_HEADING_PREFIX}{}\n\
                \n\
                {MARK_HEADING_PREFIX}{}\n\
                \n\
                {LABEL_END}\n\
                ",
            mark_first_dt.formatted, mark_second_dt.formatted,
        );
        let file = SessionFile::build(&PathBuf::new(), &contents).unwrap();
        let session = Session {
            path: file.path.clone(),
            marks: vec![mark_first, mark_second],
        };

        assert_eq!(Session::from_file(&file).unwrap(), session);
    }

    #[test]
    fn session_from_file_fails_when_there_is_no_mark() {
        let contents = format!(
            "\
                {SESSION_HEADING_PREFIX}{SESSION_TITLE}\n\
                \n\
                {MARKS_HEADING}\n\
                ",
        );
        let file = SessionFile::build(&PathBuf::from("sessions"), &contents).unwrap();
        assert!(Session::from_file(&file).is_err());
    }

    #[test]
    fn session_to_file_works() -> Result<(), Box<dyn Error>> {
        let dt = DateTime::now();
        let mark_first_dt = DateTime {
            date: dt.date.with_hour(5).unwrap(),
            formatted: DateTime::format(&dt.date.with_hour(5).unwrap()),
        };
        let mark_first = Mark::new(&mark_first_dt.date);
        let mark_second_dt = DateTime {
            date: mark_first_dt.date.with_minute(44).unwrap(),
            formatted: DateTime::format(&mark_first_dt.date.with_minute(44).unwrap()),
        };
        let mark_second = Mark {
            date: mark_second_dt.date,
            labels: HashSet::new(),
            contents: String::from("I am the second mark!\nHi!\n"),
        };
        let config = Config {
            action: Action::Mark,
            sessions_path: PathBuf::from("sessions"),
        };
        let session = Session {
            path: config
                .sessions_path
                .join(format!("{}.md", mark_first_dt.formatted)),
            marks: vec![mark_first, mark_second],
        };
        let file = SessionFile::build(
            &session.path,
            &format!(
                "\
                    {SESSION_HEADING_PREFIX}{SESSION_TITLE}\n\
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
                mark_first_dt.formatted, mark_second_dt.formatted
            ),
        )?;

        assert_eq!(session.to_file()?, file);
        Ok(())
    }

    #[test]
    fn session_to_file_and_from_file() {
        let dt = DateTime::now();
        let mark_first = Mark {
            date: dt.date.with_hour(5).unwrap().with_minute(54).unwrap(),
            labels: HashSet::new(),
            contents: String::from("feat/some-branch\n\nDid a few things"),
        };
        let mark_second = Mark {
            date: dt.date.with_hour(6).unwrap().with_minute(13).unwrap(),
            labels: HashSet::new(),
            contents: String::from("feat/new-feature"),
        };
        let config = Config {
            action: Action::Mark,
            sessions_path: PathBuf::from("sessions"),
        };
        let session = Session {
            path: config
                .sessions_path
                .join(&format!("{}.md", DateTime::format(&mark_first.date))),
            marks: vec![mark_first, mark_second],
        };
        let file = session.to_file().unwrap();
        assert_eq!(Session::from_file(&file).unwrap(), session);
    }

    #[test]
    fn mark_new_works() {
        let dt = DateTime::now();
        let mark = Mark::new(&dt.date);
        assert_eq!(Mark::new(&dt.date), mark);
    }

    #[test]
    fn mark_add_label_works() {
        let dt = DateTime::now();
        let mut mark = Mark::new(&dt.date);
        mark.add_label(&Label::End);
        let expected = Mark {
            date: dt.date,
            labels: HashSet::from_iter([Label::End]),
            contents: String::new(),
        };
        assert_eq!(mark, expected);
    }

    #[test]
    fn mark_add_label_does_not_add_duplicate_label() {
        let dt = DateTime::now();
        let mut mark = Mark::new(&dt.date);
        mark.add_label(&Label::Skip);
        let clone = mark.clone();
        mark.add_label(&Label::Skip);
        assert_eq!(mark, clone);
    }

    #[test]
    fn mark_remove_label_works() {
        let dt = DateTime::now();
        let mut mark = Mark::new(&dt.date);
        let clone = mark.clone();
        mark.add_label(&Label::Skip);
        mark.remove_label(&Label::Skip);
        assert_eq!(mark, clone);
        mark.remove_label(&Label::Skip);
        assert_eq!(mark, clone);
    }

    #[test]
    fn mark_has_label_works() {
        let dt = DateTime::now();
        let mut mark = Mark::new(&dt.date);
        mark.add_label(&Label::End);
        assert!(mark.has_label(&Label::End));
    }

    #[test]
    fn mark_write_works() {
        let dt = DateTime::now();
        let mut mark = Mark::new(&dt.date);
        let mut clone = mark.clone();
        clone.contents = String::from("This is some content.");
        mark.write("This is some content.");
        assert_eq!(mark, clone);
    }

    #[test]
    fn mark_write_overwrites_previous_content() {
        let dt = DateTime::now();
        let mut mark = Mark::new(&dt.date);
        let mut clone = mark.clone();
        clone.contents = String::from("This is some content");
        mark.write("This is some content");
        assert_eq!(mark, clone);
        clone.contents = String::from("This is overwritten content");
        mark.write("This is overwritten content");
        assert_eq!(mark, clone);
    }

    #[test]
    fn mark_erase_works() {
        let dt = DateTime::now();
        let mut mark = Mark::new(&dt.date);
        let clone = mark.clone();
        mark.write("This is content.");
        mark.erase();
        assert_eq!(mark, clone);
    }

    #[test]
    fn mark_from_string_works() {
        let dt = DateTime::now();
        let contents = format!(
            "\
                {MARK_HEADING_PREFIX}{}\n\
                \n\
                {LABEL_END}\n\
                \n\
                This is some content.\n\
                ",
            dt.formatted
        );
        let mark = Mark {
            date: dt.date,
            labels: HashSet::from_iter([Label::End]),
            contents: String::from("This is some content."),
        };
        assert_eq!(Mark::from_string(&contents).unwrap(), mark);
    }

    #[test]
    fn mark_to_string_works() {
        let dt = DateTime::now();
        let mark = Mark {
            date: dt.date,
            labels: HashSet::from_iter([Label::End]),
            contents: String::from("This is a content of a mark.\nHow are you?\n"),
        };
        let output = format!(
            "\
                {MARK_HEADING_PREFIX}{}\n\
                \n\
                {LABEL_END}\n\
                \n\
                This is a content of a mark.\n\
                How are you?\
                ",
            dt.formatted
        );
        assert_eq!(mark.to_string(), output);
    }

    #[test]
    fn label_from_args_works() -> Result<(), Box<dyn Error>> {
        assert!(Label::from_args(&[]).is_err());
        assert!(Label::from_args(&[String::from("hello")]).is_err());

        assert_eq!(Label::from_args(&[String::from("end")])?, Label::End);
        assert!(Label::from_args(&[String::from("end"), String::from("hello")]).is_err());

        assert_eq!(Label::from_args(&[String::from("skip")])?, Label::Skip);
        assert!(Label::from_args(&[String::from("skip"), String::from("hello")]).is_err());

        assert!(Label::from_args(&[String::from("tag")]).is_err());
        assert_eq!(
            Label::from_args(&[String::from("tag"), String::from("rust")])?,
            Label::Tag {
                text: String::from("rust")
            }
        );
        assert!(Label::from_args(&[
            String::from("tag"),
            String::from("rust"),
            String::from("hello")
        ])
        .is_err());

        Ok(())
    }

    #[test]
    fn label_from_string_works() {
        assert_eq!(Label::from_string(LABEL_END).unwrap(), Label::End);

        assert_eq!(Label::from_string(LABEL_SKIP).unwrap(), Label::Skip);

        assert!(Label::from_string(&format!(
            "{LABEL_TAG} {LABEL_TAG_SURROUND}{LABEL_TAG_SURROUND}"
        ))
        .is_err());
        assert!(Label::from_string(&format!("{LABEL_TAG} {LABEL_TAG_SURROUND}rust")).is_err());
        assert!(Label::from_string(&format!("rust{LABEL_TAG_SURROUND}")).is_err());
        assert_eq!(
            Label::from_string(&format!(
                "{LABEL_TAG} {LABEL_TAG_SURROUND}rust{LABEL_TAG_SURROUND}"
            ))
            .unwrap(),
            Label::Tag {
                text: String::from("rust")
            }
        );

        assert!(Label::from_string("some string").is_err())
    }

    #[test]
    fn label_to_string_works() {
        assert_eq!(Label::End.to_string(), LABEL_END);

        assert_eq!(Label::Skip.to_string(), LABEL_SKIP);

        assert_eq!(
            Label::Tag {
                text: String::from("rust"),
            }
            .to_string(),
            format!("{LABEL_TAG} {LABEL_TAG_SURROUND}rust{LABEL_TAG_SURROUND}")
        );
    }

    #[test]
    fn label_to_string_and_from_string() {
        let label = Label::End;
        let as_string = label.to_string();
        assert_eq!(Label::from_string(&as_string).unwrap(), label);
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

    #[test]
    fn date_time_get_start_of_week_works() {
        let date = DateTime::get_start_of_week();
        assert_eq!(date.weekday(), chrono::Weekday::Mon);
        assert!(
            DateTime::now().date.timestamp_millis() - date.timestamp_millis()
                < 7 * 24 * 60 * 60 * 1000
        );
        let time = date.time();
        assert_eq!(time.hour(), 0);
        assert_eq!(time.minute(), 0);
        assert_eq!(time.second(), 0);
    }

    #[test]
    fn date_time_get_time_works() {
        let start = DateTime::now().date.with_minute(2).unwrap();
        let end = start.with_minute(5).unwrap();
        let time = DateTime::get_time(&start, &end);
        assert_eq!(time, 180_000);
    }

    #[test]
    fn date_time_get_time_hr_from_milli_works() {
        let start = DateTime::now()
            .date
            .with_year(2000)
            .unwrap()
            .with_month(1)
            .unwrap()
            .with_day(1)
            .unwrap()
            .with_hour(0)
            .unwrap()
            .with_minute(0)
            .unwrap()
            .with_second(0)
            .unwrap();
        let end = start
            // 7*31*24=5208h
            // 4*30*24=2880h
            // 1*29*24=0696h
            .with_year(2001)
            .unwrap()
            // +744h
            .with_month(2)
            .unwrap()
            // +24h
            .with_day(2)
            .unwrap()
            // +2h
            .with_hour(2)
            .unwrap()
            .with_minute(2)
            .unwrap()
            .with_second(2)
            .unwrap();
        let text = "9554h 2m 2s";
        let time = DateTime::get_time(&start, &end);
        assert_eq!(DateTime::get_time_hr_from_milli(time), text);
    }
}
