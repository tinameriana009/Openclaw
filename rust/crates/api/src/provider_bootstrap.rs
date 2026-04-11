use crate::{resolve_model_alias, ApiError, AuthSource, PromptCache, ProviderClient};

#[derive(Debug)]
pub struct ProviderRuntimeBootstrap {
    client: ProviderClient,
    model: String,
    runtime: tokio::runtime::Runtime,
}

impl ProviderRuntimeBootstrap {
    pub fn new(
        model: impl AsRef<str>,
        anthropic_auth: Option<AuthSource>,
        prompt_cache_namespace: Option<&str>,
    ) -> Result<Self, ApiError> {
        let resolved_model = resolve_model_alias(model.as_ref()).to_string();
        let client =
            ProviderClient::from_model_with_anthropic_auth(&resolved_model, anthropic_auth)?;
        let client = match prompt_cache_namespace {
            Some(namespace) => client.with_prompt_cache(PromptCache::new(namespace)),
            None => client,
        };
        let runtime = tokio::runtime::Runtime::new().map_err(|error| {
            ApiError::Auth(format!(
                "provider runtime initialization failed for model={resolved_model}: {error}"
            ))
        })?;
        Ok(Self {
            client,
            model: resolved_model,
            runtime,
        })
    }

    #[cfg(test)]
    #[must_use]
    pub fn model(&self) -> &str {
        &self.model
    }

    #[must_use]
    pub fn into_parts(self) -> (ProviderClient, String, tokio::runtime::Runtime) {
        (self.client, self.model, self.runtime)
    }
}

#[cfg(test)]
mod tests {
    use super::ProviderRuntimeBootstrap;
    use crate::AuthSource;

    #[test]
    fn resolves_model_aliases_during_bootstrap() {
        let bootstrap = ProviderRuntimeBootstrap::new(
            "opus",
            Some(AuthSource::ApiKey("test-key".to_string())),
            Some("bootstrap-test"),
        )
        .expect("bootstrap should build");

        assert_eq!(bootstrap.model(), "claude-opus-4-6");
    }
}
