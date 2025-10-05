use crate::{date_time::DateTime, read_sessions_dir, Config};
use std::{
    collections::HashSet,
    error::Error,
    fs,
    path::{Path, PathBuf},
    str::FromStr,
};

const SESSION_HEADING_PREFIX: &str = "# ";
const SESSION_TITLE: &str = "Session";
const MARKS_HEADING: &str = "## Marks";
const MARK_HEADING_PREFIX: &str = "### ";
const LABEL_PREFIX: &str = "- ";
// TODO: Rename.
const LABEL_END: &str = "- end";
const LABEL_SKIP: &str = "- skip";
const LABEL_TAG: &str = "- tag";
const LABEL_TAG_SURROUND: &str = "`";

const COMMAND_VIEW_MARK_CONTENTS_SEPARATOR: &str = "---------------------------------";

pub struct Aggregator {
    sessions: Vec<Session>,
}

impl Aggregator {
    pub fn build(config: &Config) -> Result<Aggregator, Box<dyn Error>> {
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
    pub fn view(&self) -> String {
        let session = self
            .sessions
            .last()
            .expect("must always have at least one session");
        assert!(session.marks.len() > 0);
        assert!(
            !(session.marks.len() == 1
                && session.marks.last().unwrap().attribute == Attribute::Stop)
        );

        // TODO: Write as many of these as functions in their relevant structs.
        let start = DateTime::new(&session.start()).to_formatted_pretty_short();
        let start_of_week = DateTime::get_start_of_week(&session.start());
        let week_time = self
            .sessions
            .iter()
            .filter(|v| v.start().timestamp_millis() - start_of_week.timestamp_millis() >= 0)
            .fold(0, |acc, val| acc + val.get_time());
        let week_time = DateTime::get_time_hr_from_milli(week_time);
        let session_time = DateTime::get_time_hr_from_milli(session.get_time());
        let mark_last = session
            .marks
            .last()
            .expect("session must have at least one mark");
        let mark_last_time = if session.is_active() {
            let timestamp_now = DateTime::now().date.timestamp_millis();
            let timestamp_mark = mark_last.date.timestamp_millis();
            let timestamp = timestamp_now - timestamp_mark;
            assert!(timestamp >= 0);
            &DateTime::get_time_hr_from_milli(timestamp.try_into().unwrap())
        } else {
            "0"
        };
        let mark_last_contents = mark_last.to_string();

        let mut str = String::new();
        if !session.is_active() {
            str += "No active session, last session:\n";
        }
        str += &format!(
            "\
            Start: {start}\n\
            Week: {week_time}\n\
            Time: {session_time}\n\
            Mark: {mark_last_time}\n\
            {COMMAND_VIEW_MARK_CONTENTS_SEPARATOR}\n\
            {mark_last_contents}\
            "
        );
        str
    }
}

#[derive(PartialEq, Debug)]
pub struct SessionFile {
    pub path: PathBuf,
    pub contents: String,
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
pub struct Session {
    pub path: PathBuf,
    pub marks: Vec<Mark>,
}

impl Session {
    pub fn new(config: &Config, dt: &DateTime) -> Session {
        let mark = Mark::new(&dt.date);
        Session {
            path: Path::join(&config.sessions_path, format!("{}.md", dt.to_formatted())),
            marks: vec![mark],
        }
    }

    pub fn get_last(config: &Config) -> Result<Option<Session>, Box<dyn Error>> {
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

    pub fn is_active(&self) -> bool {
        self.marks
            .last()
            .expect("must always have at least one mark")
            .attribute
            != Attribute::Stop
    }

    fn get_time(&self) -> u64 {
        let mut acc = 0;
        let mut mark_preceding = &self.marks[0];
        for mark in self.marks.iter().skip(1) {
            if mark_preceding.attribute != Attribute::Skip {
                acc += DateTime::get_time(&mark_preceding.date, &mark.date);
            }
            mark_preceding = &mark;
        }
        if self.is_active() && mark_preceding.attribute != Attribute::Skip {
            acc += DateTime::get_time(&mark_preceding.date, &DateTime::now().date);
        }
        acc
    }

    pub fn stop(&mut self, dt: &DateTime) -> Result<(), &'static str> {
        if !self.is_active() {
            return Err("session already ended");
        }
        let mut mark = Mark::new(&dt.date);
        mark.attribute = Attribute::Stop;
        self.marks.push(mark);
        Ok(())
    }

    pub fn skip(&mut self) {
        let mark = self
            .marks
            .last_mut()
            .expect("session must always have at least one mark");
        mark.attribute = Attribute::Skip;
    }

    pub fn mark(&mut self, dt: &DateTime) -> Result<(), &'static str> {
        if !self.is_active() {
            return Err("can't mark, session has already ended");
        }
        let mark = Mark::new(&dt.date);
        self.marks.push(mark);
        Ok(())
    }

    pub fn remark(&mut self, dt: &DateTime) {
        let mark = self
            .marks
            .last_mut()
            .expect("session must always have at least one mark");
        mark.date = dt.date;
    }

    pub fn unmark(&mut self) -> Option<Mark> {
        if self.marks.len() > 1 {
            let mark = self.marks.pop();
            assert!(mark.is_some());
            return mark;
        } else {
            return None;
        }
    }

    pub fn tag(&mut self, tag: &Tag) -> bool {
        self.marks
            .last_mut()
            .expect("session must always have at least one mark")
            .tags
            .insert(tag.to_owned())
    }

    pub fn untag(&mut self, tag: &Tag) -> bool {
        self.marks
            .last_mut()
            .expect("session must always have at least one mark")
            .tags
            .remove(&tag)
    }

    /// Returns error if the content of the current mark is not empty.
    pub fn write(&mut self, text: &str) -> Result<(), ()> {
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

    pub fn save(&self) -> Result<(), Box<dyn Error>> {
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

    pub fn to_file(&self) -> Result<SessionFile, &'static str> {
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
pub struct Mark {
    date: chrono::DateTime<chrono::Local>,
    attribute: Attribute,
    tags: HashSet<Tag>,
    // TODO: Rename to text.
    contents: String,
}

impl Mark {
    fn new(date: &chrono::DateTime<chrono::Local>) -> Mark {
        Mark {
            date: date.clone(),
            attribute: Attribute::None,
            tags: HashSet::new(),
            contents: String::new(),
        }
    }

    /// Overwrites the content of this mark.
    fn write(&mut self, text: &str) {
        self.contents = text.to_owned();
    }

    pub fn erase(&mut self) {
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
        let mut attribute = Attribute::None;
        let mut tags: HashSet<Tag> = HashSet::new();
        if contents_without_heading.starts_with(LABEL_PREFIX) {
            for line in contents_without_heading.lines() {
                if !line.starts_with(LABEL_PREFIX) {
                    break;
                }
                let attr = Attribute::from_line(&line);
                if attribute == Attribute::None {
                    attribute = attr;
                } else if attr != Attribute::None {
                    return Err("multiple attributes per mark are not allowed")?;
                } else {
                    tags.insert(Tag::from_line(&line)?);
                }
            }
            let labels_len = tags.len() + if attribute != Attribute::None { 1 } else { 0 };
            contents_without_heading = contents_without_heading
                .lines()
                .skip(labels_len)
                .collect::<Vec<&str>>()
                .join("\n")
                .trim()
                .to_owned();
        }
        Ok(Mark {
            date,
            attribute,
            tags,
            contents: contents_without_heading,
        })
    }

    pub fn to_string(&self) -> String {
        let mut contents = format!(
            "{MARK_HEADING_PREFIX}{}",
            DateTime::new(&self.date).to_formatted_pretty()
        );
        if self.attribute != Attribute::None || !self.tags.is_empty() {
            contents += "\n";
            if self.attribute != Attribute::None {
                contents += "\n";
                contents += &self.attribute.to_line();
            }
            if !self.tags.is_empty() {
                // TODO: Put all tags on the same line?
                let mut tags: Vec<&Tag> = self.tags.iter().collect();
                // Sorts alphabetically.
                tags.sort_by(|a, b| a.text.cmp(&b.text));
                contents += &tags
                    .iter()
                    .fold(String::new(), |acc, val| acc + "\n" + &val.to_line());
            }
        }
        let trimmed = self.contents.trim();
        if !trimmed.is_empty() {
            contents += "\n\n";
            contents += &trimmed;
        }
        contents
    }
}

#[derive(PartialEq, Debug, Clone)]
pub enum Attribute {
    Stop,
    Skip,
    // TODO: Remove?
    None,
}

impl Attribute {
    fn from_line(text: &str) -> Attribute {
        match text.trim() {
            LABEL_END => Attribute::Stop,
            LABEL_SKIP => Attribute::Skip,
            _ => Attribute::None,
        }
    }

    fn to_line(&self) -> String {
        match self {
            Attribute::Stop => LABEL_END.to_owned(),
            Attribute::Skip => LABEL_SKIP.to_owned(),
            Attribute::None => String::new(),
        }
    }
}

#[derive(Eq, Hash, PartialEq, Debug, Clone)]
pub struct Tag {
    text: String,
}

// TODO: Make from_line use from_text.
impl Tag {
    pub fn from_text(text: &str) -> Result<Tag, String> {
        let text = text.trim();
        if text.is_empty() {
            Err("tag cannot be empty")?
        } else if text.contains(LABEL_TAG_SURROUND) {
            Err(format!("invalid character \"{LABEL_TAG_SURROUND}\""))
        } else {
            Ok(Tag {
                text: text.to_owned(),
            })
        }
    }

    fn from_line(text: &str) -> Result<Tag, String> {
        if text.starts_with(&format!("{LABEL_TAG} {LABEL_TAG_SURROUND}"))
            && text.ends_with(LABEL_TAG_SURROUND)
        {
            let start = format!("{LABEL_TAG} {LABEL_TAG_SURROUND}").len();
            let end = text.len() - LABEL_TAG_SURROUND.len();
            let tag_text = text[start..end].trim().to_owned();
            if tag_text.is_empty() {
                return Err("tag cannot be empty")?;
            }
            let tag = Tag { text: tag_text };
            return Ok(tag);
        } else {
            return Err(format!("couldn't parse tag from string '{}'", text));
        }
    }

    fn to_line(&self) -> String {
        let text = &self.text;
        // There shouldn't be a way to store an empty string in here.
        assert!(!text.is_empty());
        format!("{LABEL_TAG} {LABEL_TAG_SURROUND}{text}{LABEL_TAG_SURROUND}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::testing;
    use chrono::Timelike;

    fn get_template(date: &str) -> String {
        format!(
            "\
            {SESSION_HEADING_PREFIX}{SESSION_TITLE}\n\
            \n\
            {MARKS_HEADING}\n\
            \n\
            {MARK_HEADING_PREFIX}{date}\
            "
        )
    }

    #[test]
    fn aggregator_view_works() {
        let date_default = testing::date_default();
        let mark_start = Mark::new(&DateTime::new(&date_default).plus_hours(-2).date);
        let mark_end = Mark::new(&DateTime::new(&date_default).plus_minutes(-30).date);
        let mut session_third = Session {
            path: PathBuf::from("sessions"),
            marks: vec![mark_start, mark_end.clone()],
        };
        let start = DateTime::new(&session_third.start()).to_formatted_pretty();

        // Gets ignored because it's not in the current week.
        let mut session_first = Session {
            path: session_third.path.clone(),
            marks: vec![
                Mark::new(&DateTime::new(&testing::date_default()).plus_days(-12).date),
                Mark::new(&DateTime::new(&testing::date_default()).plus_days(-9).date),
            ],
        };
        session_first.marks.last_mut().unwrap().attribute = Attribute::Stop;

        let mut session_second = Session {
            path: session_third.path.clone(),
            marks: vec![
                Mark::new(&DateTime::new(&testing::date_default()).plus_days(-2).date),
                Mark::new(&DateTime::new(&testing::date_default()).plus_days(-1).date),
            ],
        };
        session_second.marks.last_mut().unwrap().attribute = Attribute::Stop;

        let aggregator = Aggregator {
            sessions: vec![
                session_first.clone(),
                session_second.clone(),
                session_third.clone(),
            ],
        };

        // Goes up to current time.
        let output = aggregator.view();
        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines.len(), 6);
        assert_eq!(lines[0], format!("Start: {start}"));
        // Not explicitly checking if it included the current time in the calculation, just
        // excluding the possibility that it calculated only up to the last mark.
        assert_ne!(lines[1], "Week: 25h 30m 0s");
        assert_ne!(lines[2], "Time: 1h 30m 0s");
        assert_ne!(lines[3], "Mark: 0");
        assert_eq!(lines[4], COMMAND_VIEW_MARK_CONTENTS_SEPARATOR);
        assert_eq!(lines[5], mark_end.to_string());

        session_third.marks.pop();
        session_third.stop(&DateTime::now()).unwrap();
        let mark = &mut session_third.marks[1];
        mark.date = mark_end.date;
        let mark_end = session_third.marks.last().unwrap();
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
                Start: {start}\n\
                Week: 25h 30m 0s\n\
                Time: 1h 30m 0s\n\
                Mark: 0\n\
                {COMMAND_VIEW_MARK_CONTENTS_SEPARATOR}\n\
                {}\
                ",
                mark_end.to_string()
            )
        );
    }

    #[test]
    fn session_file_build_works() {
        let path = PathBuf::new();
        let contents = get_template(&DateTime::now().to_formatted_pretty());
        let file = SessionFile::build(&path, &contents).unwrap();
        assert_eq!(&file.contents, &contents);
    }

    #[test]
    fn session_file_get_heading_with_contents_works() {
        let dt = &DateTime::now();
        let contents = get_template(&dt.to_formatted_pretty());
        let heading_contents = SessionFile::get_heading_with_contents(MARKS_HEADING, &contents);
        assert_eq!(
            heading_contents,
            format!(
                "\
                {MARKS_HEADING}\n\
                \n\
                {MARK_HEADING_PREFIX}{}\
                ",
                dt.to_formatted_pretty()
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
            sessions_path: PathBuf::from("."),
        };
        let dt = DateTime::now();
        let mark = Mark::new(&dt.date);
        let session = Session {
            path: config
                .sessions_path
                .join(format!("{}.md", dt.to_formatted())),
            marks: vec![mark],
        };
        assert_eq!(Session::new(&config, &DateTime::now()), session);
    }

    #[test]
    fn session_start_works() {
        let config = Config {
            sessions_path: PathBuf::from("sessions"),
        };
        let mut session = Session::new(&config, &DateTime::now());
        let date = testing::now_plus_secs(30);
        session.marks.push(Mark::new(&date));
        assert_eq!(session.start(), session.marks.first().unwrap().date);
    }

    #[test]
    fn session_end_works() {
        let config = Config {
            sessions_path: PathBuf::from("sessions"),
        };
        let mut session = Session::new(&config, &DateTime::now());
        assert_eq!(session.end(), session.marks.last().unwrap().date);
        session.stop(&DateTime::now()).unwrap();
        session.marks.last_mut().unwrap().date = testing::now_plus_secs(30);
        assert_eq!(session.end(), session.marks.last().unwrap().date);
    }

    #[test]
    fn session_is_active_works() {
        let config = Config {
            sessions_path: PathBuf::from("sessions"),
        };
        let mut session = Session::new(&config, &DateTime::now());
        assert!(session.is_active());
        assert!(session.marks.last().unwrap().attribute != Attribute::Stop);
        session.stop(&DateTime::now()).unwrap();
        assert!(!session.is_active());
        assert!(session.marks.last().unwrap().attribute == Attribute::Stop);
    }

    // It ignores `mark_first` and counts to current time, so `mark_second` is the final time.
    #[test]
    fn session_get_time_ignores_marks_if_they_have_label_skip() {
        let mut mark_first = Mark::new(&testing::now_plus_secs(-3 * 60 * 60));
        mark_first.attribute = Attribute::Skip;
        let mark_second = Mark::new(&testing::now_plus_secs(-54 * 60 - 10)); // 54m 10s
        let mark_third = Mark::new(&testing::now_plus_secs(-10 * 60));
        let session = Session {
            path: PathBuf::from("sessions"),
            marks: vec![mark_first, mark_second, mark_third],
        };
        assert_eq!(session.get_time(), (54 * 60 + 10) * 1000);
    }

    #[test]
    fn session_get_time_ignores_current_time_if_last_mark_has_label_skip() {
        let mark_first = Mark::new(&testing::now_plus_secs(-3 * 60 * 60));
        let mut mark_second = Mark::new(&testing::now_plus_secs(-1 * 60 * 60 - 33 * 60 - 20)); // 1h 33m 20s
        mark_second.attribute = Attribute::Skip;
        let session = Session {
            path: PathBuf::from("sessions"),
            marks: vec![mark_first, mark_second],
        };
        assert_eq!(session.get_time(), (1 * 60 * 60 + 26 * 60 + 40) * 1000);
    }

    #[test]
    fn session_stop_works() {
        let config = Config {
            sessions_path: PathBuf::from("sessions"),
        };
        let mut session = Session::new(&config, &DateTime::now());
        let mut clone = session.clone();
        session.stop(&DateTime::now()).unwrap();
        let dt = DateTime::now();
        let mut mark = Mark::new(&dt.date);
        mark.attribute = Attribute::Stop;
        clone.marks.push(mark);
        assert_eq!(session, clone);
    }

    #[test]
    fn cannot_stop_when_session_ended() {
        let config = Config {
            sessions_path: PathBuf::from("sessions"),
        };
        let mut session = Session::new(&config, &DateTime::now());
        session.stop(&DateTime::now()).unwrap();
        let clone = session.clone();
        assert!(session.stop(&DateTime::now()).is_err());
        assert_eq!(session, clone);
    }

    #[test]
    fn session_skip_works() {
        let config = Config {
            sessions_path: PathBuf::from("sessions"),
        };
        let mut session = Session::new(&config, &DateTime::now());
        assert_eq!(session.marks.last_mut().unwrap().attribute, Attribute::None);
        session.skip();
        assert_eq!(session.marks.last_mut().unwrap().attribute, Attribute::Skip);
    }

    #[test]
    fn session_skip_overwrites_stop() {
        let config = Config {
            sessions_path: PathBuf::from("sessions"),
        };
        let mut session = Session::new(&config, &DateTime::now());
        session.stop(&DateTime::now()).unwrap();
        assert_eq!(session.marks.last_mut().unwrap().attribute, Attribute::Stop);
        session.skip();
        assert_eq!(session.marks.last_mut().unwrap().attribute, Attribute::Skip);
    }

    #[test]
    fn session_mark_works() {
        let dt = DateTime::now();
        let config = Config {
            sessions_path: PathBuf::from("sessions"),
        };
        let mut session = Session::new(&config, &DateTime::now());
        let mark = Mark::new(&dt.date);
        session.mark(&DateTime::now()).unwrap();
        assert_eq!(session.marks.len(), 2);
        assert_eq!(session.marks[1], mark);
    }

    #[test]
    fn session_mark_preserves_integrity_of_previous_content() {
        let dt = DateTime::now();
        let mark_first = Mark::new(&dt.date.with_hour(14).unwrap());
        let mark_second = Mark::new(&mark_first.date.with_minute(47).unwrap());
        let mut session = Session {
            path: PathBuf::from(format!("./sessions/{}.md", dt.to_formatted())),
            marks: vec![mark_first, mark_second],
        };
        let mut clone = session.clone();
        session.mark(&DateTime::now()).unwrap();
        assert_eq!(session.marks.len(), 3);
        clone.marks.push(session.marks[2].clone());
        assert_eq!(session, clone);
    }

    #[test]
    fn cannot_mark_when_session_ended() {
        let config = Config {
            sessions_path: PathBuf::from("sessions"),
        };
        let mut session = Session::new(&config, &DateTime::now());
        session.stop(&DateTime::now()).unwrap();
        let clone = session.clone();
        assert!(session.mark(&DateTime::now()).is_err());
        assert_eq!(session, clone);
    }

    #[test]
    fn session_remark_works() {
        let config = Config {
            sessions_path: PathBuf::from("sessions"),
        };
        let mut session = Session::new(&config, &DateTime::now());
        session.mark(&DateTime::now()).unwrap();
        let clone = session.clone();
        session.marks.last_mut().unwrap().date = testing::now_plus_secs(30);
        assert_ne!(session, clone);
        session.remark(&DateTime::now());
        assert_eq!(session, clone);
    }

    #[test]
    fn session_unmark_works() {
        let config = Config {
            sessions_path: PathBuf::from("sessions"),
        };
        let mut session = Session::new(&config, &DateTime::now());

        assert_eq!(session.marks.len(), 1);
        assert_eq!(session.unmark(), None);
        assert_eq!(session.marks.len(), 1);

        session.mark(&DateTime::now().plus_hours(1)).unwrap();
        assert_eq!(session.marks.len(), 2);

        let mark_first = session.marks.first().unwrap().clone();
        let mark_last = session.marks.last().unwrap().clone();
        assert_ne!(mark_first, mark_last);
        assert_eq!(session.unmark(), Some(mark_last));
        assert_eq!(session.marks.len(), 1);
        assert_eq!(session.marks.last().unwrap(), &mark_first);
    }

    #[test]
    fn session_tag_works() {
        let config = Config {
            sessions_path: PathBuf::from("sessions"),
        };
        let mut session = Session::new(&config, &DateTime::now());
        // To have at least 2 marks.
        session.mark(&DateTime::now()).unwrap();
        let mut clone = session.clone();
        session.tag(&Tag::from_text("rust").unwrap());
        clone
            .marks
            .last_mut()
            .unwrap()
            .tags
            .insert(Tag::from_text("rust").unwrap().clone());
        assert_eq!(session, clone);
    }

    #[test]
    fn session_untag_works() {
        let config = Config {
            sessions_path: PathBuf::from("sessions"),
        };
        let mut session = Session::new(&config, &DateTime::now());
        // To have at least 2 marks.
        session.mark(&DateTime::now()).unwrap();
        let clone = session.clone();
        session.tag(&Tag::from_text("rust").unwrap());
        session.untag(&Tag::from_text("rust").unwrap());
        assert_eq!(session, clone);
    }

    #[test]
    fn session_write_works() {
        let config = Config {
            sessions_path: PathBuf::from("sessions"),
        };
        let mut session = Session::new(&config, &DateTime::now());
        // To have at least 2 marks.
        session.mark(&DateTime::now()).unwrap();
        let mut clone = session.clone();
        session.write("hello").unwrap();
        clone.marks.iter_mut().last().unwrap().write("hello");
        assert_eq!(session, clone);
    }

    #[test]
    fn session_write_errors_when_there_is_content() {
        let config = Config {
            sessions_path: PathBuf::from("sessions"),
        };
        let mut session = Session::new(&config, &DateTime::now());
        session.write("Some content.").unwrap();
        assert!(session.write("Some other content.").is_err());
    }

    #[test]
    fn session_from_file_works() {
        let DateTime { date, .. } = DateTime::now();
        let mark_first_dt = DateTime {
            date: date.with_hour(5).unwrap(),
            // FIX: same time for both marks breaks reading of them - doesn't read labels and keeps
            // the whole content as is in the file
            // date: date.with_hour(5).unwrap().with_minute(23).unwrap(),
            // formatted: DateTime::format(&date.with_hour(5).unwrap().with_minute(23).unwrap()),
        };
        let mark_first = Mark::new(&mark_first_dt.date);
        let mark_second_dt = DateTime {
            date: mark_first.date.with_minute(23).unwrap(),
        };
        let mut mark_second = Mark::new(&mark_second_dt.date);
        mark_second.attribute = Attribute::Stop;

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
            mark_first_dt.to_formatted_pretty(),
            mark_second_dt.to_formatted_pretty(),
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
        };
        let mark_first = Mark::new(&mark_first_dt.date);
        let mark_second_dt = DateTime {
            date: mark_first_dt.date.with_minute(44).unwrap(),
        };
        let mark_second = Mark {
            date: mark_second_dt.date,
            attribute: Attribute::None,
            tags: HashSet::new(),
            contents: String::from("I am the second mark!\nHi!\n"),
        };
        let config = Config {
            sessions_path: PathBuf::from("sessions"),
        };
        let session = Session {
            path: config
                .sessions_path
                .join(format!("{}.md", mark_first_dt.to_formatted())),
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
                mark_first_dt.to_formatted_pretty(),
                mark_second_dt.to_formatted_pretty()
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
            attribute: Attribute::None,
            tags: HashSet::new(),
            contents: String::from("feat/some-branch\n\nDid a few things"),
        };
        let mark_second = Mark {
            date: dt.date.with_hour(6).unwrap().with_minute(13).unwrap(),
            attribute: Attribute::None,
            tags: HashSet::new(),
            contents: String::from("feat/new-feature"),
        };
        let config = Config {
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
    fn mark_from_string_works() -> Result<(), Box<dyn Error>> {
        let dt = DateTime::now();
        let contents = format!(
            "\
                {MARK_HEADING_PREFIX}{}\n\
                \n\
                {LABEL_END}\n\
                {LABEL_TAG} {LABEL_TAG_SURROUND}rust{LABEL_TAG_SURROUND}\n\
                {LABEL_TAG} {LABEL_TAG_SURROUND}time tracker{LABEL_TAG_SURROUND}\n\
                \n\
                This is some content.\n\
                ",
            dt.to_formatted_pretty()
        );
        let mark = Mark {
            date: dt.date,
            attribute: Attribute::Stop,
            tags: HashSet::from_iter([Tag::from_text("rust")?, Tag::from_text("time tracker")?]),
            contents: String::from("This is some content."),
        };
        assert_eq!(Mark::from_string(&contents).unwrap(), mark);
        Ok(())
    }

    #[test]
    fn mark_from_string_fails_if_multiple_attributes_are_specified() -> Result<(), Box<dyn Error>> {
        let dt = DateTime::now();
        let contents = format!(
            "\
                {MARK_HEADING_PREFIX}{}\n\
                \n\
                {LABEL_END}\n\
                {LABEL_SKIP}\n\
                \n\
                This is some content.\n\
                ",
            dt.to_formatted_pretty()
        );
        assert!(Mark::from_string(&contents).is_err());
        Ok(())
    }

    #[test]
    fn mark_to_string_works() -> Result<(), Box<dyn Error>> {
        let dt = DateTime::now();
        let mark = Mark {
            date: dt.date,
            attribute: Attribute::Stop,
            tags: HashSet::from_iter([Tag::from_text("time tracker")?, Tag::from_text("rust")?]),
            contents: String::from("This is a content of a mark.\nHow are you?\n"),
        };
        let output = format!(
            "\
                {MARK_HEADING_PREFIX}{}\n\
                \n\
                {LABEL_END}\n\
                {LABEL_TAG} {LABEL_TAG_SURROUND}rust{LABEL_TAG_SURROUND}\n\
                {LABEL_TAG} {LABEL_TAG_SURROUND}time tracker{LABEL_TAG_SURROUND}\n\
                \n\
                This is a content of a mark.\n\
                How are you?\
                ",
            dt.to_formatted_pretty()
        );
        assert_eq!(mark.to_string(), output);
        Ok(())
    }

    #[test]
    fn mark_to_string_from_string_works() -> Result<(), Box<dyn Error>> {
        let dt = DateTime::now();

        let mark = Mark {
            date: dt.date,
            attribute: Attribute::None,
            tags: HashSet::new(),
            contents: String::from("This is a content of a mark.\nHow are you?"),
        };
        assert_eq!(mark, Mark::from_string(&mark.to_string())?);

        let mark = Mark {
            date: dt.date,
            attribute: Attribute::Stop,
            tags: HashSet::new(),
            contents: String::from("This is a content of a mark.\nHow are you?"),
        };
        assert_eq!(mark, Mark::from_string(&mark.to_string())?);

        let mark = Mark {
            date: dt.date,
            attribute: Attribute::None,
            tags: HashSet::from_iter([Tag::from_text("rust")?, Tag::from_text("time tracker")?]),
            contents: String::from("This is a content of a mark.\nHow are you?"),
        };
        assert_eq!(mark, Mark::from_string(&mark.to_string())?);

        let mark = Mark {
            date: dt.date,
            attribute: Attribute::Stop,
            tags: HashSet::from_iter([Tag::from_text("rust")?, Tag::from_text("time tracker")?]),
            contents: String::from("This is a content of a mark.\nHow are you?"),
        };
        assert_eq!(mark, Mark::from_string(&mark.to_string())?);

        Ok(())
    }

    #[test]
    fn attribute_from_line_works() {
        assert_eq!(Attribute::from_line(LABEL_END), Attribute::Stop);
        assert_eq!(Attribute::from_line(LABEL_SKIP), Attribute::Skip);
        assert_eq!(Attribute::from_line(LABEL_TAG), Attribute::None);
        assert_eq!(Attribute::from_line("- something else"), Attribute::None);
        assert_eq!(Attribute::from_line("something else"), Attribute::None);
    }

    #[test]
    fn attribute_to_line_works() {
        assert_eq!(Attribute::Stop.to_line(), LABEL_END);
        assert_eq!(Attribute::Skip.to_line(), LABEL_SKIP);
        assert_eq!(Attribute::None.to_line(), "");
    }

    #[test]
    fn tag_from_text_works() -> Result<(), Box<dyn Error>> {
        assert_eq!(
            Tag::from_text("rust")?,
            Tag {
                text: String::from("rust")
            }
        );
        assert!(Tag::from_text("").is_err());
        assert!(Tag::from_text(LABEL_TAG_SURROUND).is_err());

        Ok(())
    }

    #[test]
    fn tag_from_line_works() -> Result<(), Box<dyn Error>> {
        assert_eq!(
            Tag::from_line(&format!(
                "{LABEL_TAG} {LABEL_TAG_SURROUND}rust{LABEL_TAG_SURROUND}"
            ))?,
            Tag {
                text: String::from("rust")
            }
        );
        assert!(Tag::from_line(&format!("rust")).is_err());
        assert!(Tag::from_line(&format!("{LABEL_TAG} rust")).is_err());
        assert!(Tag::from_line(&format!("{LABEL_TAG} {LABEL_TAG_SURROUND}rust")).is_err());
        assert!(Tag::from_line(&format!("{LABEL_TAG} rust{LABEL_TAG_SURROUND}")).is_err());
        assert!(Tag::from_line(&format!("{LABEL_TAG_SURROUND}rust")).is_err());
        assert!(Tag::from_line(&format!("rust{LABEL_TAG_SURROUND}")).is_err());

        Ok(())
    }

    #[test]
    fn tag_to_line_works() -> Result<(), Box<dyn Error>> {
        assert_eq!(
            Tag::from_text("rust")?.to_line(),
            format!("{LABEL_TAG} {LABEL_TAG_SURROUND}rust{LABEL_TAG_SURROUND}")
        );

        Ok(())
    }
}
