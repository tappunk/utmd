use serde::Serialize;

#[derive(Debug, Clone)]
pub struct Reporter {
    json: bool,
    quiet: bool,
}

impl Reporter {
    pub fn new(json: bool, quiet: bool) -> Self {
        Self { json, quiet }
    }

    pub fn is_json(&self) -> bool {
        self.json
    }

    pub fn info(&self, msg: &str) {
        if self.quiet || self.json {
            return;
        }
        eprintln!("info: {}", msg);
    }

    pub fn error(&self, msg: &str) {
        if self.json {
            return;
        }
        eprintln!("error: {}", msg);
    }

    pub fn print_json<T: Serialize>(&self, payload: &T) -> color_eyre::Result<()> {
        println!("{}", serde_json::to_string_pretty(payload)?);
        Ok(())
    }

    pub fn print_stdout(&self, msg: &str) {
        if self.json {
            return;
        }
        println!("{}", msg);
    }
}
