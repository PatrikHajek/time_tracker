use config::{Action, Config};
use date_time::DateTime;
use session::{Aggregator, Attribute, Session, SessionFile, Tag};
use std::{env, error::Error, fs, io, path::PathBuf, process::Command};

mod config;
mod date_time;
mod session;
#[cfg(test)]
mod testing;

pub fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    let (action, config) =
        setup(&args).map_err(|err| format!("Problem parsing arguments: {err}"))?;

    let out = match action {
        Action::Start { date } => start(&config, &date),
        Action::Mark { date } => mark(&config, &date),
        Action::Remark { date } => remark(&config, &date),
        Action::Unmark => unmark(&config),
        Action::Path => path(&config),
        Action::View => view(&config),
        Action::Attribute { attribute: attr } => attribute(&config, attr),
        Action::Tag { tag: tag_ } => tag(&config, &tag_),
        Action::Untag { tag } => untag(&config, &tag),
        Action::Write { text } => write(&config, &text),
        Action::Version => Ok(version()),
    }
    .map_err(|err| format!("Application error: {err}"))?;
    Ok(out)
}

fn setup(args: &[String]) -> Result<(Action, Config), Box<dyn Error>> {
    if args.len() < 2 {
        return Err("not enough arguments")?;
    }

    // First arg (args[0]) is the name of the program.
    let action = Action::build(&args[1], &args[2..])?;
    let config = Config::build()?;
    Ok((action, config))
}

fn start(config: &Config, date: &DateTime) -> Result<(), Box<dyn Error>> {
    if let Some(session) = Session::get_last(&config)? {
        if session.is_active() {
            return Err("another session is already active")?;
        }
    }

    let session = Session::new(&config, &date);
    let SessionFile { path, contents } = session.to_file()?;
    if fs::exists(&path)? {
        return Err("this session file is already created")?;
    };
    fs::write(&path, &contents).map_err(|_| "session directory doesn't exist")?;
    println!("Started: {}", &date.to_formatted_time());
    Ok(())
}

fn mark(config: &Config, date: &DateTime) -> Result<(), Box<dyn Error>> {
    let Some(mut session) = Session::get_last(&config)? else {
        return Err("no active session found")?;
    };

    session.mark(&date)?;
    session.save()?;
    println!("Marked: {}", &date.to_formatted_time());
    Ok(())
}

fn remark(config: &Config, date: &DateTime) -> Result<(), Box<dyn Error>> {
    let Some(mut session) = Session::get_last(&config)? else {
        return Err("no active session found")?;
    };

    session.remark(&date);
    session.save()?;
    println!("Remarked to: {}", &date.to_formatted_time());
    Ok(())
}

fn unmark(config: &Config) -> Result<(), Box<dyn Error>> {
    let Some(mut session) = Session::get_last(&config)? else {
        return Err("no active session found")?;
    };

    if let Some(mark) = session.unmark() {
        session.save()?;
        println!("Removed last mark:\n{}", mark.to_line());
    } else {
        println!("Cannot remove the first mark");
    }
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

fn attribute(config: &Config, attribute: Attribute) -> Result<(), Box<dyn Error>> {
    let Some(mut session) = Session::get_last(&config)? else {
        return Err("no active session found")?;
    };

    session.set_attribute(attribute);
    session.save()?;
    // println!("Set attribute: {attribute:?}");
    Ok(())
}

fn tag(config: &Config, tag: &Tag) -> Result<(), Box<dyn Error>> {
    let Some(mut session) = Session::get_last(&config)? else {
        // TODO: message should be more like: "no session found", change all other occurrences
        return Err("no active session found")?;
    };

    let was_added = session.tag(&tag);
    session.save()?;
    // TODO: Improve output.
    if was_added {
        println!("Added tag: {tag:?}");
    } else {
        println!("Tag `{tag:?}` already present");
    }
    Ok(())
}

fn untag(config: &Config, tag: &Tag) -> Result<(), Box<dyn Error>> {
    let Some(mut session) = Session::get_last(&config)? else {
        return Err("no active session found")?;
    };

    let was_removed = session.untag(&tag);
    session.save()?;
    // TODO: Improve output.
    if was_removed {
        println!("Removed tag: {tag:?}");
    } else {
        println!("Tag `{tag:?}` not present")
    }
    Ok(())
}

fn write(config: &Config, text: &str) -> Result<(), Box<dyn Error>> {
    let Some(mut session) = Session::get_last(&config)? else {
        return Err("no active session found")?;
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
