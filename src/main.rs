mod opt;
mod patch;

use bumpalo::Bump;
use patch::{Changeset, ChangesetParser, PatchParser};
use std::{
    fs,
    io::{self, Read},
};

// Usage:
// git format-patch `
//     --stdout 7a73e12a137433d030d10dbc05705ab48240e332..a5d58f842b7de075c3dcc73eefe0f1737fcb28ec `
//     | sql-changes.exe `
//     > output.json

#[derive(Debug, serde::Serialize)]
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

    let mut writer = opt
        .output()
        .and_then(|path| {
            std::fs::File::open(path)
                .ok()
                .map(|file| Box::new(file) as Box<dyn std::io::Write>)
        })
        .unwrap_or_else(|| Box::new(io::stdout()));

    let mapped_sets: Vec<_> = sets.iter().map(|x| JsonFormatter::new(x)).collect();
    serde_json::to_writer_pretty(&mut writer, &mapped_sets)?;

    Ok(())
}

fn read_stdin() -> io::Result<String> {
    let mut buf = String::new();
    io::stdin().read_to_string(&mut buf)?;
    Ok(buf)
}
