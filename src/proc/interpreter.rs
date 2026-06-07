use crate::agent::hera::HeraAgent;
use crate::agent::printer::PrinterAgent;
use crate::agent::query::QueryAgent;
use crate::agent::time::TimeServiceAgent;
use crate::agent::units::MeasurementUnitAgent;
use crate::agent::user::UserProfileAgent;
use crate::agent::weather::WeatherAgent;
use crate::llm::ToolClient;
use crate::proc::stack::FactsStack;
use crate::types::StringStream;
use crate::{agent::health::HealthAgent, llm::SlmRequest};
use futures::{StreamExt, stream};
use log::{debug, trace};
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub(crate) struct Action<'a> {
    agent: &'a str,
    prompt: &'a str,
}

impl<'a> Action<'a> {
    fn new(agent: &'a str, prompt: &'a str) -> Self {
        Self { agent, prompt }
    }
}

pub struct Interpreter<'a> {
    tool_client: &'a ToolClient,
}

impl <'a> Interpreter<'a> {
    pub fn new(tool_client: &'a ToolClient) -> Self {
        trace!("new() -> Self");
        Self { tool_client }
    }

    pub async fn eval(&mut self, text: &str) -> StringStream {
        trace!("eval(&mut self, text: &str) -> StringStream");
        let actions: Vec<Action> = text
            .lines()
            .map(|line| line.split_once(":"))
            .flatten()
            .map(|(agent, prompt)| Action::new(agent, prompt))
            .collect();

        let mut stack = String::new();

        let mut facts_stack = FactsStack::new();
        let mut stream_operations: Vec<StringStream> = Vec::new();

        for action in actions {
            let prompt = facts_stack.inject_variables(&action.prompt);
            debug!("prompt: {}", prompt);
            facts_stack.push_prompt(&prompt);
            // TODO: check if all prompt's variables are resolved

            let mut request = SlmRequest::new(&prompt);
            request.add_system(&facts_stack.context);
            request.set_use_history(false);
            debug!("request: {:?}", request);

            debug!("facts: {:?}", facts_stack.facts);
            let result: Option<String> = match action.agent {
                "time-service" => TimeServiceAgent::new().exec(&request).ok(),
                "user-profile" => UserProfileAgent::new(self.tool_client).exec(&request).await.ok(),
                "measure-units" => MeasurementUnitAgent::new().exec(&request).ok(),
                "health" => HealthAgent::new(self.tool_client).exec(&mut request).await.ok(),
                "weather" => WeatherAgent::new(self.tool_client).exec(&mut request).await.ok(),
                "home-automation" => HeraAgent::new(self.tool_client).exec(&mut request).await.ok(),
                _ => None,
            };
            if let Some(response) = result {
                debug!("response: {}", response);
                stack += &format!("{}\n", response);
                facts_stack.push_facts(&response);
            }

            let mut request = SlmRequest::new(&prompt);
            request.add_system(&facts_stack.context);
            request.set_use_history(false);
            let stream: Option<StringStream> = match action.agent {
                "query" => QueryAgent::new().exec(request).await.ok(),
                "printer" => PrinterAgent::new().exec(request).await.ok(),
                _ => None,
            };
            if let Some(stream) = stream {
                stream_operations.push(stream);
            }
        }

        // TODO: we may need to add to model history the initial prompt and question agent response, but not intermediate steps

        stream_operations.insert(0, facts_stack.get_reasoning_stream());
        if stream_operations.is_empty() {
            return Box::pin(stream::empty());
        }
        let all_streams = stream_operations
            .into_iter()
            .reduce(|acc, stream| Box::pin(acc.chain(stream)))
            .unwrap();
        all_streams
    }
}
