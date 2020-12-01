mod opt;
mod patch;

use std::{
    fs::{self, File},
    io::{self, Read},
};

use bumpalo::Bump;
use opt::Opt;
use patch::{Changeset, ChangesetParser, PatchParser};
use serde::Serialize;

// Usage:
// git format-patch `
//     --stdout 7a73e12a137433d030d10dbc05705ab48240e332..a5d58f842b7de075c3dcc73eefe0f1737fcb28ec `
//     | sql-changes.exe `
//     > output.json

// Import into excel as Data > New Query > From Other Sources > Web
// URL: file:///C:/users/jarcher/src/changes.json
// Convert to table, then select columns

#[derive(Debug, Serialize)]
struct JsonFormatter<'a> {
    path: &'a str,
    additions: String,
    deletions: String,
}

impl<'a> JsonFormatter<'a> {
    fn new(changeset: &'a Changeset) -> Self {
        Self {
            path: changeset.path(),
            additions: changeset.additions(),
            deletions: changeset.deletions(),
        }
    }
}

fn main() -> io::Result<()> {
    let opt = opt::Opt::from_args();
    let patch = match opt.path() {
        Some(path) => fs::read_to_string(path)?,
        None => read_stdin()?,
    };

    let hparser = PatchParser::new();
    let cparser = ChangesetParser::new();
    let headers = Bump::new();

    let mut sets = Vec::new();
    for (header, patch) in hparser.patches(&patch) {
        let header = headers.alloc(header);
        let extend_from = cparser
            .changesets(header, patch)
            .filter(|x| x.path().contains("/dbo/"));
        sets.extend(extend_from);
    }

    let mut writer = get_writer(&opt)?;
    let mapped_sets: Vec<_> = sets.iter().map(|x| JsonFormatter::new(x)).collect();
    serde_json::to_writer_pretty(&mut writer, &mapped_sets)?;

    Ok(())
}

fn read_stdin() -> io::Result<String> {
    let mut buf = String::new();
    io::stdin().read_to_string(&mut buf)?;
    Ok(buf)
}

fn get_writer(opt: &Opt) -> io::Result<Box<dyn io::Write>> {
    match opt.output() {
        Some(path) => File::open(path).map(|x| Box::new(x) as Box<dyn io::Write>),
        None => Ok(Box::new(io::stdout())),
    }
}
