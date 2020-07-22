mod opt;
mod patch;

use bumpalo::Bump;
use patch::{ChangesetCsvFormatter, ChangesetParser, PatchParser};
use std::io::Read;
use std::{fs, io};

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

    // println!("{:#?}", sets);
    // println!("Count: {}", sets.len());

    let mut writer = match opt.output() {
        Some(path) => Box::new(std::fs::File::create(path)?) as Box<dyn std::io::Write>,
        None => Box::new(io::stdout()),
    };
    let mut writer = csv::Writer::from_writer(&mut writer);

    for record in sets.iter().map(ChangesetCsvFormatter::new) {
        writer.serialize(record)?;
    }

    Ok(())
}

fn read_stdin() -> io::Result<String> {
    let mut buf = String::new();
    io::stdin().read_to_string(&mut buf)?;
    Ok(buf)
}
