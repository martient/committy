use serde::{Deserialize, Serialize};
use thiserror::Error;
use std::time::Duration;

#[derive(Debug, Error)]
pub enum LlmError {
    #[error("AI provider not configured")]
    NotConfigured,
    #[error("AI provider request failed: {0}")]
    RequestFailed(String),
    #[error("AI provider timeout")]
    Timeout,
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LlmProvider {
    OpenRouter,
    Ollama,
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

        #[derive(Serialize)]
        struct Message<'a> { role: &'a str, content: &'a str }
        #[derive(Serialize)]
        struct RequestBody<'a> {
            model: &'a str,
            messages: Vec<Message<'a>>,
            max_tokens: u32,
            temperature: f32,
            #[serde(skip_serializing_if = "Option::is_none")]
            response_format: Option<serde_json::Value>,
        }
        let response_format = if json_mode {
            Some(serde_json::json!({"type": "json_object"}))
        } else { None };

        let body = RequestBody {
            model: &self.model,
            messages: vec![
                Message { role: "system", content: system_prompt },
                Message { role: "user", content: user_prompt },
            ],
            max_tokens,
            temperature,
            response_format,
        };

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
            return Err(LlmError::RequestFailed(format!(
                "status {}", resp.status()
            )));
        }

        #[derive(Deserialize)]
        struct Choice { message: ChoiceMessage }
        #[derive(Deserialize)]
        struct ChoiceMessage { content: String }
        #[derive(Deserialize)]
        struct ResponseBody { choices: Vec<Choice> }

        let rb: ResponseBody = resp
            .json()
            .await
            .map_err(|e| LlmError::Parse(e.to_string()))?;
        let content = rb
            .choices
            .get(0)
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

        #[derive(Serialize)]
        struct Message<'a> { role: &'a str, content: &'a str }
        #[derive(Serialize)]
        struct RequestBody<'a> {
            model: &'a str,
            messages: Vec<Message<'a>>,
            options: OllamaOptions,
        }
        #[derive(Serialize)]
        struct OllamaOptions {
            temperature: f32,
            num_predict: i32,
            #[serde(skip_serializing_if = "Option::is_none")]
            format: Option<&'static str>,
        }

        let body = RequestBody {
            model: &self.model,
            messages: vec![
                Message { role: "system", content: system_prompt },
                Message { role: "user", content: user_prompt },
            ],
            options: OllamaOptions { temperature, num_predict: max_tokens as i32, format: if json_mode { Some("json") } else { None } },
        };

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
            return Err(LlmError::RequestFailed(format!(
                "status {}", resp.status()
            )));
        }

        #[derive(Deserialize)]
        struct ResponseMessage { content: String }
        #[derive(Deserialize)]
        struct ResponseBody { message: ResponseMessage }

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

