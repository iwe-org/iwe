use liwe::model::config::Model;
use reqwest::Method;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, env::var, time::Duration};

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

pub fn apply_prompt<'a>(prompt: String, model: &Model) -> String {
    let client = reqwest::blocking::Client::new();

    let token = var(model.api_key_env.clone())
        .expect(&format!("{} env var must be set", model.api_key_env));

    let request = ChatCompletionRequest {
        model: model.name.clone(),
        messages: vec![ChatCompletionMessage {
            role: Role::User,
            content: Some(prompt),
            name: Some("user".to_string()),
        }],
        temperature: model.temperature,
        top_p: Some(1.0),
        n: Some(1),
        stream: Some(false),
        stop: vec![], // can stop on a new line
        seed: None,
        max_tokens: model.max_tokens,
        presence_penalty: None,
        frequency_penalty: None,
        logit_bias: None,
        user: "user".to_string(),
        response_format: None,
        max_completion_tokens: model.max_completion_tokens,
    };

    let response = client
        .request(
            Method::POST,
            format!("{}/v1/chat/completions", model.base_url),
        )
        .timeout(Duration::from_secs(60))
        .json(&request)
        .bearer_auth(token)
        .send()
        .unwrap()
        .json::<ChatCompletion>();

    match response {
        Ok(ok) => ok.choices[0].message.content.as_ref().unwrap().clone(),
        Err(err) => {
            format!("Error: {:?}", err)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore]
    fn test_chat() {
        dbg!(apply_prompt(
            "test".to_string(),
            &Model {
                api_key_env: "OPENAI_API_KEY".to_string(),
                base_url: "https://api.openai.com".to_string(),
                name: "gpt-4o".to_string(),
                max_tokens: None,
                max_completion_tokens: None,
                temperature: None,
            }
        ));
    }
}
