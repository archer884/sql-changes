enum Event<'a> {
    // diff --git a/DocGen/DeployToCi.scmp b/DocGen/DeployToCi.scmp
    Diff { left: &'a str, right: &'a str },

    // +some text
    Addition(&'a str),

    // -some text here
    Deletion(&'a str),

    // new file
    NewFile,
}

struct Summary {
    file_name: String,
    additions: String,
    deletions: String,
}

struct Selector {
}

impl Selector {
    fn select(&self, s: &str) -> Option<Event> {
        if s.starts_with('+') {
            return Some(Event::Addition(&s[1..]));
        }

        if s.starts_with('-') {
            return Some(Event::Deletion(&s[1..]));
        }

        if s.starts_with("diff --git") {
            return Some(Event::Diff {
                
            })
        }
    }
}

fn main() {
    
}
