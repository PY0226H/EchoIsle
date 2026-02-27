use crate::{AppError, AppState};
use chat_core::{AdapterType, AgentArgs, AgentType, ChatAgent};
use serde::{Deserialize, Serialize};
use tracing::info;
use utoipa::ToSchema;

#[derive(Debug, Clone, Default, ToSchema, Serialize, Deserialize)]
#[serde(rename_all(serialize = "camelCase"))]
pub struct CreateAgent {
    pub name: String,
    pub r#type: AgentType,
    pub adapter: AdapterType,
    pub model: String,
    pub prompt: String,
    #[serde(default = "default_map")]
    pub args: serde_json::Value,
}

fn default_map() -> serde_json::Value {
    serde_json::Value::Object(serde_json::Map::new())
}

fn is_supported_model(adapter: &AdapterType, model: &str) -> bool {
    let model = model.trim();
    if model.is_empty() {
        return false;
    }

    match adapter {
        // OpenAI model families:
        // - gpt-* (e.g. gpt-4o)
        // - o1/o3/o4 reasoning families
        AdapterType::Openai => {
            let model = model.to_ascii_lowercase();
            model.starts_with("gpt-")
                || model.starts_with("o1")
                || model.starts_with("o3")
                || model.starts_with("o4")
        }
        // Ollama model availability is deployment-dependent, validate non-empty only.
        AdapterType::Ollama => true,
        // Test adapter ignores model at runtime, validate non-empty only.
        AdapterType::Test => true,
    }
}

#[derive(Debug, Clone, Default, ToSchema, Serialize, Deserialize)]
#[serde(rename_all(serialize = "camelCase"))]
pub struct UpdateAgent {
    pub id: u64,
    #[serde(default)]
    pub prompt: String,
    #[serde(default)]
    pub args: serde_json::Value,
}

impl CreateAgent {
    pub fn new(
        name: impl Into<String>,
        r#type: AgentType,
        adapter: AdapterType,
        model: impl Into<String>,
        prompt: impl Into<String>,
        args: impl Serialize,
    ) -> Self {
        Self {
            name: name.into(),
            r#type,
            adapter,
            model: model.into(),
            prompt: prompt.into(),
            args: serde_json::to_value(args).unwrap(),
        }
    }
}

impl UpdateAgent {
    pub fn new(id: u64, prompt: impl Into<String>, args: impl Serialize) -> Self {
        Self {
            id,
            prompt: prompt.into(),
            args: serde_json::to_value(args).unwrap(),
        }
    }
}

#[allow(dead_code)]
impl AppState {
    /// Create a new agent in a chat
    pub async fn create_agent(
        &self,
        input: CreateAgent,
        chat_id: u64,
    ) -> Result<ChatAgent, AppError> {
        // check if agent exists
        if self.agent_name_exists(chat_id, &input.name).await? {
            info!("Agent {} already exists in chat {chat_id}", input.name);
            return Err(AppError::CreateAgentError(format!(
                "Agent {} already exists",
                input.name
            )));
        }

        if !is_supported_model(&input.adapter, &input.model) {
            info!(
                "Unsupported model '{}' for adapter {:?} in chat {chat_id}",
                input.model, input.adapter
            );
            return Err(AppError::CreateAgentError(format!(
                "model '{}' is not supported by adapter {:?}",
                input.model, input.adapter
            )));
        }

        let agent = sqlx::query_as(
            r#"
            INSERT INTO chat_agents (chat_id, name, type, adapter, model, prompt, args)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING *
            "#,
        )
        .bind(chat_id as i64)
        .bind(input.name)
        .bind(input.r#type)
        .bind(input.adapter)
        .bind(input.model)
        .bind(input.prompt)
        .bind(sqlx::types::Json(AgentArgs::from(input.args)))
        .fetch_one(&self.pool)
        .await?;

        Ok(agent)
    }

    /// check if an agent name exists in a chat
    pub async fn agent_name_exists(&self, chat_id: u64, name: &str) -> Result<bool, AppError> {
        let exists = sqlx::query_scalar(
            r#"
            SELECT EXISTS (SELECT 1 FROM chat_agents WHERE chat_id = $1 AND name = $2)
            "#,
        )
        .bind(chat_id as i64)
        .bind(name)
        .fetch_one(&self.pool)
        .await?;

        Ok(exists)
    }

    /// check if an agent id exists in a chat
    pub async fn agent_id_exists(&self, chat_id: u64, agent_id: u64) -> Result<bool, AppError> {
        let exists = sqlx::query_scalar(
            r#"
            SELECT EXISTS (SELECT 1 FROM chat_agents WHERE chat_id = $1 AND id = $2)
            "#,
        )
        .bind(chat_id as i64)
        .bind(agent_id as i64)
        .fetch_one(&self.pool)
        .await?;

        Ok(exists)
    }

    /// List all agents in a chat
    pub async fn list_agents(&self, chat_id: u64) -> Result<Vec<ChatAgent>, AppError> {
        let agents = sqlx::query_as(
            r#"
            SELECT * FROM chat_agents WHERE chat_id = $1 ORDER BY id ASC
            "#,
        )
        .bind(chat_id as i64)
        .fetch_all(&self.pool)
        .await?;

        Ok(agents)
    }

    /// update an agent in a chat
    pub async fn update_agent(
        &self,
        input: UpdateAgent,
        chat_id: u64,
    ) -> Result<ChatAgent, AppError> {
        let agent_id = input.id;
        let prompt = input.prompt;
        let args = input.args;

        // check if agent exists
        if !self.agent_id_exists(chat_id, agent_id).await? {
            info!("Agent {agent_id} does not exist in chat {chat_id}");
            return Err(AppError::UpdateAgentError(format!(
                "Agent {} does not exist",
                agent_id
            )));
        }

        let agent = match (prompt.as_str(), &args) {
            ("", _) => {
                sqlx::query_as(
                    r#"
                    UPDATE chat_agents SET args = $1 WHERE chat_id = $2 AND id = $3 RETURNING *
                    "#,
                )
                .bind(sqlx::types::Json(AgentArgs::from(args)))
                .bind(chat_id as i64)
                .bind(agent_id as i64)
                .fetch_one(&self.pool)
                .await?
            }
            (_, _) => {
                sqlx::query_as(
                    r#"
                    UPDATE chat_agents SET prompt = $1, args = $2 WHERE chat_id = $3 AND id = $4 RETURNING *
                    "#,
                )
                .bind(prompt)
                .bind(sqlx::types::Json(AgentArgs::from(args)))
                .bind(chat_id as i64)
                .bind(agent_id as i64)
                .fetch_one(&self.pool)
                .await?
            }
        };

        Ok(agent)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use std::collections::HashMap;

    #[tokio::test]
    async fn create_agent_should_work() -> Result<()> {
        let (_tdb, state) = AppState::new_for_test().await?;
        let input = CreateAgent::new(
            "test",
            AgentType::Proxy,
            AdapterType::Ollama,
            "llama3.2",
            "You are a helpful assistant",
            HashMap::<String, String>::new(),
        );
        let agent = state
            .create_agent(input, 1)
            .await
            .expect("create chat failed");

        assert_eq!(agent.name, "test");
        assert_eq!(agent.r#type, AgentType::Proxy);
        assert_eq!(agent.adapter, AdapterType::Ollama);
        assert_eq!(agent.model, "llama3.2");
        assert_eq!(agent.prompt, "You are a helpful assistant");
        assert_eq!(agent.args, sqlx::types::Json(AgentArgs::empty()));
        Ok(())
    }

    #[tokio::test]
    async fn list_agents_should_work() -> Result<()> {
        let (_tdb, state) = AppState::new_for_test().await?;
        let agents = state.list_agents(1).await.expect("list agents failed");
        assert_eq!(agents.len(), 1);
        assert_eq!(agents[0].name, "translation");
        assert_eq!(agents[0].r#type, AgentType::Proxy);
        assert_eq!(agents[0].prompt, "If language is Chinese, translate to English, if language is English, translate to Chinese. Please reply with the translated content directly. No explanation is needed. Here is the content: ");
        assert_eq!(agents[0].args, sqlx::types::Json(AgentArgs::empty()));
        Ok(())
    }

    #[test]
    fn supported_model_validation_should_match_adapter_rules() {
        assert!(is_supported_model(&AdapterType::Openai, "gpt-4o"));
        assert!(is_supported_model(&AdapterType::Openai, "o3-mini"));
        assert!(!is_supported_model(&AdapterType::Openai, "llama3.2"));

        assert!(is_supported_model(&AdapterType::Ollama, "llama3.2"));
        assert!(is_supported_model(&AdapterType::Test, "any-model"));
        assert!(!is_supported_model(&AdapterType::Test, "   "));
    }

    #[tokio::test]
    async fn create_agent_should_reject_unsupported_openai_model() -> Result<()> {
        let (_tdb, state) = AppState::new_for_test().await?;
        let input = CreateAgent::new(
            "bad-openai",
            AgentType::Proxy,
            AdapterType::Openai,
            "llama3.2",
            "You are a helpful assistant",
            HashMap::<String, String>::new(),
        );
        let err = state
            .create_agent(input, 1)
            .await
            .expect_err("expected error");
        assert_eq!(
            err.to_string(),
            "create agent error: model 'llama3.2' is not supported by adapter Openai"
        );
        Ok(())
    }

    #[tokio::test]
    async fn create_agent_should_reject_empty_model() -> Result<()> {
        let (_tdb, state) = AppState::new_for_test().await?;
        let input = CreateAgent::new(
            "empty-model",
            AgentType::Proxy,
            AdapterType::Ollama,
            "   ",
            "You are a helpful assistant",
            HashMap::<String, String>::new(),
        );
        let err = state
            .create_agent(input, 1)
            .await
            .expect_err("expected error");
        assert_eq!(
            err.to_string(),
            "create agent error: model '   ' is not supported by adapter Ollama"
        );
        Ok(())
    }

    #[tokio::test]
    async fn update_agent_should_work() -> Result<()> {
        let (_tdb, state) = AppState::new_for_test().await?;
        // create an agent
        let input = CreateAgent::new(
            "test",
            AgentType::Proxy,
            AdapterType::Ollama,
            "llama3.2",
            "You are a helpful assistant",
            HashMap::<String, String>::new(),
        );
        let agent = state
            .create_agent(input, 1)
            .await
            .expect("create agent failed");
        // update the agent
        let input = UpdateAgent::new(
            agent.id as u64,
            "Can you tell me the weather in Tokyo?",
            HashMap::<String, String>::new(),
        );
        let agent = state
            .update_agent(input, 1)
            .await
            .expect("update agent failed");
        assert_eq!(agent.prompt, "Can you tell me the weather in Tokyo?");
        assert_eq!(agent.args, sqlx::types::Json(AgentArgs::empty()));
        Ok(())
    }
}
