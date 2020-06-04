mod opt;
mod patch;

use patch::ChangesetParser;
use std::io::Read;
use std::{fs, io};

fn main() -> io::Result<()> {
    let patch = match opt::Opt::from_args().path() {
        Some(path) => fs::read_to_string(path)?,
        None => read_stdin()?,
    };

    let cparser = ChangesetParser::new();
    let sets: Vec<_> = cparser
        .changesets(&patch)
        .filter(|x| x.path().contains("/dbo/"))
        .map(|x| x.to_json_formatter())
        .collect();

    serde_json::to_writer_pretty(&mut std::io::stdout(), &sets)?;
    Ok(())
}

fn read_stdin() -> io::Result<String> {
    let mut buf = String::new();
    io::stdin().read_to_string(&mut buf)?;
    Ok(buf)
}
