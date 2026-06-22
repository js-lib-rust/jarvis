use chrono::Utc;
use log::trace;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// LLM chat request for /v1/chat/completions endpoint from OpenAI API

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct LlmMessage {
    role: String,
    content: String,
}

impl LlmMessage {
    pub(crate) fn new(role: &str, content: &str) -> Self {
        Self {
            role: role.to_string(),
            content: content.to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct LlmRequest {
    model: Option<String>,
    messages: Vec<LlmMessage>,
    stream: Option<bool>,
    temperature: Option<f32>,
}

impl LlmRequest {
    const MAX_PROMPT: usize = 400;

    pub(crate) fn from_messages(system: &str, user: &str) -> Self {
        let messages = vec![
            LlmMessage::new("system", system),
            LlmMessage::new("user", user),
        ];
        Self {
            model: None,
            messages: messages,
            stream: Some(true),
            temperature: Some(0.0),
        }
    }

    pub(crate) fn get_routable_prompt(&self) -> Option<&str> {
        trace!("get_prompt(&self) -> Option<&str>");
        self.messages
            .iter()
            .rev()
            .find(|m| m.role == "user")
            .map(|m| m.content.as_str())
            .filter(|&prompt| prompt.len() < Self::MAX_PROMPT)
    }

    pub(crate) fn is_stream(&self) -> bool {
        self.stream.unwrap_or(false)
    }
}

// LLM chat response chunk for /v1/chat/completions endpoint from OpenAI API

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct LlmDelta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct LlmChoice {
    finish_reason: Option<String>,
    index: u32,
    delta: LlmDelta,
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct LlmChunk {
    id: Uuid,
    object: String,
    created: u64,
    model: String,
    system_fingerprint: Option<String>,
    choices: Vec<LlmChoice>,
}

// r#"{"choices":[{"finish_reason":null,"index":0,"delta":{"reasoning_content":"text chunk"}}],"created":1778613613,"id":"chatcmpl-MSTqvWZpYTXOWj8AU9CGBMzZCgjFnoQZ","model":"gemma-4-26B-A4B-Q5_K_M.gguf","system_fingerprint":"b8961-f42e29fdf","object":"chat.completion.chunk"}"#;
impl LlmChunk {
    pub(crate) fn from_reasoning(reasoning: &str) -> Self {
        LlmChunk::new(LlmDelta {
            content: None,
            reasoning_content: Some(reasoning.to_string()),
        })
    }

    fn new(delta: LlmDelta) -> Self {
        let choice = LlmChoice {
            finish_reason: None,
            index: 0,
            delta: delta,
        };
        LlmChunk {
            id: Uuid::new_v4(),
            object: "chat.completion.chunk".to_string(),
            created: Utc::now().timestamp() as u64,
            model: "gemma-4-26B-A4B-Q5_K_M.gguf".to_string(),
            system_fingerprint: None,
            choices: vec![choice],
        }
    }
}

// LLM models response for /v1/models endpoint from OpenAI API

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct LlmModel {
    name: String,
    model: String,
}

impl LlmModel {
    pub(crate) fn new(model: &str) -> Self {
        Self {
            name: model.to_string(),
            model: model.to_string(),
        }
    }
}
#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct LlmModelData {
    id: String,
    object: String,
}

impl LlmModelData {
    pub(crate) fn new(id: &str) -> Self {
        Self {
            id: id.to_string(),
            object: "model".to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct LlmModels {
    models: Vec<LlmModel>,
    data: Vec<LlmModelData>,
    object: String,
}

impl LlmModels {
    pub(crate) fn new(models: Vec<LlmModel>, data: Vec<LlmModelData>) -> Self {
        Self {
            models,
            data,
            object: "list".to_string(),
        }
    }
}

#[derive(Serialize, Debug)]
pub(crate) struct RouterResponse {
    pub(crate) estimated_confidence: f32,
    pub(crate) confidence: f32,
    pub(crate) text: String,
    pub(crate) processing_time: f32,
}
