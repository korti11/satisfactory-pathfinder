use colored::Colorize;
use serde::Serialize;

pub struct Formatter {
    pub json_mode: bool,
}

impl Formatter {
    pub fn new(json_mode: bool) -> Self {
        Self { json_mode }
    }

    pub fn print_json<T: Serialize + ?Sized>(&self, value: &T) {
        println!("{}", serde_json::to_string_pretty(value).unwrap());
    }

    pub fn header(&self, text: &str) {
        if !self.json_mode {
            println!("{}", text.bold());
        }
    }

    pub fn field(&self, label: &str, value: &str) {
        if !self.json_mode {
            println!("  {:<16} {}", format!("{}:", label).dimmed(), value);
        }
    }

    pub fn separator(&self) {
        if !self.json_mode {
            println!("{}", "─".repeat(50).dimmed());
        }
    }
}
