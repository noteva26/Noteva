//! AI writing assistant endpoints.

use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::api::middleware::{ApiError, AppState, AuthenticatedUser};

const DEFAULT_SYSTEM_PROMPT: &str =
    "You are a concise blog writing assistant. Return only the requested result, without explanations.";
const DEFAULT_TITLE_PROMPT: &str =
    "Generate one clear blog post title for this Markdown content:\n\n{{content}}";
const DEFAULT_SLUG_PROMPT: &str =
    "Generate one lowercase URL slug using only letters, numbers and hyphens. Title: {{title}}\nCurrent slug: {{slug}}";
const DEFAULT_SUMMARY_PROMPT: &str =
    "Write a concise article summary in the same language as the article. Keep it under 120 Chinese characters or 80 English words.\n\nTitle: {{title}}\n\nContent:\n{{content}}";
const DEFAULT_FORMAT_MARKDOWN_PROMPT: &str =
    "Clean up only Markdown formatting and structure. Do not change, rewrite, add, remove, or translate any wording. Return the full Markdown only.\n\n{{content}}";
const DEFAULT_IMPROVE_WRITING_PROMPT: &str =
    "Improve the expression and readability of this article while preserving meaning and Markdown structure. Return the full Markdown only.\n\nTitle: {{title}}\nSummary: {{summary}}\n\nContent:\n{{content}}";

#[derive(Debug, Deserialize)]
pub struct AiAssistRequest {
    pub task: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub slug: Option<String>,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub content: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AiAssistResponse {
    pub result: String,
}

struct AiConfig {
    provider: String,
    api_key: String,
    api_base: String,
    model: String,
    system_prompt: String,
}

pub async fn assist(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
    Json(req): Json<AiAssistRequest>,
) -> Result<Json<AiAssistResponse>, ApiError> {
    let config = read_ai_config(&state).await?;
    let prompt = build_prompt(&state, &req).await?;
    let client = reqwest::Client::new();

    let result = match config.provider.as_str() {
        "openai_responses" => call_openai_responses(&client, &config, &prompt).await?,
        "gemini" => call_gemini(&client, &config, &prompt).await?,
        "claude" => call_claude(&client, &config, &prompt).await?,
        _ => call_openai_chat(&client, &config, &prompt).await?,
    };

    Ok(Json(AiAssistResponse { result }))
}

async fn read_ai_config(state: &AppState) -> Result<AiConfig, ApiError> {
    let api_key = read_setting(state, "ai_api_key").await.unwrap_or_default();
    if api_key.trim().is_empty() {
        return Err(ApiError::validation_error("AI API key is not configured"));
    }

    let provider = read_setting(state, "ai_provider")
        .await
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "openai_chat".to_string());
    let model = read_setting(state, "ai_model")
        .await
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| default_model(&provider).to_string());
    let api_base = read_setting(state, "ai_api_base")
        .await
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| default_api_base(&provider).to_string());
    let system_prompt = read_setting(state, "ai_system_prompt")
        .await
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_SYSTEM_PROMPT.to_string());

    Ok(AiConfig {
        provider,
        api_key,
        api_base,
        model,
        system_prompt,
    })
}

async fn build_prompt(state: &AppState, req: &AiAssistRequest) -> Result<String, ApiError> {
    let template_key = match req.task.as_str() {
        "title" => ("ai_prompt_title", DEFAULT_TITLE_PROMPT),
        "slug" => ("ai_prompt_slug", DEFAULT_SLUG_PROMPT),
        "summary" => ("ai_prompt_summary", DEFAULT_SUMMARY_PROMPT),
        "format_markdown" => ("ai_prompt_format_markdown", DEFAULT_FORMAT_MARKDOWN_PROMPT),
        "improve_writing" => ("ai_prompt_improve_writing", DEFAULT_IMPROVE_WRITING_PROMPT),
        _ => return Err(ApiError::validation_error("Unsupported AI task")),
    };

    let template = read_setting(state, template_key.0)
        .await
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| template_key.1.to_string());

    Ok(render_prompt_template(&template, req))
}

fn render_prompt_template(template: &str, req: &AiAssistRequest) -> String {
    template
        .replace("{{title}}", req.title.as_deref().unwrap_or("").trim())
        .replace("{{slug}}", req.slug.as_deref().unwrap_or("").trim())
        .replace("{{summary}}", req.summary.as_deref().unwrap_or("").trim())
        .replace("{{content}}", req.content.as_deref().unwrap_or("").trim())
}

async fn call_openai_chat(
    client: &reqwest::Client,
    config: &AiConfig,
    prompt: &str,
) -> Result<String, ApiError> {
    let endpoint = format!("{}/chat/completions", config.api_base.trim_end_matches('/'));
    let payload = json!({
        "model": config.model,
        "messages": [
            { "role": "system", "content": config.system_prompt },
            { "role": "user", "content": prompt }
        ],
        "temperature": 0.4
    });

    let body = send_json_request(
        client
            .post(endpoint)
            .bearer_auth(config.api_key.trim())
            .json(&payload),
    )
    .await?;

    read_path_text(&body, &["choices", "0", "message", "content"])
}

async fn call_openai_responses(
    client: &reqwest::Client,
    config: &AiConfig,
    prompt: &str,
) -> Result<String, ApiError> {
    let endpoint = format!("{}/responses", config.api_base.trim_end_matches('/'));
    let payload = json!({
        "model": config.model,
        "instructions": config.system_prompt,
        "input": prompt,
        "store": false
    });

    let body = send_json_request(
        client
            .post(endpoint)
            .bearer_auth(config.api_key.trim())
            .json(&payload),
    )
    .await?;

    read_path_text(&body, &["output_text"])
        .or_else(|_| read_path_text(&body, &["output", "0", "content", "0", "text"]))
}

async fn call_gemini(
    client: &reqwest::Client,
    config: &AiConfig,
    prompt: &str,
) -> Result<String, ApiError> {
    let endpoint = format!(
        "{}/models/{}:generateContent?key={}",
        config.api_base.trim_end_matches('/'),
        config.model,
        config.api_key.trim()
    );
    let payload = json!({
        "contents": [{
            "role": "user",
            "parts": [{ "text": format!("{}\n\n{}", config.system_prompt, prompt) }]
        }]
    });

    let body = send_json_request(client.post(endpoint).json(&payload)).await?;
    read_path_text(&body, &["candidates", "0", "content", "parts", "0", "text"])
}

async fn call_claude(
    client: &reqwest::Client,
    config: &AiConfig,
    prompt: &str,
) -> Result<String, ApiError> {
    let endpoint = format!("{}/messages", config.api_base.trim_end_matches('/'));
    let payload = json!({
        "model": config.model,
        "max_tokens": 4096,
        "system": config.system_prompt,
        "messages": [{ "role": "user", "content": prompt }]
    });

    let body = send_json_request(
        client
            .post(endpoint)
            .header("x-api-key", config.api_key.trim())
            .header("anthropic-version", "2023-06-01")
            .json(&payload),
    )
    .await?;

    read_path_text(&body, &["content", "0", "text"])
}

async fn send_json_request(builder: reqwest::RequestBuilder) -> Result<Value, ApiError> {
    let response = builder
        .send()
        .await
        .map_err(|e| ApiError::internal_error(format!("AI request failed: {}", e)))?;

    if !response.status().is_success() {
        return Err(ApiError::internal_error(format!(
            "AI provider returned {}",
            response.status()
        )));
    }

    response
        .json::<Value>()
        .await
        .map_err(|e| ApiError::internal_error(format!("Invalid AI response: {}", e)))
}

fn read_path_text(value: &Value, path: &[&str]) -> Result<String, ApiError> {
    let mut current = value;
    for segment in path {
        current = if let Ok(index) = segment.parse::<usize>() {
            current.get(index)
        } else {
            current.get(*segment)
        }
        .ok_or_else(|| ApiError::internal_error("AI response is empty"))?;
    }

    current
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| ApiError::internal_error("AI response is empty"))
}

fn default_api_base(provider: &str) -> &'static str {
    match provider {
        "gemini" => "https://generativelanguage.googleapis.com/v1beta",
        "claude" => "https://api.anthropic.com/v1",
        _ => "https://api.openai.com/v1",
    }
}

fn default_model(provider: &str) -> &'static str {
    match provider {
        "gemini" => "gemini-1.5-flash",
        "claude" => "claude-3-5-sonnet-latest",
        _ => "gpt-4o-mini",
    }
}

async fn read_setting(state: &AppState, key: &str) -> Option<String> {
    state.settings_service.get(key).await.ok().flatten()
}
