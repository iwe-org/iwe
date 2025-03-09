use reqwest::Method;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, env::var};

pub mod templates;

#[derive(Serialize, Debug, Clone)]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<ChatCompletionMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub stop: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_completion_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logit_bias: Option<HashMap<String, f32>>,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub user: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<ResponseFormat>,
}

#[derive(Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct ChatCompletion {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<Choce>,
}

#[derive(Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct Choce {
    pub index: u64,
    pub finish_reason: String,
    pub message: ChatCompletionMessage,
}

#[derive(Deserialize, Serialize, Debug, Clone, Eq, PartialEq, Default)]
pub struct ChatCompletionMessage {
    pub role: Role,
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone, Copy, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
}

#[derive(Serialize, Debug, Clone, Eq, PartialEq)]
pub struct ResponseFormat {
    #[serde(rename = "type")]
    format_type: String,
}

impl Default for Role {
    fn default() -> Self {
        Self::User
    }
}

pub fn apply_prompt<'a>(prompt: String) -> String {
    let client = reqwest::blocking::Client::new();

    let token = var("OPENAI_API_KEY").expect("OPENAI_API_KEY must be set");

    let request = ChatCompletionRequest {
        model: "gpt-3.5-turbo".to_string(),
        messages: vec![ChatCompletionMessage {
            role: Role::User,
            content: Some(prompt),
            name: Some("User".to_string()),
        }],
        temperature: Some(0.7),
        top_p: Some(1.0),
        n: Some(1),
        stream: Some(false),
        stop: vec!["\n".to_string()],
        seed: Some(0),
        max_tokens: Some(100),
        presence_penalty: Some(0.0),
        frequency_penalty: Some(0.0),
        logit_bias: None,
        user: "user-id-123".to_string(),
        response_format: None,
        max_completion_tokens: None,
    };

    let response = client
        .request(Method::POST, "https://api.openai.com/v1/chat/completions")
        .json(&request)
        .bearer_auth(token)
        .send()
        .unwrap()
        .json::<ChatCompletion>();

    response.ok().unwrap().choices[0]
        .message
        .content
        .as_ref()
        .unwrap()
        .clone()
}

#[cfg(test)]
mod tests {
    use reqwest::Method;

    use super::*;
    use std::env;

    #[test]
    #[ignore]
    fn test_chat() {
        let client = reqwest::blocking::Client::new();

        let token = env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY must be set");

        let request = ChatCompletionRequest {
            model: "gpt-3.5-turbo".to_string(),
            messages: vec![ChatCompletionMessage {
                role: Role::User,
                content: Some("Hello, world!".to_string()),
                name: Some("User".to_string()),
            }],
            temperature: Some(0.7),
            top_p: Some(1.0),
            n: Some(1),
            stream: Some(false),
            stop: vec!["\n".to_string()],
            seed: Some(0),
            max_tokens: Some(100),
            presence_penalty: Some(0.0),
            frequency_penalty: Some(0.0),
            logit_bias: None,
            user: "user-id-123".to_string(),
            response_format: None,
            max_completion_tokens: None,
        };

        let response = client
            .request(Method::POST, "https://api.openai.com/v1/chat/completions")
            .json(&request)
            .bearer_auth(token)
            .send()
            .unwrap()
            .json::<ChatCompletion>();

        println!("{:?}", response);
    }
}
