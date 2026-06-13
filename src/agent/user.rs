use crate::llm::ToolClient;
use crate::service::Property;
use crate::service::user::{
    GetProperty, ListProperties, RemoveProperty, RenameProperty, SetProperty, UpdateProperty,
};
use crate::types::Result;
use log::{debug, trace};
use serde::Deserialize;

pub struct UserProfileAgent<'a> {
    tool_client: &'a ToolClient,
}

impl<'a> UserProfileAgent<'a> {
    pub fn new(tool_client: &'a ToolClient) -> Self {
        Self { tool_client }
    }

    pub async fn exec(&self, prompt: &str) -> Result<String> {
        trace!("UserProfileAgent::exec(&self, prompt: &str) -> Result<String>");
        debug!("prompt: {}", prompt);

        if prompt == "Get my username." {
            return Ok(Property::value("username", &self.username()));
        }

        let mut function: Function = self.tool_client.get_function(prompt, "user").await?;
        function.exec().await
    }

    fn username(&self) -> String {
        String::from("Rotaru Iulian")
    }
}

#[derive(Deserialize)]
#[serde(tag = "function")]
enum Function {
    #[serde(rename = "set_property")]
    SetProperty(SetProperty),
    #[serde(rename = "update_property")]
    UpdateProperty(UpdateProperty),
    #[serde(rename = "rename_property")]
    RenameProperty(RenameProperty),
    #[serde(rename = "remove_property")]
    RemoveProperty(RemoveProperty),
    #[serde(rename = "get_property")]
    GetProperty(GetProperty),
    #[serde(rename = "list_properties")]
    ListProperties(ListProperties),
}

impl Function {
    async fn exec(&mut self) -> Result<String> {
        match self {
            Function::SetProperty(call) => call.exec().await,
            Function::UpdateProperty(call) => call.exec().await,
            Function::RenameProperty(call) => call.exec().await,
            Function::RemoveProperty(call) => call.exec().await,
            Function::GetProperty(call) => call.exec().await,
            Function::ListProperties(call) => call.exec().await,
        }
    }
}
