use structopt::StructOpt;

#[derive(Debug, StructOpt)]
pub struct Opt {
    path: Option<String>,
    #[structopt(short, long)]
    output: Option<String>,
}

impl Opt {
    pub fn from_args() -> Self {
        StructOpt::from_args()
    }

    pub fn path(&self) -> Option<&str> {
        self.path.as_ref().map(AsRef::as_ref)
    }

    pub fn output(&self) -> Option<&str> {
        self.output.as_ref().map(AsRef::as_ref)
    }
}
