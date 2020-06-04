use regex::Regex;
use serde::Serialize;
use std::io::Read;
use std::{fs, io, iter};
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
struct Opt {
    path: Option<String>,
}

impl Opt {
    fn path(&self) -> Option<&str> {
        self.path.as_ref().map(AsRef::as_ref)
    }
}

#[derive(Clone, Debug)]
enum Event<'a> {
    Commit(&'a str),
    Diff(Diff<'a>),
    Addition(&'a str),
    Deletion(&'a str),
}

impl Event<'_> {
    fn is_diff(&self) -> bool {
        match self {
            Event::Diff(_) => true,
            _ => false,
        }
    }
}

#[derive(Clone, Debug)]
struct Diff<'a> {
    left: &'a str,
    right: &'a str,
}

struct EventPathFilter {
    take: bool,
    path: String,
}

impl EventPathFilter {
    fn new(path: impl Into<String>) -> Self {
        Self {
            take: false,
            path: path.into(),
        }
    }

    fn take(&mut self, event: &Event) -> bool {
        if let Event::Diff(diff) = event {
            self.take = diff.right.contains(&self.path);
        }
        self.take
    }
}

#[derive(Debug)]
struct Summary<'a> {
    commit: Option<&'a str>,
    path: &'a str,
    additions: Vec<Vec<&'a str>>,
    deletions: Vec<Vec<&'a str>>,
}

#[derive(Debug, Serialize)]
struct SummaryJsonFormatter {
    commit: Option<String>,
    path: String,
    additions: String,
    deletions: String,
}

impl<'a> Summary<'a> {
    fn new(path: &'a str) -> Self {
        Self {
            commit: None,
            path,
            additions: Vec::new(),
            deletions: Vec::new(),
        }
    }

    fn set_hash(&mut self, hash: &'a str) {
        self.commit = Some(hash);
    }

    fn to_json_formatter(self) -> SummaryJsonFormatter {
        fn format_changes<'a>(changes: Vec<Vec<&'a str>>) -> String {
            let mut buf = String::new();
            let mut change_sets = changes.into_iter();

            if let Some(initial) = change_sets.next() {
                for line in initial {
                    buf += line;
                    buf += "\n";
                }
            }

            for set in change_sets {
                buf += " ...\n";
                for line in set {
                    buf += line;
                    buf += "\n";
                }
            }

            buf
        }

        let additions = format_changes(self.additions);
        let deletions = format_changes(self.deletions);

        SummaryJsonFormatter {
            commit: self.commit.map(Into::into),
            path: self.path.into(),
            additions,
            deletions,
        }
    }
}

struct Partition<'a, I: Iterator + 'a, P> {
    source: &'a mut iter::Peekable<I>,
    predicate: P,
}

impl<'a, I, P> Partition<'a, I, P>
where
    I: Iterator + 'a,
    P: FnMut(&I::Item) -> bool,
{
    fn new(source: &'a mut iter::Peekable<I>, predicate: P) -> Self {
        Self { source, predicate }
    }
}

impl<'a, I, P> Iterator for Partition<'a, I, P>
where
    I: Iterator + 'a,
    P: FnMut(&I::Item) -> bool,
{
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        match self.source.peek().map(&mut self.predicate) {
            Some(true) => self.source.next(),
            _ => None,
        }
    }
}

struct SummaryAdapter<'a, I: Iterator + 'a> {
    source: iter::Peekable<I>,
    commit: Option<&'a str>,
}

impl<'a, I: Iterator<Item = Event<'a>> + 'a> SummaryAdapter<'a, I> {
    fn new(source: I) -> SummaryAdapter<'a, I> {
        Self {
            source: source.peekable(),
            commit: None,
        }
    }
}

impl<'a, T: Iterator<Item = Event<'a>>> Iterator for SummaryAdapter<'a, T> {
    type Item = Summary<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut has_emitted = false;
        let mut partition = Partition::new(&mut self.source, |event| {
            let result = !has_emitted || !event.is_diff();
            if event.is_diff() {
                has_emitted = true;
            }
            result
        });

        let mut summary = match partition.next() {
            Some(Event::Diff(diff)) => Summary::new(diff.right),
            _ => return None,
        };

        let mut last_was_addition = false;
        for event in partition {
            match event {
                Event::Commit(hash) => {
                    if let Some(x) = self.commit {
                        summary.set_hash(x);
                    }
                    self.commit = Some(hash);
                    return Some(summary);
                }
                
                // These branches filter out the BOM.
                Event::Addition(addition) if addition != "∩╗┐" => {
                    if summary.additions.is_empty() || !last_was_addition {
                        last_was_addition = true;
                        summary.additions.push(vec![addition]);
                    } else {
                        summary.additions.last_mut().unwrap().push(addition)
                    }
                }

                Event::Deletion(deletion) if deletion != "∩╗┐" => {
                    if summary.deletions.is_empty() || last_was_addition {
                        last_was_addition = false;
                        summary.deletions.push(vec![deletion]);
                    } else {
                        summary.deletions.last_mut().unwrap().push(deletion);
                    }
                }

                // We're really not interested in the munged byte order mark.
                Event::Addition(_) | Event::Deletion(_) => (),

                _ => panic!("Shouldn't be possible to receive a second Event::Diff"),
            }
        }

        if let Some(x) = self.commit {
            summary.set_hash(x);
        }

        Some(summary)
    }
}

fn main() -> io::Result<()> {
    let input = match Opt::from_args().path() {
        Some(path) => fs::read_to_string(path)?,
        None => read_stdin()?,
    };

    let mut filter = EventPathFilter::new("/dbo/");
    let selector = EventSelector::new();
    let events = input
        .lines()
        .filter_map(|x| selector.select_event(x))
        .filter(|event| filter.take(event));

    let change_summaries: Vec<_> = SummaryAdapter::new(events)
        .map(|x| x.to_json_formatter())
        .collect();

    serde_json::to_writer_pretty(&mut std::io::stdout(), &change_summaries)?;
    Ok(())
}

struct EventSelector {
    commit_pattern: Regex,
}

impl EventSelector {
    pub fn new() -> Self {
        Self { commit_pattern: Regex::new(r#"From ([A-z0-9]{40})"#).unwrap() }
    }

    pub fn select_event<'a>(&self, s: &'a str) -> Option<Event<'a>> {
        if s.is_empty() || s.len() < 2 {
            return None;
        }
    
        if s.starts_with('+') && !s.starts_with("+++") {
            return Some(Event::Addition(&s[1..]));
        }
    
        if s.starts_with('-') && !s.starts_with("---") {
            return Some(Event::Deletion(&s[1..]));
        }
    
        if s.starts_with("diff --git") {
            return build_diff(s).map(Event::Diff);
        }
    
        if let Some(x) = self.commit_pattern.captures(s) {
            return Some(Event::Commit(x.get(1).unwrap().as_str()));
        }

        None
    }
}

fn build_diff(s: &str) -> Option<Diff> {
    let locations = s.find("a/").and_then(|a| s.rfind("b/").map(|b| (a, b)));
    locations.map(|(left, right)| Diff {
        left: s[left + 2..right].trim(),
        right: &s[right + 2..],
    })
}

fn read_stdin() -> io::Result<String> {
    let mut buf = String::new();
    io::stdin().read_to_string(&mut buf)?;
    Ok(buf)
}
