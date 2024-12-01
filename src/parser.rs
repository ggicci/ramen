use clap::{Arg, Command};
use thiserror::Error;
use yaml_rust::{ScanError, Yaml, YamlLoader};

use once_cell::sync::Lazy;
use regex::Regex;

static REG_SHORT_LONG_ARG_NAME: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^-(?P<short>[a-zA-Z])/--(?P<long>[a-zA-Z][a-zA-Z0-9-]*)$").unwrap());

#[derive(Debug, Error)]
pub enum Error {
    #[error("parse yaml error: {0}")]
    ParseYaml(#[from] ScanError),

    #[error("no docs detected in the given yaml")]
    NoDocs,

    #[error("multi-docs detected in the given yaml, which is not supported")]
    MultiDocs,
}

pub struct ArgumentParser {
    doc: Yaml,
}

impl ArgumentParser {
    pub fn new(doc: Yaml) -> Self {
        Self { doc }
    }

    /// The version of the spec.
    pub fn version(&self) -> &str {
        self.doc["version"].as_str().unwrap_or_default()
    }

    /// The name of the program.
    pub fn program(&self) -> &str {
        self.doc["program"].as_str().unwrap_or_default()
    }

    /// A description of the program.
    pub fn about(&self) -> &str {
        self.doc["about"].as_str().unwrap_or_default()
    }

    /// Create a list of Argument instance by parsing the `args` definitions.
    pub fn args(&self) -> Vec<Argument> {
        self.doc["args"]
            .as_vec()
            .map(|vec| vec.iter().map(|item| Argument::new(item.clone())).collect())
            .unwrap_or_default()
    }
}

/// Represents a [`clap::Arg`], see tutorial:
/// https://docs.rs/clap/latest/clap/_tutorial/chapter_2/index.html
pub struct Argument {
    doc: Yaml,
}

impl Argument {
    pub fn new(doc: Yaml) -> Self {
        Self { doc }
    }

    pub fn bare_name(&self) -> Option<&str> {
        self.doc.as_str()
    }

    pub fn name(&self) -> &str {
        self.bare_name()
            .or(self.doc["name"].as_str())
            .unwrap_or_default()
    }

    /// Provide the short arg name, ex. -c, -d, -t, etc.
    pub fn short(&self) -> Option<char> {
        match self.bare_name() {
            Some(name) => extract_short_long_name(name)
                .map(|(short, _)| short.chars().next())
                .flatten(),
            None => self.doc["short"]
                .as_str()
                .map(|x| x.chars().next())
                .flatten(),
        }
    }

    /// Provide the long arg name, ex. --file, --num-threads, etc.
    pub fn long(&self) -> Option<String> {
        match self.bare_name() {
            Some(name) => extract_short_long_name(name).map(|(_, long)| long),
            None => self.doc["long"].as_str().map(|x| x.to_string()),
        }
    }

    /// The type of the argument, can be string, number, boolean.
    pub fn r#type(&self) -> &str {
        self.doc["type"].as_str().unwrap_or("string")
    }

    /// The default value of the argument on absent.
    pub fn default(&self) -> &str {
        self.doc["default"].as_str().unwrap_or_default()
    }

    pub fn select(&self) -> Option<Vec<&str>> {
        self.doc["select"]
            .as_vec()
            .map(|x| x.iter().map(|v| v.as_str().unwrap_or_default()).collect())
    }
}

pub fn parse(yaml: &str) -> Result<String, Error> {
    let res = String::new();

    let mut docs = YamlLoader::load_from_str(yaml)?;
    validate_root_docs(&docs)?;

    let parser = ArgumentParser::new(docs.remove(0));
    let mut command = Command::new(parser.program().to_string()).about(parser.about().to_string());

    let args = parser.args();
    for arg in args.iter() {
        let mut clap_arg = Arg::new(arg.name().to_string()).short(arg.short());
        if let Some(long) = arg.long() {
            clap_arg = clap_arg.long(long);
        }
        command = command.arg(clap_arg);
    }
    command.build();

    let matches = command.get_matches();
    println!("{:?}", matches);
    Ok("".to_string())
}

fn validate_root_docs(docs: &Vec<Yaml>) -> Result<(), Error> {
    if docs.len() == 0 {
        return Err(Error::NoDocs);
    }
    if docs.len() > 1 {
        return Err(Error::MultiDocs);
    }
    Ok(())
}

/// Extract the short and long name from the given text when it complies to the pattern `-s/--long`.
fn extract_short_long_name(haystack: &str) -> Option<(String, String)> {
    if let Some(captures) = REG_SHORT_LONG_ARG_NAME.captures(haystack) {
        let short_name = captures.name("short").unwrap().as_str();
        let long_name = captures.name("long").unwrap().as_str();
        Some((short_name.to_string(), long_name.to_string()))
    } else {
        None
    }
}

#[cfg(test)]
mod test {
    use yaml_rust::{Yaml, YamlLoader};

    use super::{Argument, ArgumentParser};

    #[test]
    fn get_program() -> anyhow::Result<()> {
        let parser = ArgumentParser::new(load_yaml("program: hello")?);
        assert_eq!("hello", parser.program());
        Ok(())
    }

    #[test]
    fn arg_bare_name() -> anyhow::Result<()> {
        let doc = load_yaml("SRC")?;
        let parg = Argument::new(doc);
        assert_eq!(Some("SRC"), parg.bare_name());
        assert_eq!("SRC", parg.name());
        Ok(())
    }

    #[test]
    fn arg_name() -> anyhow::Result<()> {
        let doc = load_yaml(
            r#"
        name: DEST
        "#,
        )?;
        let parg = Argument::new(doc);
        assert!(parg.bare_name().is_none());
        assert_eq!("DEST", parg.name());
        Ok(())
    }

    fn load_yaml(yaml: &str) -> anyhow::Result<Yaml> {
        let mut docs = YamlLoader::load_from_str(yaml)?;
        Ok(docs.remove(0))
    }
}
