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

#[derive(Debug, Serialize)]
struct Summary<'a> {
    path: &'a str,
    additions: Vec<&'a str>,
    deletions: Vec<&'a str>,
}

impl<'a> Summary<'a> {
    fn new(path: &'a str) -> Self {
        Self {
            path,
            additions: Vec::new(),
            deletions: Vec::new(),
        }
    }

    fn from_events<I: Iterator<Item = Event<'a>>>(mut source: iter::Peekable<I>) -> Self {
        let mut summary = match source.next() {
            Some(Event::Diff(diff)) => Summary::new(diff.right),
            _ => return Summary::new(""),
        };

        for event in source {
            match event {
                // These branches filter out the BOM.
                Event::Addition(addition) if addition != "∩╗┐" => {
                    summary.additions.push(addition)
                }
                Event::Deletion(deletion) if deletion != "∩╗┐" => {
                    summary.deletions.push(deletion)
                }

                // We're really not interested in the munged byte order mark.
                Event::Addition(_) | Event::Deletion(_) => (),

                _ => panic!("Shouldn't be possible to receive a second Event::Diff"),
            }
        }

        summary
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

struct SummaryAdapter<I: Iterator> {
    source: iter::Peekable<I>,
}

impl<'a, I: Iterator<Item = Event<'a>>> SummaryAdapter<I> {
    fn new(source: I) -> SummaryAdapter<I> {
        Self {
            source: source.peekable(),
        }
    }
}

impl<'a, T: Iterator<Item = Event<'a>>> Iterator for SummaryAdapter<T> {
    type Item = Summary<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut has_emitted = false;
        let partition = Partition::new(&mut self.source, |event| {
            let result = !has_emitted || !event.is_diff();
            has_emitted = true;
            result
        });

        let mut partition = partition.peekable();
        if partition.peek().is_some() {
            Some(Summary::from_events(partition))
        } else {
            None
        }
    }
}

fn main() -> io::Result<()> {
    let input = match Opt::from_args().path() {
        Some(path) => fs::read_to_string(path)?,
        None => read_stdin()?,
    };

    let mut filter = EventPathFilter::new("/dbo/");
    let events = input
        .lines()
        .filter_map(select_event)
        .filter(|event| filter.take(event));

    let change_summaries: Vec<_> = SummaryAdapter::new(events).collect();
    serde_json::to_writer_pretty(&mut std::io::stdout(), &change_summaries)?;
    Ok(())
}

fn select_event(s: &str) -> Option<Event> {
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

    None
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
