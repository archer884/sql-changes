use regex::Regex;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Header<'a> {
    hash: &'a str,
    author: &'a str,
    date: &'a str,
}

#[derive(Debug, Serialize)]
pub struct Changeset<'a> {
    header: &'a Header<'a>,
    path: &'a str,
    additions: Vec<&'a str>,
    deletions: Vec<&'a str>,
}

impl Changeset<'_> {
    pub fn path(&self) -> &str {
        self.path
    }

    pub fn additions(&self) -> String {
        self.additions.join("\n")
    }

    pub fn deletions(&self) -> String {
        self.deletions.join("\n")
    }
}

pub struct PatchParser {
    commit_pattern: Regex,
}

impl PatchParser {
    pub fn new() -> Self {
        Self {
            commit_pattern: Regex::new(r#"From ([A-z0-9]{40}).*\nFrom: (.+)\nDate: (.+)"#).unwrap(),
        }
    }

    pub fn patches<'a, 't>(&'a self, text: &'t str) -> Patches<'a, 't> {
        Patches { parser: self, text }
    }

    fn read_header<'a>(&self, text: &'a str) -> Option<(usize, Header<'a>)> {
        self.commit_pattern.captures(text).map(|x| {
            let header = Header {
                hash: x.get(1).unwrap().as_str().trim(),
                author: x.get(2).unwrap().as_str().trim(),
                date: x.get(3).unwrap().as_str().trim(),
            };
            (x.get(0).unwrap().end(), header)
        })
    }

    fn locations<'a>(&'a self, text: &'a str) -> impl Iterator<Item = usize> + 'a {
        self.commit_pattern.find_iter(text).map(|x| x.start())
    }
}

pub struct Patches<'a, 't> {
    parser: &'a PatchParser,
    text: &'t str,
}

impl<'a, 't> Iterator for Patches<'a, 't> {
    type Item = (Header<'t>, &'t str);

    fn next(&mut self) -> Option<Self::Item> {
        let (patch_start, header) = self.parser.read_header(self.text)?;
        let next_header_start = self.parser.locations(self.text).nth(1);

        match next_header_start {
            Some(patch_end) => {
                let result = (header, &self.text[patch_start..patch_end]);
                self.text = &self.text[patch_end..];
                Some(result)
            }

            None => {
                let result = (header, &self.text[patch_start..]);
                self.text = "";
                Some(result)
            }
        }
    }
}

pub struct ChangesetParser {
    diff_pattern: Regex,
}

impl ChangesetParser {
    pub fn new() -> Self {
        ChangesetParser {
            diff_pattern: Regex::new(r#"diff --git a/(.+) b/(.+)"#).unwrap(),
        }
    }

    pub fn changesets<'a, 't>(
        &'a self,
        header: &'t Header<'t>,
        text: &'t str,
    ) -> Changesets<'a, 't> {
        Changesets {
            parser: self,
            header,
            text,
        }
    }

    fn read_path<'a>(&self, text: &'a str) -> Option<(usize, &'a str)> {
        self.diff_pattern
            .captures(text)
            .map(|x| (x.get(0).unwrap().end(), x.get(2).unwrap().as_str().trim()))
    }

    fn locations<'a>(&'a self, text: &'a str) -> impl Iterator<Item = usize> + 'a {
        self.diff_pattern.find_iter(text).map(|x| x.start())
    }
}

pub struct Changesets<'a, 't> {
    parser: &'a ChangesetParser,
    header: &'t Header<'t>,
    text: &'t str,
}

impl<'a, 't> Iterator for Changesets<'a, 't> {
    type Item = Changeset<'t>;

    fn next(&mut self) -> Option<Self::Item> {
        let (changeset_start, path) = self.parser.read_path(self.text)?;
        let next_changeset_start = self.parser.locations(self.text).nth(1);

        let changes = match next_changeset_start {
            Some(changeset_end) => {
                let changes = &self.text[changeset_start..changeset_end];
                self.text = &self.text[changeset_end..];
                changes
            }

            None => {
                let changes = &self.text[changeset_start..];
                self.text = "";
                changes
            }
        };

        let mut additions = Vec::new();
        let mut deletions = Vec::new();

        for line in changes.lines() {
            if line.starts_with('+') && !line.starts_with("+++") {
                additions.push(line);
            }

            if line.starts_with('-') && !line.starts_with("---") {
                deletions.push(line);
            }
        }

        Some(Changeset {
            header: self.header,
            path,
            additions,
            deletions,
        })
    }
}
