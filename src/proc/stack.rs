use futures::{StreamExt, stream};
use log::{debug, trace};
use serde_json::Value;
use std::collections::HashMap;

use crate::{llm::LlmChunk, types::StringStream};

pub(in crate::proc) struct FactsStack {
    pub(in crate::proc) context: String,
    pub(in crate::proc) facts: HashMap<String, Value>,
}

impl FactsStack {
    pub(in crate::proc) fn new() -> Self {
        trace!("FactsStack::new() -> Self");
        Self {
            context: String::new(),
            facts: HashMap::new(),
        }
    }

    pub(in crate::proc) fn push_prompt(&mut self, prompt: &str) {
        trace!("FactsStack::push_prompt(&mut self, prompt: &str)");
        debug!("prompt: {}", prompt);

        if !self.context.is_empty() {
            self.context += "\n\n";
        }
        let prompt = match prompt.strip_suffix(".") {
            Some(stripped_prompt) => stripped_prompt,
            None => prompt,
        };
        self.context += prompt;
        self.context += ":\n";
    }

    pub(in crate::proc) fn push_facts(&mut self, facts: &str) {
        trace!("FactsStack::push_facts(&mut self, facts: &str)");
        debug!("facts: {}", facts);

        self.context += facts;

        if let Ok(facts) = serde_json::from_str::<HashMap<String, Value>>(facts) {
            if facts.len() == 1 {
                self.facts.extend(facts.into_iter());
            }
        }
    }

    pub(in crate::proc) fn inject_variables(&self, prompt: &str) -> String {
        let mut result = String::new();
        let mut chars = prompt.chars().peekable();

        while let Some(c) = chars.next() {
            if c == '$' && chars.peek() == Some(&'{') {
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

                match self.facts.get(var_name.as_str()) {
                    Some(value) => result.push_str(&value.to_string().trim_matches('"')),
                    None => {
                        result.push_str("${");
                        result.push_str(&var_name);
                        result.push_str("}");
                    }
                }
            } else {
                result.push(c);
            }
        }

        result
    }

    pub(in crate::proc) fn get_reasoning_stream(&self) -> StringStream {
        // we need to allocate lines on heap because returned stream takes ownership
        let reasoning_lines: Vec<String> = self
            .context
            .split_inclusive('\n')
            .map(|line| LlmChunk::from_reasoning(line))
            .map(|chunk| serde_json::to_string(&chunk))
            .flatten() // unwrap Ok variants while filtering out the Err variants
            .collect();
        let reasoning_stream = stream::iter(reasoning_lines)
            .map(|line| Ok(format!("{}\n", line)))
            .boxed();
        reasoning_stream
    }
}
