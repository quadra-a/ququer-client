use anyhow::Result;
use serde::Serialize;

use crate::config::Config;

pub fn print_result<T: Serialize>(config: &Config, value: &T) -> Result<()> {
    if config.output == "text" {
        // text mode: pretty-print JSON as fallback
        println!("{}", serde_json::to_string_pretty(value)?);
    } else {
        println!("{}", serde_json::to_string(value)?);
    }
    Ok(())
}

pub fn print_raw(config: &Config, value: &serde_json::Value) -> Result<()> {
    if config.output == "text" {
        println!("{}", serde_json::to_string_pretty(value)?);
    } else {
        println!("{}", value);
    }
    Ok(())
}
