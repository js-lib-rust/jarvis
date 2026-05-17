use crate::agent::context::{MeasurementUnitAgent, TimeServiceAgent, UserProfileAgent};
use crate::agent::hera::HeraAgent;
use crate::agent::printer::PrinterAgent;
use crate::agent::query::QueryAgent;
use crate::agent::weather::WeatherAgent;
use crate::proc::stack::FactsStack;
use crate::types::StringStream;
use crate::{agent::health::HealthAgent, slm::SlmRequest};
use futures::{StreamExt, stream};
use log::{debug, trace};
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub(crate) struct Action {
    agent: String,
    prompt: String,
}

impl Action {
    fn new(agent: &str, prompt: &str) -> Self {
        Self {
            agent: agent.to_string(),
            prompt: prompt.to_string(),
        }
    }
}

pub struct Interpreter {
    actions: Vec<Action>,
}

impl Interpreter {
    pub fn new(text: &str) -> Self {
        trace!("new(text: &str) -> Self");
        let actions: Vec<Action> = text
            .lines()
            .map(|line| line.split_once(":"))
            .flatten()
            .map(|(agent, prompt)| Action::new(agent, prompt))
            .collect();
        Self { actions }
    }

    pub async fn eval(&mut self) -> StringStream {
        trace!("eval(&mut self) -> StringStream");
        let mut stack = String::new();

        let mut facts_stack = FactsStack::new();
        let mut stream_operations: Vec<StringStream> = Vec::new();

        for action in &self.actions {
            let prompt = facts_stack.inject_variables(&action.prompt);
            debug!("prompt: {}", prompt);
            facts_stack.push_prompt(&prompt);
            // TODO: check if all prompt's variables are resolved

            let mut request = SlmRequest::new(&prompt);
            request.add_system(&facts_stack.context);
            request.set_use_history(false);
            debug!("request: {:?}", request);

            debug!("facts: {:?}", facts_stack.facts);
            let result: Option<String> = match action.agent.as_str() {
                "time-service" => TimeServiceAgent::new().execs(request).ok(),
                "user-profile" => UserProfileAgent::new().execs(request).await.ok(),
                "measure-units" => MeasurementUnitAgent::new().execs(request).ok(),
                "health" => HealthAgent::new().execs(request).await.ok(),
                "weather" => WeatherAgent::new().execs(request).await.ok(),
                "home-automation" => HeraAgent::new().execs(request).await.ok(),
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
            let stream: Option<StringStream> = match action.agent.as_str() {
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
