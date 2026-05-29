mod router;
mod slm;
mod tool;
mod types;

pub(crate) use router::RouterClient;

pub(crate) use tool::ToolClient;

pub(crate) use types::LlmChunk;
pub(crate) use types::LlmModel;
pub(crate) use types::LlmModelData;
pub(crate) use types::LlmModels;
pub(crate) use types::LlmRequest;

pub(crate) use slm::SlmClient;
pub(crate) use slm::SlmRequest;

pub(crate) struct RouterResponse {
    pub(crate) text: String,
    pub(crate) confidence: f32,
}
