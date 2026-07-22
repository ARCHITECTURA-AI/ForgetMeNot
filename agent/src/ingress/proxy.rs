use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChatChoice {
    pub index: u32,
    pub message: ChatMessage,
    pub finish_reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChatCompletionResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<ChatChoice>,
}

pub trait UpstreamForwarder {
    fn forward(&self, request: &ChatCompletionRequest) -> anyhow::Result<ChatCompletionResponse>;
}

pub struct MockUpstreamForwarder {
    canned_response: String,
}

impl MockUpstreamForwarder {
    #[must_use]
    pub fn new(canned_response: impl Into<String>) -> Self {
        Self {
            canned_response: canned_response.into(),
        }
    }
}

impl UpstreamForwarder for MockUpstreamForwarder {
    fn forward(&self, request: &ChatCompletionRequest) -> anyhow::Result<ChatCompletionResponse> {
        if request.messages.is_empty() {
            return Err(anyhow::anyhow!("Request messages cannot be empty"));
        }

        Ok(ChatCompletionResponse {
            id: "chatcmpl-mock-12345".to_string(),
            object: "chat.completion".to_string(),
            created: 1718000000,
            model: request.model.clone(),
            choices: vec![ChatChoice {
                index: 0,
                message: ChatMessage {
                    role: "assistant".to_string(),
                    content: self.canned_response.clone(),
                },
                finish_reason: "stop".to_string(),
            }],
        })
    }
}

pub fn forward_chat_completion<F: UpstreamForwarder>(
    forwarder: &F,
    request: ChatCompletionRequest,
) -> anyhow::Result<ChatCompletionResponse> {
    forwarder.forward(&request)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compatible_request_is_forwarded() {
        let forwarder = MockUpstreamForwarder::new("Paris");

        let request = ChatCompletionRequest {
            model: "gpt-4o".to_string(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: "What is the capital of France?".to_string(),
            }],
        };

        let result = forward_chat_completion(&forwarder, request);

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.model, "gpt-4o");
        assert_eq!(response.object, "chat.completion");
        assert_eq!(response.choices.len(), 1);
        assert_eq!(response.choices[0].message.role, "assistant");
        assert_eq!(response.choices[0].message.content, "Paris");
    }

    #[test]
    fn test_empty_messages_returns_error() {
        let forwarder = MockUpstreamForwarder::new("Paris");
        let request = ChatCompletionRequest {
            model: "gpt-4o".to_string(),
            messages: vec![],
        };

        let result = forward_chat_completion(&forwarder, request);
        assert!(result.is_err());
    }
}
