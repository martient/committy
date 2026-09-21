use serde::Deserialize;
use std::time::Duration;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum LlmError {
    #[error("AI provider not configured")]
    NotConfigured,
    #[error("AI provider request failed: {0}")]
    RequestFailed(String),
    #[error("AI response parse error: {0}")]
    Parse(String),
}

#[async_trait::async_trait]
pub trait LlmClient: Send + Sync {
    async fn suggest_commit(
        &self,
        system_prompt: &str,
        user_prompt: &str,
        json_mode: bool,
        max_tokens: u32,
        temperature: f32,
        timeout_ms: u64,
    ) -> Result<String, LlmError>;
}

/// JSON Schema describing [`AiCommitSuggestion`].
///
/// Ollama constrains decoding to a supplied schema, which is stricter than
/// asking for "some JSON" and removes most parse failures.
fn suggestion_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "commit_type": {"type": "string"},
            "short": {"type": "string"},
            "scope": {"type": "string"},
            "long": {"type": "string"},
            "message": {"type": "string"},
        },
    })
}

fn chat_messages(system_prompt: &str, user_prompt: &str) -> serde_json::Value {
    serde_json::json!([
        {"role": "system", "content": system_prompt},
        {"role": "user", "content": user_prompt},
    ])
}

/// Build the OpenRouter `/chat/completions` request body.
fn openrouter_request_body(
    model: &str,
    system_prompt: &str,
    user_prompt: &str,
    json_mode: bool,
    max_tokens: u32,
    temperature: f32,
) -> serde_json::Value {
    let mut body = serde_json::json!({
        "model": model,
        "messages": chat_messages(system_prompt, user_prompt),
        "max_tokens": max_tokens,
        "temperature": temperature,
    });
    if json_mode {
        body["response_format"] = serde_json::json!({"type": "json_object"});
    }
    body
}

/// Build the Ollama `/api/chat` request body.
///
/// Two details are load-bearing and were wrong before: `stream` must be false
/// (the endpoint defaults to true and would return newline-delimited events
/// that cannot deserialize as one object), and `format` is a **top-level**
/// field, not an entry under `options`.
fn ollama_request_body(
    model: &str,
    system_prompt: &str,
    user_prompt: &str,
    json_mode: bool,
    max_tokens: u32,
    temperature: f32,
) -> serde_json::Value {
    let mut body = serde_json::json!({
        "model": model,
        "messages": chat_messages(system_prompt, user_prompt),
        "stream": false,
        "options": {
            "temperature": temperature,
            "num_predict": max_tokens as i32,
        },
    });
    if json_mode {
        body["format"] = suggestion_schema();
    }
    body
}

pub struct OpenRouterClient {
    pub base_url: String,
    pub api_key: Option<String>,
    pub model: String,
}

pub struct OllamaClient {
    pub base_url: String,
    pub model: String,
}

#[async_trait::async_trait]
impl LlmClient for OpenRouterClient {
    async fn suggest_commit(
        &self,
        system_prompt: &str,
        user_prompt: &str,
        json_mode: bool,
        max_tokens: u32,
        temperature: f32,
        timeout_ms: u64,
    ) -> Result<String, LlmError> {
        let api_key = self
            .api_key
            .as_ref()
            .ok_or_else(|| LlmError::NotConfigured)?;

        let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));

        let body = openrouter_request_body(
            &self.model,
            system_prompt,
            user_prompt,
            json_mode,
            max_tokens,
            temperature,
        );

        let client = reqwest::Client::builder()
            .timeout(Duration::from_millis(timeout_ms))
            .build()
            .map_err(|e| LlmError::RequestFailed(e.to_string()))?;

        let resp = client
            .post(&url)
            .bearer_auth(api_key)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| LlmError::RequestFailed(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(LlmError::RequestFailed(format!("status {}", resp.status())));
        }

        #[derive(Deserialize)]
        struct Choice {
            message: ChoiceMessage,
        }
        #[derive(Deserialize)]
        struct ChoiceMessage {
            content: String,
        }
        #[derive(Deserialize)]
        struct ResponseBody {
            choices: Vec<Choice>,
        }

        let rb: ResponseBody = resp
            .json()
            .await
            .map_err(|e| LlmError::Parse(e.to_string()))?;
        let content = rb
            .choices
            .first()
            .ok_or_else(|| LlmError::Parse("no choices".into()))?
            .message
            .content
            .clone();
        Ok(content)
    }
}

#[async_trait::async_trait]
impl LlmClient for OllamaClient {
    async fn suggest_commit(
        &self,
        system_prompt: &str,
        user_prompt: &str,
        json_mode: bool,
        max_tokens: u32,
        temperature: f32,
        timeout_ms: u64,
    ) -> Result<String, LlmError> {
        let url = format!("{}/api/chat", self.base_url.trim_end_matches('/'));

        let body = ollama_request_body(
            &self.model,
            system_prompt,
            user_prompt,
            json_mode,
            max_tokens,
            temperature,
        );

        let client = reqwest::Client::builder()
            .timeout(Duration::from_millis(timeout_ms))
            .build()
            .map_err(|e| LlmError::RequestFailed(e.to_string()))?;

        let resp = client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| LlmError::RequestFailed(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(LlmError::RequestFailed(format!("status {}", resp.status())));
        }

        #[derive(Deserialize)]
        struct ResponseMessage {
            content: String,
        }
        #[derive(Deserialize)]
        struct ResponseBody {
            message: ResponseMessage,
        }

        let rb: ResponseBody = resp
            .json()
            .await
            .map_err(|e| LlmError::Parse(e.to_string()))?;
        Ok(rb.message.content)
    }
}

#[derive(Debug, Deserialize)]
pub struct AiCommitSuggestion {
    pub commit_type: Option<String>,
    pub short: Option<String>,
    pub scope: Option<String>,
    pub long: Option<String>,
    pub message: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ollama_body_disables_streaming() {
        let body = ollama_request_body("llama3.2", "sys", "user", true, 256, 0.2);
        assert_eq!(
            body["stream"],
            serde_json::Value::Bool(false),
            "Ollama defaults /api/chat to stream:true; a streaming response is \
             newline-delimited and will not deserialize as one object"
        );
    }

    #[test]
    fn ollama_body_puts_format_at_the_top_level() {
        let body = ollama_request_body("llama3.2", "sys", "user", true, 256, 0.2);
        assert!(
            body["format"].is_object(),
            "Ollama reads `format` as a top-level field; nesting it under `options` \
             silently disables constrained decoding. Got: {body}"
        );
        assert!(
            body["options"]["format"].is_null(),
            "`format` must not also be left inside `options`"
        );
    }

    #[test]
    fn ollama_body_constrains_to_the_suggestion_schema() {
        let body = ollama_request_body("llama3.2", "sys", "user", true, 256, 0.2);
        let props = &body["format"]["properties"];
        for field in ["commit_type", "short", "scope", "long", "message"] {
            assert_eq!(
                props[field]["type"],
                serde_json::Value::String("string".into()),
                "schema must describe AiCommitSuggestion::{field}"
            );
        }
    }

    #[test]
    fn ollama_body_omits_format_when_json_mode_is_off() {
        let body = ollama_request_body("llama3.2", "sys", "user", false, 256, 0.2);
        assert!(body["format"].is_null());
        assert_eq!(body["stream"], serde_json::Value::Bool(false));
    }

    #[test]
    fn ollama_body_carries_sampling_options() {
        let body = ollama_request_body("llama3.2", "sys", "user", true, 512, 0.7);
        assert_eq!(body["options"]["num_predict"], serde_json::json!(512));
        assert_eq!(body["model"], serde_json::json!("llama3.2"));
        assert_eq!(body["messages"][0]["role"], serde_json::json!("system"));
        assert_eq!(body["messages"][1]["content"], serde_json::json!("user"));
    }

    #[test]
    fn openrouter_body_requests_json_object_when_asked() {
        let body = openrouter_request_body("m", "sys", "user", true, 256, 0.2);
        assert_eq!(
            body["response_format"]["type"],
            serde_json::json!("json_object")
        );
        let plain = openrouter_request_body("m", "sys", "user", false, 256, 0.2);
        assert!(plain["response_format"].is_null());
    }
}
