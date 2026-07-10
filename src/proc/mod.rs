mod app;
mod interpreter;
mod stack;

use std::{borrow::Cow, collections::HashMap};

pub(crate) use interpreter::Interpreter;
use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize, Debug)]
pub(crate) struct Action<'a> {
    pub(crate) agent: &'a str,
    pub(crate) prompt: Cow<'a, str>,
}

impl<'a> Action<'a> {
    pub(crate) fn new(agent: &'a str, prompt: &'a str) -> Self {
        Self {
            agent,
            prompt: Cow::Borrowed(prompt),
        }
    }

    pub(crate) fn inject_variables(&mut self, variables: &HashMap<String, Value>) {
        let mut chars = self.prompt.chars().peekable();

        let mut prompt = String::new();
        let mut modified = false;
        while let Some(c) = chars.next() {
            if c == '$' && chars.peek() == Some(&'{') {
                modified = true;
                chars.next();

                let mut var_name = String::new();
                while let Some(&next_c) = chars.peek() {
                    if next_c == '}' {
                        chars.next();
                        break;
                    }
                    var_name.push(next_c);
                    chars.next();
                }

                match variables.get(var_name.as_str()) {
                    Some(value) => prompt.push_str(&value.to_string().trim_matches('"')),
                    None => {
                        prompt.push_str("${");
                        prompt.push_str(&var_name);
                        prompt.push_str("}");
                    }
                }
            } else {
                prompt.push(c);
            }
        }

        if modified {
            self.prompt = Cow::Owned(prompt);
        }
    }
}
