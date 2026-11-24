mod config;

use config::Thresholds;
use ignore::WalkBuilder;
use std::borrow::Cow;
use std::env;
use std::error::Error;
use std::ffi::{OsStr, OsString};
use std::fs::File;
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use unicode_width::UnicodeWidthStr;

const USAGE: &str = "Usage: lines [-d DIR] [--tests] --exts EXT[,EXT...]";
const FILTER_USAGE: &str =
    "lines [--warn] [--bad] [--n-warn N] [--n-bad N] [-d DIR] [--tests] [--exts EXT[,EXT...]]";
const DIRECTORY: &str = "\x1b[1;36m";
const BOLD: &str = "\x1b[1m";
const GOOD: &str = "\x1b[32m";
const WARN: &str = "\x1b[1;33m";
const BAD: &str = "\x1b[1;31m";
const BRANCH: &str = "\x1b[90m";
const RESET: &str = "\x1b[0m";
const BAR: &str = "++++++++++++++++++++";

struct Entry {
    name: OsString,
    lines: usize,
    children: Vec<Entry>,
}

#[derive(Clone, Copy)]
enum Level {
    Normal,
    Warn,
    Bad,
}

impl Level {
    fn of(lines: usize, thresholds: Thresholds) -> Self {
        match lines {
            n if n > thresholds.bad => Self::Bad,
            n if n >= thresholds.warn => Self::Warn,
            _ => Self::Normal,
        }
    }

    fn color(self) -> &'static str {
        match self {
            Self::Normal => "",
            Self::Warn => WARN,
            Self::Bad => BAD,
        }
    }
}

fn count_lines(path: &Path, text_only: bool) -> io::Result<Option<usize>> {
    let mut reader = BufReader::new(File::open(path)?);
    if text_only && reader.fill_buf()?.contains(&0) {
        return Ok(None);
    }
    let mut lines = 0;
    while reader.skip_until(b'\n')? != 0 {
        lines += 1;
    }
    Ok(Some(lines))
}

fn close_dir(dirs: &mut Vec<Entry>) {
    let dir = dirs.pop().unwrap();
    if !dir.children.is_empty() {
        let parent = dirs.last_mut().unwrap();
        parent.lines += dir.lines;
        parent.children.push(dir);
    }
}

fn scan(dir: &Path, exts: &[&str], tests: bool) -> Result<(Vec<Entry>, usize), Box<dyn Error>> {
    let walk = WalkBuilder::new(dir)
        .standard_filters(false)
        .hidden(true)
        .parents(true)
        .git_ignore(true)
        .require_git(false)
        .filter_entry(move |entry| {
            let name = entry.file_name();
            !name.as_encoded_bytes().starts_with(b".")
                && (tests
                    || !entry.file_type().is_some_and(|kind| kind.is_dir())
                    || !matches!(name.to_str(), Some("test" | "tests")))
        })
        .sort_by_file_name(|a, b| a.cmp(b))
        .build();
    let mut dirs = vec![Entry { name: OsString::new(), lines: 0, children: Vec::new() }];
    let mut longest = 0;

    for item in walk {
        let item = item?;
        if let Some(err) = item.error() {
            eprintln!("lines: {err}");
        }
        let Some(kind) = item.file_type() else {
            continue;
        };
        if item.depth() == 0 {
            if !kind.is_dir() {
                return Err(format!("{}: not a directory", dir.display()).into());
            }
            continue;
        }

        while dirs.len() > item.depth() {
            close_dir(&mut dirs);
        }
        let name = item.file_name().to_owned();
        if kind.is_dir() {
            dirs.push(Entry { name, lines: 0, children: Vec::new() });
        } else if kind.is_file()
            && (exts.is_empty()
                || item.path().extension().is_some_and(|ext| exts.iter().any(|e| ext == *e)))
        {
            let Some(lines) = count_lines(item.path(), exts.is_empty())
                .map_err(|err| format!("{}: {err}", item.path().display()))?
            else {
                continue;
            };
            longest = longest.max(lines);
            let parent = dirs.last_mut().unwrap();
            parent.lines += lines;
            parent.children.push(Entry { name, lines, children: Vec::new() });
        }
    }

    while dirs.len() > 1 {
        close_dir(&mut dirs);
    }
    Ok((dirs.pop().unwrap().children, longest))
}

fn display_name(name: &OsStr) -> Cow<'_, str> {
    let name = name.to_string_lossy();
    if !name.chars().any(char::is_control) {
        return name;
    }

    let mut escaped = String::with_capacity(name.len());
    for ch in name.chars() {
        if ch.is_control() {
            escaped.extend(ch.escape_default());
        } else {
            escaped.push(ch);
        }
    }
    Cow::Owned(escaped)
}

fn tree_width(entries: &[Entry]) -> usize {
    entries
        .iter()
        .map(|entry| 4 + display_name(&entry.name).width().max(tree_width(&entry.children)))
        .max()
        .unwrap_or(0)
}

fn format_count(lines: usize) -> String {
    let digits = lines.to_string();
    let first = (digits.len() - 1) % 3 + 1;
    let mut count = String::from(&digits[..first]);
    for i in (first..digits.len()).step_by(3) {
        count.push(',');
        count.push_str(&digits[i..i + 3]);
    }
    count
}

fn print_count(
    out: &mut impl Write,
    column: usize,
    label_width: usize,
    lines: usize,
    color: &str,
    marks: usize,
) -> io::Result<()> {
    let count = format_count(lines);
    let padding = column - label_width - count.len();
    write!(out, "{:padding$}{color}({count})", "")?;
    if marks > 0 {
        write!(out, " {}", &BAR[..marks])?;
    }
    writeln!(out, "{RESET}")
}

fn print_tree(
    out: &mut impl Write,
    entries: &[Entry],
    prefix: &mut String,
    thresholds: Thresholds,
    column: usize,
    lines_per_mark: usize,
) -> io::Result<()> {
    for (i, entry) in entries.iter().enumerate() {
        let last = i + 1 == entries.len();
        let branch = if last { "└── " } else { "├── " };
        let is_dir = !entry.children.is_empty();
        let lines = entry.lines;
        let (name_style, count_style) = if is_dir {
            (DIRECTORY, BOLD)
        } else {
            let color = Level::of(lines, thresholds).color();
            (color, color)
        };
        let name = display_name(&entry.name);
        write!(out, "{BRANCH}{prefix}{branch}{RESET}{name_style}{name}{RESET}")?;
        print_count(
            out,
            column,
            prefix.as_str().width() + branch.width() + name.width(),
            lines,
            count_style,
            if is_dir { 0 } else { lines.div_ceil(lines_per_mark) },
        )?;

        if is_dir {
            let len = prefix.len();
            prefix.push_str(if last { "    " } else { "│   " });
            print_tree(out, &entry.children, prefix, thresholds, column, lines_per_mark)?;
            prefix.truncate(len);
        }
    }
    Ok(())
}

fn collect_files(
    entries: &[Entry],
    dir: &Path,
    include: &impl Fn(usize) -> bool,
    files: &mut Vec<(PathBuf, usize)>,
) {
    for entry in entries {
        let path = dir.join(&entry.name);
        if !entry.children.is_empty() {
            collect_files(&entry.children, &path, include, files);
        } else if include(entry.lines) {
            files.push((path, entry.lines));
        }
    }
}

fn print_filtered(
    out: &mut impl Write,
    entries: &[Entry],
    thresholds: Thresholds,
    warn: bool,
    bad: bool,
    lines_per_mark: usize,
) -> io::Result<()> {
    let mut files = Vec::new();
    let include = |lines| match Level::of(lines, thresholds) {
        Level::Normal => false,
        Level::Warn => warn,
        Level::Bad => warn || bad,
    };
    collect_files(entries, Path::new(""), &include, &mut files);
    if files.is_empty() {
        let level = match (warn, bad) {
            (true, true) => "bad/warn",
            (true, false) => "warn",
            _ => "bad",
        };
        return writeln!(out, "{BOLD}{GOOD}[ok] no {level} file{RESET}");
    }
    let column = files
        .iter()
        .map(|(path, lines)| {
            display_name(path.as_os_str()).width() + format_count(*lines).len() + 2
        })
        .max()
        .unwrap_or(0);

    for (path, lines) in files {
        let name = display_name(path.as_os_str());
        let color = Level::of(lines, thresholds).color();
        write!(out, "{color}{name}{RESET}")?;
        print_count(out, column, name.width(), lines, color, lines.div_ceil(lines_per_mark))?;
    }
    Ok(())
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut args = env::args_os().skip(1).peekable();
    if args.next_if(|arg| arg == "config").is_some() {
        return config::run(args);
    }
    let (mut warn, mut bad) = (false, false);
    let (mut n_warn, mut n_bad) = (None, None);
    let mut dir = None;
    let mut exts = None;
    let mut tests = false;

    while let Some(arg) = args.next() {
        let (flag, value) = match arg.to_str().and_then(|s| s.split_once('=')) {
            Some((flag @ ("--n-warn" | "--n-bad"), value)) => (Some(flag), Some(value)),
            _ => (arg.to_str(), None),
        };
        match flag {
            Some("-h" | "--help") => {
                println!("{USAGE}\n       {FILTER_USAGE}\n       {}", config::USAGE);
                println!("\n--warn includes warning and bad files; --bad shows only bad files.");
                println!("Threshold overrides imply --warn unless a filter is specified.");
                return Ok(());
            }
            Some("--warn") => warn = true,
            Some("--bad") => bad = true,
            Some("--tests") => tests = true,
            Some(flag @ ("--n-warn" | "--n-bad")) => {
                let threshold = if flag == "--n-warn" { &mut n_warn } else { &mut n_bad };
                if threshold.is_some() {
                    return Err(format!("duplicate argument: {flag}").into());
                }
                let value = value.map(OsString::from).or_else(|| args.next());
                *threshold = Some(config::parse_threshold(flag, value)?);
            }
            Some("-d") if dir.is_none() => {
                dir = Some(PathBuf::from(args.next().ok_or("-d requires a directory")?));
            }
            Some("--exts") if exts.is_none() => {
                exts = Some(
                    args.next()
                        .and_then(|value| value.into_string().ok())
                        .filter(|value| {
                            !value.starts_with('-') && value.split(',').all(|e| !e.is_empty())
                        })
                        .ok_or("--exts requires a comma-separated list of extensions")?,
                );
            }
            _ => {
                return Err(format!("unexpected argument: {}", arg.to_string_lossy()).into());
            }
        }
    }

    if !warn && !bad && (n_warn.is_some() || n_bad.is_some()) {
        warn = true;
    }
    let filtering = warn || bad;
    let dir = dir.unwrap_or_else(|| PathBuf::from("."));
    let exts: Vec<_> = match exts.as_deref() {
        Some(exts) => exts.split(',').collect(),
        None if filtering => Vec::new(),
        None => return Err(USAGE.into()),
    };
    let defaults = Thresholds::load()?;
    let thresholds =
        Thresholds::new(n_warn.unwrap_or(defaults.warn), n_bad.unwrap_or(defaults.bad))?;
    let (entries, longest) = scan(&dir, &exts, tests)?;
    let lines_per_mark = longest.div_ceil(BAR.len()).max(10);
    let mut out = BufWriter::new(anstream::stdout().lock());
    if filtering {
        print_filtered(&mut out, &entries, thresholds, warn, bad, lines_per_mark)?;
        return Ok(out.flush()?);
    }
    let total: usize = entries.iter().map(|entry| entry.lines).sum();
    let root = display_name(dir.as_os_str());
    let column = root.width().max(tree_width(&entries)) + format_count(total).len() + 2;
    write!(out, "{DIRECTORY}{root}{RESET}")?;
    print_count(&mut out, column, root.width(), total, BOLD, 0)?;
    print_tree(&mut out, &entries, &mut String::new(), thresholds, column, lines_per_mark)?;
    out.flush()?;
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("lines: {err}");
            ExitCode::FAILURE
        }
    }
}
