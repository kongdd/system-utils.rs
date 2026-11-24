use std::env;
use std::error::Error;
use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::PathBuf;

pub const USAGE: &str = "lines config list\n       lines config --n-warn N --n-bad N";

#[derive(Clone, Copy)]
pub struct Thresholds {
    pub warn: usize,
    pub bad: usize,
}

impl Thresholds {
    const DEFAULT: Self = Self { warn: 300, bad: 600 };

    pub fn new(warn: usize, bad: usize) -> Result<Self, &'static str> {
        if warn > bad {
            return Err("--n-warn must not exceed --n-bad");
        }
        Ok(Self { warn, bad })
    }

    pub fn load() -> Result<Self, Box<dyn Error>> {
        let Some(path) = config_dir().map(|dir| dir.join("thresholds")) else {
            return Ok(Self::DEFAULT);
        };
        let parse = || -> Result<Self, Box<dyn Error>> {
            let text = match fs::read_to_string(&path) {
                Ok(text) => text,
                Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(Self::DEFAULT),
                Err(err) => return Err(err.into()),
            };
            let (warn, bad) = text.trim().split_once(' ').ok_or("expected two thresholds")?;
            Ok(Self::new(warn.parse()?, bad.parse()?)?)
        };
        parse().map_err(|err| format!("{}: {err}", path.display()).into())
    }
}

fn config_dir() -> Option<PathBuf> {
    let key = if cfg!(windows) { "APPDATA" } else { "XDG_CONFIG_HOME" };
    let base = env::var_os(key)
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| env::home_dir().map(|home| home.join(".config")))?;
    Some(base.join("lines"))
}

pub fn parse_threshold(flag: &str, value: Option<OsString>) -> Result<usize, String> {
    value
        .and_then(|value| value.to_str()?.parse().ok())
        .ok_or_else(|| format!("{flag} requires a non-negative integer"))
}

pub fn run(args: impl Iterator<Item = OsString>) -> Result<(), Box<dyn Error>> {
    let mut args = args.peekable();
    if args.next_if(|arg| arg == "list").is_some() {
        if args.next().is_some() {
            return Err("Usage: lines config list".into());
        }
        let Thresholds { warn, bad } = Thresholds::load()?;
        println!("n-warn = {warn}\nn-bad = {bad}");
        return Ok(());
    }

    let (mut warn, mut bad) = (None, None);
    while let Some(arg) = args.next() {
        let slot = match arg.to_str() {
            Some("-h" | "--help") => {
                println!("Usage: {USAGE}");
                return Ok(());
            }
            Some("--n-warn") if warn.is_none() => &mut warn,
            Some("--n-bad") if bad.is_none() => &mut bad,
            _ => {
                return Err(format!(
                    "unexpected argument: {}\nUsage: {USAGE}",
                    arg.to_string_lossy()
                )
                .into());
            }
        };
        *slot = Some(parse_threshold(&arg.to_string_lossy(), args.next())?);
    }

    let (Some(warn), Some(bad)) = (warn, bad) else {
        return Err(format!("Usage: {USAGE}").into());
    };
    let thresholds = Thresholds::new(warn, bad)?;
    let dir = config_dir().ok_or("could not locate the user configuration directory")?;
    let path = dir.join("thresholds");
    fs::create_dir_all(&dir)
        .and_then(|()| fs::write(&path, format!("{} {}\n", thresholds.warn, thresholds.bad)))
        .map_err(|err| format!("{}: {err}", path.display()))?;
    Ok(())
}
