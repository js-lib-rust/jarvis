use crate::config::Config;
use crate::llm::ToolClient;
use crate::proc::Action;
use crate::service::Property;
use crate::service::user_profile::{
    GetProperty, ListProperties, RemoveProperty, RenameProperty, SetProperty, UpdateProperty,
};
use crate::types::Result;
use log::trace;
use serde::Deserialize;

pub struct UserProfileAgent<'a> {
    tool_client: &'a ToolClient,
}

impl<'a> UserProfileAgent<'a> {
    pub fn new(tool_client: &'a ToolClient) -> Self {
        Self { tool_client }
    }

    pub async fn exec(&self, action: &Action<'a>) -> Result<String> {
        trace!("UserProfileAgent::exec(&self, action: &Action<'a>) -> Result<String>");
        let mut function: Function = self
            .tool_client
            .get_function(&action.prompt, action.agent)
            .await?;
        function.exec().await
    }
}

#[derive(Deserialize)]
pub struct GetUsername {
    user_id: Option<String>,
}

impl GetUsername {
    pub async fn exec(&self) -> Result<String> {
        trace!("GetUsername::exec(&self) -> Result<String>");
        let user_id = match &self.user_id {
            Some(user_id) => user_id,
            None => &Config::get().username,
        };
        Ok(Property::value("username", user_id))
    }
}

#[derive(Deserialize)]
#[serde(tag = "function")]
enum Function {
    #[serde(rename = "get_username")]
    GetUsername(GetUsername),
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
            Function::GetUsername(call) => call.exec().await,
            Function::SetProperty(call) => call.exec().await,
            Function::UpdateProperty(call) => call.exec().await,
            Function::RenameProperty(call) => call.exec().await,
            Function::RemoveProperty(call) => call.exec().await,
            Function::GetProperty(call) => call.exec().await,
            Function::ListProperties(call) => call.exec().await,
        }
    }
}
