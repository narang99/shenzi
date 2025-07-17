use std::io::{self, Write};

use anyhow::Result;

pub fn ask_user(prompt: &str, default: &Option<String>) -> Result<String> {
    let mut value = raw_ask(prompt)?;
    while value.is_empty() {
        match default {
            Some(d) => return Ok(d.to_string()),
            None => {
                println!("empty value not allowed");
                value = raw_ask(prompt)?;
            }
        }
    }
    Ok(value.trim().to_string())
}

fn raw_ask(prompt: &str) -> Result<String> {
    println!("{}", prompt);
    print!("> ");
    io::stdout().flush()?;

    let mut value = String::new();
    io::stdin().read_line(&mut value)?;
    Ok(value.trim().to_string())
}
