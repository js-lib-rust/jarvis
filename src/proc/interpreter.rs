use crate::agent::home_automation::HomeAutomationAgent;
use crate::agent::printer::PrinterAgent;
use crate::agent::query::QueryAgent;
use crate::agent::time_service::TimeServiceAgent;
use crate::agent::measure_units::MeasureUnitAgent;
use crate::agent::user_profile::UserProfileAgent;
use crate::agent::weather::WeatherAgent;
use crate::llm::ToolClient;
use crate::proc::Action;
use crate::proc::stack::FactsStack;
use crate::types::StringStream;
use crate::{agent::health::HealthAgent, proc::app::AppManager};
use futures::StreamExt;
use log::{debug, trace};

pub struct Interpreter<'a> {
    tool_client: &'a ToolClient,
}

impl<'a> Interpreter<'a> {
    pub fn new(tool_client: &'a ToolClient) -> Self {
        trace!("new() -> Self");
        Self { tool_client }
    }

    pub async fn eval(&mut self, text: &str) -> Option<StringStream> {
        trace!("eval(&mut self, text: &str) -> Option<StringStream>");
        let actions: Vec<Action> = text
            .lines()
            .map(|line| line.split_once(":"))
            .flatten()
            .map(|(agent, prompt)| Action::new(agent, prompt))
            .collect();

        let mut facts_stack = FactsStack::new();
        let mut stream_operations: Vec<StringStream> = Vec::new();

        for mut action in actions {
            action.inject_variables(&facts_stack.facts);
            debug!("prompt: {}", action.prompt);
            if action.agent == "default" {
                return None;
            }
            facts_stack.push_prompt(&action.prompt);
            // TODO: check if all prompt's variables are resolved
            debug!("facts: {:?}", facts_stack.facts);

            // let result: Option<String> = match action.agent {
            //     "app-manager" => AppManager::new(self.tool_client).exec(prompt).await.ok(),
            //     _ => None,
            // };
            // println!("system result: {:?}", result);

            let result: Option<String> = match action.agent {
                "app-manager" => AppManager::new(self.tool_client).exec(&action).await.ok(),
                "time-service" => TimeServiceAgent::new().exec(&action).ok(),
                "user-profile" => UserProfileAgent::new(self.tool_client)
                    .exec(&action)
                    .await
                    .ok(),
                "measure-units" => MeasureUnitAgent::new().exec(&action).ok(),
                "health" => HealthAgent::new(self.tool_client).exec(&action).await.ok(),
                "weather" => WeatherAgent::new(self.tool_client).exec(&action).await.ok(),
                "home-automation" => HomeAutomationAgent::new(self.tool_client).exec(&action).await.ok(),
                _ => None,
            };
            if let Some(response) = result {
                debug!("response: {}", response);
                facts_stack.push_facts(&response);
            }

            let system = facts_stack.context.as_str();
            let prompt = &action.prompt;
            let stream: Option<StringStream> = match action.agent {
                "query" => QueryAgent::new().exec(system, prompt).await.ok(),
                "printer" => PrinterAgent::new().exec(system, prompt).await.ok(),
                _ => None,
            };
            if let Some(stream) = stream {
                stream_operations.push(stream);
            }
        }

        // TODO: we may need to add to model history the initial prompt and question agent response, but not intermediate steps

        stream_operations.insert(0, facts_stack.get_reasoning_stream());
        let all_streams = stream_operations
            .into_iter()
            .reduce(|acc, stream| Box::pin(acc.chain(stream)))
            .unwrap();
        Some(all_streams)
    }
}
