use std::iter::{self, FromIterator};

#[derive(Clone, Debug)]
enum Event<'a> {
    // diff --git a/DocGen/DeployToCi.scmp b/DocGen/DeployToCi.scmp
    Diff(Diff<'a>),

    // +some text
    Addition(&'a str),

    // -some text here
    Deletion(&'a str),

    // new file
    NewFile,
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

#[derive(Debug, Default)]
struct AdditionSummary<'a> {
    additions: Vec<&'a str>,
}

#[derive(Debug, Default)]
struct DeletionSummary<'a> {
    deletions: Vec<&'a str>,
}

#[derive(Debug)]
struct Summary<'a> {
    path: &'a str,
    additions: Vec<AdditionSummary<'a>>,
    deletions: Vec<DeletionSummary<'a>>,
}

impl<'a> Summary<'a> {
    fn new(path: &'a str) -> Self {
        Self {
            path,
            additions: Vec::new(),
            deletions: Vec::new(),
        }
    }
}

impl<'a> FromIterator<Event<'a>> for Summary<'a> {
    fn from_iter<T: IntoIterator<Item = Event<'a>>>(iter: T) -> Self {
        let mut source = iter.into_iter();
        let mut summary = match source.next() {
            Some(Event::Diff(diff)) => Summary::new(diff.right),
            _ => return Summary::new(""),
        };

        let mut last_was_addition = false;
        for event in source {
            match event {
                Event::Addition(addition) => {
                    if summary.additions.is_empty() || !last_was_addition {
                        last_was_addition = true;
                        let mut additions = AdditionSummary::default();
                        additions.additions.push(addition);
                        summary.additions.push(additions);
                    }
                }

                Event::Deletion(deletion) => {

                }

                _ => panic!("Shouldn't be possible to receive a second Event::Diff"),
            }
        }
        unimplemented!()
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

struct SummaryAdapter<'a, I: Iterator> {
    source: iter::Peekable<I>,
    summary: Option<Summary<'a>>,
}

impl<'a, I: Iterator<Item = Event<'a>>> SummaryAdapter<'a, I> {
    fn new(source: I) -> SummaryAdapter<'a, I> {
        Self {
            source: source.peekable(),
            summary: None,
        }
    }
}

impl<'a, T: Iterator<Item = Event<'a>>> Iterator for SummaryAdapter<'a, T> {
    type Item = Summary<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut has_emitted = true;
        let partition = Partition::new(&mut self.source, |event| {
            let result = !has_emitted || event.is_diff();
            has_emitted = true;
            result
        });
        Some(partition.collect())
    }
}

fn main() {
    // FIXME: obviously, this path ain't gonna work.
    let mut filter = EventPathFilter::new("/dbo/");
    let input = include_str!("../../../Documents/diff.txt");
    let events = input
        .lines()
        .filter_map(select_event)
        .filter(|event| filter.take(event));

    let mut count = 0;
    for summary in SummaryAdapter::new(events) {
        count += 1;
        println!("{}: {:?}", count, summary);
    }
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
