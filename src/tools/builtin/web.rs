//! Native Worker web tools for controlled fetch and search access.

use std::collections::HashMap;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use futures::StreamExt;
use regex::Regex;
use reqwest::Url;
use serde_json::{Value, json};

use crate::context::JobContext;
use crate::llm::recording::{HttpExchangeRequest, HttpExchangeResponse};
use crate::safety::LeakDetector;
use crate::tools::builtin::http::{build_pinned_client, validate_and_resolve_url, validate_url};
use crate::tools::tool::{
    ApprovalRequirement, Tool, ToolDomain, ToolError, ToolOutput, ToolRateLimitConfig, require_str,
};

#[cfg(feature = "html-to-markdown")]
use html_to_markdown_rs::convert;
#[cfg(feature = "html-to-markdown")]
use readabilityrs::Readability;

const MAX_RESPONSE_SIZE: usize = 5 * 1024 * 1024;
const DEFAULT_FETCH_TIMEOUT_SECS: u64 = 30;
const MAX_FETCH_TIMEOUT_SECS: u64 = 120;
const MAX_REDIRECTS: usize = 3;
const DEFAULT_MAX_CHARS: usize = 50_000;
const MIN_MAX_CHARS: usize = 100;
const MAX_MAX_CHARS: usize = 200_000;
const DEFAULT_SEARCH_TIMEOUT_SECS: u64 = 12;
const DEFAULT_SEARCH_RESULTS: usize = 5;
const MAX_SEARCH_RESULTS: usize = 10;
const ACCEPT_HEADER: &str =
    "text/markdown, text/html;q=0.9, application/json;q=0.9, text/plain;q=0.8, */*;q=0.5";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExtractMode {
    Markdown,
    Text,
}

impl ExtractMode {
    fn as_str(self) -> &'static str {
        match self {
            Self::Markdown => "markdown",
            Self::Text => "text",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SearchProvider {
    Tavily,
    Brave,
}

impl SearchProvider {
    fn as_str(self) -> &'static str {
        match self {
            Self::Tavily => "tavily",
            Self::Brave => "brave",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SearchProviderPreference {
    Auto,
    Provider(SearchProvider),
}

impl SearchProviderPreference {
    fn label(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Provider(provider) => provider.as_str(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct WebFetchTool;

impl WebFetchTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for WebFetchTool {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct WebSearchTool {
    config: WebSearchConfig,
}

impl WebSearchTool {
    pub fn from_env() -> Self {
        Self {
            config: WebSearchConfig::from_env(),
        }
    }

    #[cfg(test)]
    fn with_config(config: WebSearchConfig) -> Self {
        Self { config }
    }
}

#[derive(Debug, Clone)]
struct WebSearchConfig {
    provider: SearchProviderPreference,
    fallback_provider: Option<SearchProvider>,
    max_results: usize,
    timeout: Duration,
    tavily_api_key: Option<String>,
    brave_api_key: Option<String>,
    tavily_url: String,
    brave_url: String,
}

impl WebSearchConfig {
    fn from_env() -> Self {
        let provider = parse_provider_preference_env(
            std::env::var("IRONCLAW_WEB_SEARCH_PROVIDER")
                .ok()
                .as_deref(),
        )
        .unwrap_or(SearchProviderPreference::Auto);
        let fallback_provider = parse_optional_provider_env(
            std::env::var("IRONCLAW_WEB_SEARCH_FALLBACK_PROVIDER")
                .ok()
                .as_deref(),
        );
        let max_results = std::env::var("IRONCLAW_WEB_SEARCH_MAX_RESULTS")
            .ok()
            .and_then(|raw| raw.parse::<usize>().ok())
            .unwrap_or(DEFAULT_SEARCH_RESULTS)
            .clamp(1, MAX_SEARCH_RESULTS);
        let timeout_secs = std::env::var("IRONCLAW_WEB_SEARCH_TIMEOUT_SECS")
            .ok()
            .and_then(|raw| raw.parse::<u64>().ok())
            .unwrap_or(DEFAULT_SEARCH_TIMEOUT_SECS)
            .clamp(1, MAX_FETCH_TIMEOUT_SECS);

        Self {
            provider,
            fallback_provider,
            max_results,
            timeout: Duration::from_secs(timeout_secs),
            tavily_api_key: env_var_trimmed("TAVILY_API_KEY"),
            brave_api_key: env_var_trimmed("BRAVE_API_KEY"),
            tavily_url: std::env::var("IRONCLAW_WEB_SEARCH_TAVILY_URL")
                .unwrap_or_else(|_| "https://api.tavily.com/search".to_string()),
            brave_url: std::env::var("IRONCLAW_WEB_SEARCH_BRAVE_URL")
                .unwrap_or_else(|_| "https://api.search.brave.com/res/v1/web/search".to_string()),
        }
    }
}

#[derive(Debug)]
struct FetchHttpResponse {
    status: u16,
    headers: HashMap<String, String>,
    body: String,
    final_url: String,
    redirects_followed: usize,
}

#[derive(Debug)]
struct FetchContent {
    extractor: &'static str,
    text: String,
    json_body: Option<Value>,
}

#[derive(Debug, Clone)]
struct SearchAttempt {
    provider: SearchProvider,
    outcome: Value,
}

#[derive(Debug, Clone)]
struct SearchProviderResult {
    rows: Vec<Value>,
    provider_used: SearchProvider,
}

#[derive(Debug, Clone)]
struct SearchProviderError {
    kind: &'static str,
    message: String,
    retry_after_secs: Option<u64>,
}

#[async_trait]
impl Tool for WebFetchTool {
    fn name(&self) -> &str {
        "web_fetch"
    }

    fn description(&self) -> &str {
        "Fetch a URL with Worker-native network safety controls and return structured extracted content."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "HTTPS URL to fetch"
                },
                "extractMode": {
                    "type": "string",
                    "enum": ["markdown", "text"],
                    "description": "Extraction mode (mock-compatible alias)"
                },
                "extract_mode": {
                    "type": "string",
                    "enum": ["markdown", "text"],
                    "description": "Extraction mode"
                },
                "maxChars": {
                    "type": "integer",
                    "minimum": MIN_MAX_CHARS,
                    "maximum": MAX_MAX_CHARS,
                    "description": "Maximum characters to return (mock-compatible alias)"
                },
                "max_chars": {
                    "type": "integer",
                    "minimum": MIN_MAX_CHARS,
                    "maximum": MAX_MAX_CHARS,
                    "description": "Maximum characters to return"
                },
                "timeout_secs": {
                    "type": "integer",
                    "minimum": 1,
                    "maximum": MAX_FETCH_TIMEOUT_SECS,
                    "description": "Fetch timeout in seconds"
                }
            },
            "required": ["url"]
        })
    }

    async fn execute(&self, params: Value, ctx: &JobContext) -> Result<ToolOutput, ToolError> {
        let start = Instant::now();
        let url = require_str(&params, "url")?;
        let extract_mode = parse_extract_mode(&params)?;
        let max_chars = parse_fetch_max_chars(&params)?;
        let timeout = parse_timeout_secs(params.get("timeout_secs"))?;

        let result = match fetch_url(url, timeout, ctx).await {
            Ok(response) => build_fetch_success(url, extract_mode, max_chars, timeout, response)?,
            Err(error) => build_fetch_error(url, extract_mode, max_chars, timeout, error),
        };

        Ok(ToolOutput::success(result, start.elapsed()))
    }

    fn requires_sanitization(&self) -> bool {
        true
    }

    fn requires_approval(&self, _params: &Value) -> ApprovalRequirement {
        ApprovalRequirement::Never
    }

    fn execution_timeout(&self) -> Duration {
        Duration::from_secs(MAX_FETCH_TIMEOUT_SECS + 5)
    }

    fn domain(&self) -> ToolDomain {
        ToolDomain::Container
    }

    fn rate_limit_config(&self) -> Option<ToolRateLimitConfig> {
        Some(ToolRateLimitConfig::new(20, 200))
    }
}

#[async_trait]
impl Tool for WebSearchTool {
    fn name(&self) -> &str {
        "web_search"
    }

    fn description(&self) -> &str {
        "Search the web with Worker-native providers and structured provider/fallback evidence."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Search query"
                },
                "count": {
                    "type": "integer",
                    "minimum": 1,
                    "maximum": MAX_SEARCH_RESULTS,
                    "description": "Maximum number of normalized results to return"
                },
                "provider": {
                    "type": "string",
                    "enum": ["auto", "tavily", "brave"],
                    "description": "Optional provider override"
                },
                "fallback_provider": {
                    "type": "string",
                    "enum": ["none", "tavily", "brave"],
                    "description": "Optional fallback provider override"
                }
            },
            "required": ["query"]
        })
    }

    async fn execute(&self, params: Value, ctx: &JobContext) -> Result<ToolOutput, ToolError> {
        let start = Instant::now();
        let query = require_str(&params, "query")?.trim().to_string();
        if query.is_empty() {
            return Err(ToolError::InvalidParameters(
                "query must not be empty".to_string(),
            ));
        }

        let count = parse_result_count(&params, self.config.max_results)?;
        let provider = parse_provider_preference_value(params.get("provider"))?
            .unwrap_or(self.config.provider);
        let fallback_provider = if params.get("fallback_provider").is_some() {
            parse_fallback_provider_value(params.get("fallback_provider"))?
        } else {
            self.config.fallback_provider
        };

        let result = search_web(
            &self.config,
            &query,
            count,
            provider,
            fallback_provider,
            ctx,
        )
        .await;

        Ok(ToolOutput::success(result, start.elapsed()))
    }

    fn requires_sanitization(&self) -> bool {
        true
    }

    fn requires_approval(&self, _params: &Value) -> ApprovalRequirement {
        ApprovalRequirement::Never
    }

    fn execution_timeout(&self) -> Duration {
        Duration::from_secs(MAX_FETCH_TIMEOUT_SECS + 5)
    }

    fn domain(&self) -> ToolDomain {
        ToolDomain::Container
    }

    fn rate_limit_config(&self) -> Option<ToolRateLimitConfig> {
        Some(ToolRateLimitConfig::new(20, 200))
    }
}

async fn fetch_url(
    url: &str,
    timeout: Duration,
    ctx: &JobContext,
) -> Result<FetchHttpResponse, ToolError> {
    let initial_url = validate_url(url)?;
    let initial_request = HttpExchangeRequest {
        method: "GET".to_string(),
        url: initial_url.to_string(),
        headers: vec![(
            reqwest::header::ACCEPT.to_string(),
            ACCEPT_HEADER.to_string(),
        )],
        body: None,
    };

    if let Some(interceptor) = ctx.http_interceptor.as_ref()
        && let Some(recorded) = interceptor.before_request(&initial_request).await
    {
        return Ok(FetchHttpResponse {
            status: recorded.status,
            headers: header_map_from_pairs(&recorded.headers),
            body: recorded.body,
            final_url: initial_request.url,
            redirects_followed: 0,
        });
    }

    let detector = LeakDetector::new();
    detector
        .scan_http_request(initial_request.url.as_str(), &initial_request.headers, None)
        .map_err(|error| ToolError::NotAuthorized(error.to_string()))?;

    let mut current_url = initial_url;
    let mut redirects_followed = 0usize;

    loop {
        let response = simple_get(&current_url, timeout).await?;
        let status = response.status().as_u16();

        if (300..400).contains(&status) {
            if redirects_followed >= MAX_REDIRECTS {
                return Err(ToolError::ExecutionFailed(format!(
                    "too many redirects (max {})",
                    MAX_REDIRECTS
                )));
            }

            let location = response
                .headers()
                .get(reqwest::header::LOCATION)
                .and_then(|value| value.to_str().ok())
                .ok_or_else(|| {
                    ToolError::ExecutionFailed(format!(
                        "redirect (HTTP {}) has no Location header",
                        status
                    ))
                })?;
            let next_url = current_url.join(location).map_err(|error| {
                ToolError::ExecutionFailed(format!(
                    "could not resolve relative redirect '{}': {}",
                    location, error
                ))
            })?;
            current_url = validate_url(next_url.as_str())?;
            redirects_followed += 1;
            continue;
        }

        let headers = collect_headers(response.headers());
        let body = read_text_body(response, &current_url, MAX_RESPONSE_SIZE).await?;

        if let Some(interceptor) = ctx.http_interceptor.as_ref() {
            interceptor
                .after_response(
                    &initial_request,
                    &HttpExchangeResponse {
                        status,
                        headers: headers
                            .iter()
                            .map(|(key, value)| (key.clone(), value.clone()))
                            .collect(),
                        body: body.clone(),
                    },
                )
                .await;
        }

        return Ok(FetchHttpResponse {
            status,
            headers,
            body,
            final_url: current_url.to_string(),
            redirects_followed,
        });
    }
}

fn build_fetch_success(
    url: &str,
    extract_mode: ExtractMode,
    max_chars: usize,
    timeout: Duration,
    response: FetchHttpResponse,
) -> Result<Value, ToolError> {
    let content_type = response_content_type(&response.headers);
    let extracted = extract_fetch_content(
        &response.body,
        content_type.as_deref(),
        extract_mode,
        &response.final_url,
    )?;
    let (text, truncated) = truncate_text(&extracted.text, max_chars);
    let length = text.chars().count();

    Ok(json!({
        "ok": response.status < 400,
        "status": if response.status < 400 { "ok" } else { "failed" },
        "url": url,
        "finalUrl": response.final_url,
        "http_status": response.status,
        "status_code": response.status,
        "content_type": content_type,
        "extractMode": extract_mode.as_str(),
        "extract_mode": extract_mode.as_str(),
        "extractor": extracted.extractor,
        "truncated": truncated,
        "maxChars": max_chars,
        "max_chars": max_chars,
        "length": length,
        "text": text,
        "json": extracted.json_body,
        "evidence": {
            "kind": "web_fetch",
            "url": url,
            "final_url": response.final_url,
            "status_code": response.status,
            "content_type": content_type,
            "extract_mode": extract_mode.as_str(),
            "extractor": extracted.extractor,
            "truncated": truncated,
            "max_chars": max_chars,
            "timeout_secs": timeout.as_secs(),
            "redirects_followed": response.redirects_followed,
        }
    }))
}

fn build_fetch_error(
    url: &str,
    extract_mode: ExtractMode,
    max_chars: usize,
    timeout: Duration,
    error: ToolError,
) -> Value {
    let (status, kind, retry_after_secs) = classify_tool_error(&error);
    json!({
        "ok": false,
        "status": status,
        "url": url,
        "finalUrl": url,
        "http_status": Value::Null,
        "status_code": Value::Null,
        "content_type": Value::Null,
        "extractMode": extract_mode.as_str(),
        "extract_mode": extract_mode.as_str(),
        "extractor": Value::Null,
        "truncated": false,
        "maxChars": max_chars,
        "max_chars": max_chars,
        "length": 0,
        "text": Value::Null,
        "json": Value::Null,
        "error": {
            "kind": kind,
            "message": error.to_string(),
            "retry_after_secs": retry_after_secs,
        },
        "evidence": {
            "kind": "web_fetch",
            "url": url,
            "final_url": url,
            "status_code": Value::Null,
            "content_type": Value::Null,
            "extract_mode": extract_mode.as_str(),
            "extractor": Value::Null,
            "truncated": false,
            "max_chars": max_chars,
            "timeout_secs": timeout.as_secs(),
            "redirects_followed": 0,
        }
    })
}

async fn search_web(
    config: &WebSearchConfig,
    query: &str,
    count: usize,
    provider: SearchProviderPreference,
    fallback_provider: Option<SearchProvider>,
    ctx: &JobContext,
) -> Value {
    let mut attempts = Vec::new();
    let provider_order = provider_order(provider, fallback_provider);
    let mut first_error: Option<SearchProviderError> = None;

    for (index, provider_name) in provider_order.iter().copied().enumerate() {
        match search_provider(config, provider_name, query, count, ctx).await {
            Ok(result) => {
                let fallback_used = index > 0;
                attempts.push(SearchAttempt {
                    provider: provider_name,
                    outcome: json!({
                        "provider": provider_name.as_str(),
                        "ok": true,
                        "count": result.rows.len(),
                    }),
                });
                return json!({
                    "ok": true,
                    "status": if fallback_used { "degraded" } else { "ok" },
                    "query": query,
                    "count": count,
                    "provider_requested": provider.label(),
                    "provider_used": result.provider_used.as_str(),
                    "fallback_used": fallback_used,
                    "results": result.rows,
                    "error": Value::Null,
                    "evidence": {
                        "kind": "web_search",
                        "provider_requested": provider.label(),
                        "provider_used": result.provider_used.as_str(),
                        "fallback_used": fallback_used,
                        "attempts": attempts.into_iter().map(search_attempt_to_value).collect::<Vec<_>>(),
                    }
                });
            }
            Err(error) => {
                if first_error.is_none() {
                    first_error = Some(error.clone());
                }
                attempts.push(SearchAttempt {
                    provider: provider_name,
                    outcome: json!({
                        "provider": provider_name.as_str(),
                        "ok": false,
                        "error": {
                            "kind": error.kind,
                            "message": error.message,
                            "retry_after_secs": error.retry_after_secs,
                        }
                    }),
                });
            }
        }
    }

    let error = first_error.unwrap_or(SearchProviderError {
        kind: "not_configured",
        message: "no web search provider available".to_string(),
        retry_after_secs: None,
    });
    let status = if error.kind == "auth_missing" {
        "blocked"
    } else {
        "failed"
    };

    json!({
        "ok": false,
        "status": status,
        "query": query,
        "count": count,
        "provider_requested": provider.label(),
        "provider_used": Value::Null,
        "fallback_used": false,
        "results": Vec::<Value>::new(),
        "error": {
            "kind": error.kind,
            "message": error.message,
            "retry_after_secs": error.retry_after_secs,
        },
        "evidence": {
            "kind": "web_search",
            "provider_requested": provider.label(),
            "provider_used": Value::Null,
            "fallback_used": false,
            "attempts": attempts.into_iter().map(search_attempt_to_value).collect::<Vec<_>>(),
        }
    })
}

async fn search_provider(
    config: &WebSearchConfig,
    provider: SearchProvider,
    query: &str,
    count: usize,
    ctx: &JobContext,
) -> Result<SearchProviderResult, SearchProviderError> {
    match provider {
        SearchProvider::Tavily => search_tavily(config, query, count, ctx).await,
        SearchProvider::Brave => search_brave(config, query, count, ctx).await,
    }
}

async fn search_tavily(
    config: &WebSearchConfig,
    query: &str,
    count: usize,
    ctx: &JobContext,
) -> Result<SearchProviderResult, SearchProviderError> {
    let api_key = config
        .tavily_api_key
        .as_deref()
        .ok_or_else(|| SearchProviderError {
            kind: "auth_missing",
            message: "TAVILY_API_KEY not configured".to_string(),
            retry_after_secs: None,
        })?;

    let body = json!({
        "api_key": api_key,
        "query": query,
        "search_depth": "basic",
        "max_results": count,
        "include_answer": false,
    });

    let response = perform_json_request(
        "POST",
        &config.tavily_url,
        vec![(
            reqwest::header::CONTENT_TYPE.to_string(),
            "application/json".to_string(),
        )],
        Some(body),
        config.timeout,
        ctx,
    )
    .await
    .map_err(map_search_tool_error)?;

    ensure_success_status(response.status, "tavily", &response.headers)?;
    let parsed: Value =
        serde_json::from_str(&response.body).map_err(|error| SearchProviderError {
            kind: "invalid_response",
            message: format!("Tavily returned invalid JSON: {}", error),
            retry_after_secs: None,
        })?;
    let rows = parsed
        .get("results")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .take(count)
        .filter_map(|item| {
            item.as_object().map(|object| {
                json!({
                    "title": object.get("title").and_then(Value::as_str).unwrap_or_default(),
                    "url": object.get("url").and_then(Value::as_str).unwrap_or_default(),
                    "description": object.get("content").and_then(Value::as_str).unwrap_or_default(),
                })
            })
        })
        .collect();

    Ok(SearchProviderResult {
        rows,
        provider_used: SearchProvider::Tavily,
    })
}

async fn search_brave(
    config: &WebSearchConfig,
    query: &str,
    count: usize,
    ctx: &JobContext,
) -> Result<SearchProviderResult, SearchProviderError> {
    let api_key = config
        .brave_api_key
        .as_deref()
        .ok_or_else(|| SearchProviderError {
            kind: "auth_missing",
            message: "BRAVE_API_KEY not configured".to_string(),
            retry_after_secs: None,
        })?;

    let mut url = Url::parse(&config.brave_url).map_err(|error| SearchProviderError {
        kind: "config_error",
        message: format!("invalid Brave endpoint URL: {}", error),
        retry_after_secs: None,
    })?;
    url.query_pairs_mut()
        .append_pair("q", query)
        .append_pair("count", &count.to_string());

    let response = perform_json_request(
        "GET",
        url.as_str(),
        vec![
            (
                reqwest::header::ACCEPT.to_string(),
                "application/json".to_string(),
            ),
            ("X-Subscription-Token".to_string(), api_key.to_string()),
        ],
        None,
        config.timeout,
        ctx,
    )
    .await
    .map_err(map_search_tool_error)?;

    ensure_success_status(response.status, "brave", &response.headers)?;
    let parsed: Value =
        serde_json::from_str(&response.body).map_err(|error| SearchProviderError {
            kind: "invalid_response",
            message: format!("Brave returned invalid JSON: {}", error),
            retry_after_secs: None,
        })?;
    let rows = parsed
        .get("web")
        .and_then(Value::as_object)
        .and_then(|web| web.get("results"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .take(count)
        .filter_map(|item| {
            item.as_object().map(|object| {
                json!({
                    "title": object.get("title").and_then(Value::as_str).unwrap_or_default(),
                    "url": object.get("url").and_then(Value::as_str).unwrap_or_default(),
                    "description": object.get("description").and_then(Value::as_str).unwrap_or_default(),
                })
            })
        })
        .collect();

    Ok(SearchProviderResult {
        rows,
        provider_used: SearchProvider::Brave,
    })
}

async fn perform_json_request(
    method: &str,
    url: &str,
    headers: Vec<(String, String)>,
    body: Option<Value>,
    timeout: Duration,
    ctx: &JobContext,
) -> Result<FetchHttpResponse, ToolError> {
    let parsed_url = validate_url(url)?;
    let body_string = body
        .as_ref()
        .map(serde_json::to_string)
        .transpose()
        .map_err(|error| ToolError::InvalidParameters(format!("invalid JSON body: {}", error)))?;

    let request = HttpExchangeRequest {
        method: method.to_string(),
        url: parsed_url.to_string(),
        headers: headers.clone(),
        body: body_string.clone(),
    };

    if let Some(interceptor) = ctx.http_interceptor.as_ref()
        && let Some(recorded) = interceptor.before_request(&request).await
    {
        return Ok(FetchHttpResponse {
            status: recorded.status,
            headers: header_map_from_pairs(&recorded.headers),
            body: recorded.body,
            final_url: parsed_url.to_string(),
            redirects_followed: 0,
        });
    }

    let detector = LeakDetector::new();
    detector
        .scan_http_request(
            parsed_url.as_str(),
            &headers,
            body_string.as_ref().map(String::as_bytes),
        )
        .map_err(|error| ToolError::NotAuthorized(error.to_string()))?;

    let host = parsed_url
        .host_str()
        .ok_or_else(|| ToolError::InvalidParameters("URL missing host".to_string()))?
        .to_string();
    let resolved_addrs = validate_and_resolve_url(&parsed_url).await?;
    let client = build_pinned_client(
        &host,
        &resolved_addrs,
        timeout,
        reqwest::redirect::Policy::none(),
    )?;

    let mut req = match method {
        "GET" => client.get(parsed_url.clone()),
        "POST" => client.post(parsed_url.clone()),
        other => {
            return Err(ToolError::InvalidParameters(format!(
                "unsupported method: {}",
                other
            )));
        }
    }
    .timeout(timeout);

    for (key, value) in &headers {
        req = req.header(key.as_str(), value.as_str());
    }
    if let Some(body_string) = body_string.clone() {
        req = req.body(body_string);
    }

    let response = req.send().await.map_err(|error| {
        if error.is_timeout() {
            ToolError::Timeout(timeout)
        } else {
            ToolError::ExternalService(error.to_string())
        }
    })?;

    if let Some(interceptor) = ctx.http_interceptor.as_ref() {
        let status = response.status().as_u16();
        let response_headers = collect_headers(response.headers());
        let body = read_text_body(response, &parsed_url, MAX_RESPONSE_SIZE).await?;
        interceptor
            .after_response(
                &request,
                &HttpExchangeResponse {
                    status,
                    headers: response_headers
                        .iter()
                        .map(|(key, value)| (key.clone(), value.clone()))
                        .collect(),
                    body: body.clone(),
                },
            )
            .await;
        return Ok(FetchHttpResponse {
            status,
            headers: response_headers,
            body,
            final_url: parsed_url.to_string(),
            redirects_followed: 0,
        });
    }

    let status = response.status().as_u16();
    let response_headers = collect_headers(response.headers());
    let body = read_text_body(response, &parsed_url, MAX_RESPONSE_SIZE).await?;

    Ok(FetchHttpResponse {
        status,
        headers: response_headers,
        body,
        final_url: parsed_url.to_string(),
        redirects_followed: 0,
    })
}

async fn simple_get(url: &Url, timeout: Duration) -> Result<reqwest::Response, ToolError> {
    let host = url
        .host_str()
        .ok_or_else(|| ToolError::InvalidParameters("URL missing host".to_string()))?
        .to_string();
    let resolved_addrs = validate_and_resolve_url(url).await?;
    let client = build_pinned_client(
        &host,
        &resolved_addrs,
        timeout,
        reqwest::redirect::Policy::none(),
    )?;

    client
        .get(url.clone())
        .header(reqwest::header::ACCEPT, ACCEPT_HEADER)
        .send()
        .await
        .map_err(|error| {
            if error.is_timeout() {
                ToolError::Timeout(timeout)
            } else {
                ToolError::ExternalService(error.to_string())
            }
        })
}

async fn read_text_body(
    response: reqwest::Response,
    url: &Url,
    max_size: usize,
) -> Result<String, ToolError> {
    if let Some(content_length) = response.headers().get(reqwest::header::CONTENT_LENGTH)
        && let Ok(content_length) = content_length.to_str()
        && let Ok(content_length) = content_length.parse::<usize>()
        && content_length > max_size
    {
        return Err(ToolError::ExecutionFailed(format!(
            "response Content-Length ({} bytes) exceeds maximum allowed size ({} bytes) for {}",
            content_length, max_size, url
        )));
    }

    let mut body = Vec::new();
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|error| {
            ToolError::ExternalService(format!("failed to read response body: {}", error))
        })?;
        if body.len() + chunk.len() > max_size {
            return Err(ToolError::ExecutionFailed(format!(
                "response body exceeds maximum allowed size ({} bytes)",
                max_size
            )));
        }
        body.extend_from_slice(&chunk);
    }

    Ok(String::from_utf8_lossy(&body).into_owned())
}

fn extract_fetch_content(
    body: &str,
    content_type: Option<&str>,
    extract_mode: ExtractMode,
    final_url: &str,
) -> Result<FetchContent, ToolError> {
    if content_type.is_some_and(|value| value.contains("application/json")) {
        let parsed: Value = serde_json::from_str(body)
            .map_err(|error| ToolError::ExecutionFailed(format!("invalid JSON body: {}", error)))?;
        return Ok(FetchContent {
            extractor: "json",
            text: serde_json::to_string_pretty(&parsed).map_err(|error| {
                ToolError::ExecutionFailed(format!("failed to format JSON body: {}", error))
            })?,
            json_body: Some(parsed),
        });
    }

    if content_type.is_some_and(is_html_content_type) || looks_like_html(body) {
        let text = extract_html(body, final_url, extract_mode)?;
        return Ok(FetchContent {
            extractor: "readability",
            text,
            json_body: None,
        });
    }

    Ok(FetchContent {
        extractor: "raw",
        text: body.to_string(),
        json_body: None,
    })
}

#[cfg(feature = "html-to-markdown")]
fn extract_html(html: &str, url: &str, extract_mode: ExtractMode) -> Result<String, ToolError> {
    let readability = Readability::new(html, Some(url), None)
        .map_err(|error| ToolError::ExecutionFailed(format!("readability parser: {:?}", error)))?;
    let article = readability.parse().ok_or_else(|| {
        ToolError::ExecutionFailed("failed to extract article content".to_string())
    })?;
    let clean_html = article.content.ok_or_else(|| {
        ToolError::ExecutionFailed("no content extracted from article".to_string())
    })?;

    match extract_mode {
        ExtractMode::Markdown => convert(&clean_html, None)
            .map_err(|error| ToolError::ExecutionFailed(format!("HTML to markdown: {}", error))),
        ExtractMode::Text => {
            let text = strip_html(&clean_html);
            if text.is_empty() {
                return Err(ToolError::ExecutionFailed(
                    "no readable text extracted from article".to_string(),
                ));
            }
            Ok(text)
        }
    }
}

#[cfg(not(feature = "html-to-markdown"))]
fn extract_html(html: &str, _url: &str, _extract_mode: ExtractMode) -> Result<String, ToolError> {
    Ok(strip_html(html))
}

fn strip_html(input: &str) -> String {
    static SCRIPT_RE: OnceLock<Regex> = OnceLock::new();
    static STYLE_RE: OnceLock<Regex> = OnceLock::new();
    static TAG_RE: OnceLock<Regex> = OnceLock::new();
    static WHITESPACE_RE: OnceLock<Regex> = OnceLock::new();

    let without_script = SCRIPT_RE
        .get_or_init(|| Regex::new(r"(?is)<script[\s\S]*?</script>").expect("valid script regex"))
        .replace_all(input, " ");
    let without_style = STYLE_RE
        .get_or_init(|| Regex::new(r"(?is)<style[\s\S]*?</style>").expect("valid style regex"))
        .replace_all(&without_script, " ");
    let without_tags = TAG_RE
        .get_or_init(|| Regex::new(r"(?is)<[^>]+>").expect("valid tag regex"))
        .replace_all(&without_style, " ");
    let normalized = WHITESPACE_RE
        .get_or_init(|| Regex::new(r"[ \t\r\n]+").expect("valid whitespace regex"))
        .replace_all(&without_tags, " ");

    normalized.trim().to_string()
}

fn truncate_text(input: &str, max_chars: usize) -> (String, bool) {
    let total_chars = input.chars().count();
    if total_chars <= max_chars {
        return (input.to_string(), false);
    }

    let truncated = input.chars().take(max_chars).collect::<String>();
    (truncated, true)
}

fn parse_extract_mode(params: &Value) -> Result<ExtractMode, ToolError> {
    match first_string(params, &["extract_mode", "extractMode"]) {
        None => Ok(ExtractMode::Markdown),
        Some("markdown") => Ok(ExtractMode::Markdown),
        Some("text") => Ok(ExtractMode::Text),
        Some(other) => Err(ToolError::InvalidParameters(format!(
            "extract_mode must be 'markdown' or 'text', got '{}'",
            other
        ))),
    }
}

fn parse_fetch_max_chars(params: &Value) -> Result<usize, ToolError> {
    let value = first_u64(params, &["max_chars", "maxChars"])
        .transpose()?
        .unwrap_or(DEFAULT_MAX_CHARS as u64);
    let parsed = usize::try_from(value).map_err(|_| {
        ToolError::InvalidParameters("max_chars is too large for this platform".to_string())
    })?;
    if !(MIN_MAX_CHARS..=MAX_MAX_CHARS).contains(&parsed) {
        return Err(ToolError::InvalidParameters(format!(
            "max_chars must be between {} and {}",
            MIN_MAX_CHARS, MAX_MAX_CHARS
        )));
    }
    Ok(parsed)
}

fn parse_timeout_secs(value: Option<&Value>) -> Result<Duration, ToolError> {
    let secs = match value {
        None | Some(Value::Null) => DEFAULT_FETCH_TIMEOUT_SECS,
        Some(Value::Number(number)) => number.as_u64().ok_or_else(|| {
            ToolError::InvalidParameters("timeout_secs must be a positive integer".to_string())
        })?,
        Some(Value::String(raw)) => raw.parse::<u64>().map_err(|_| {
            ToolError::InvalidParameters("timeout_secs must be a positive integer".to_string())
        })?,
        Some(_) => {
            return Err(ToolError::InvalidParameters(
                "timeout_secs must be a positive integer".to_string(),
            ));
        }
    };

    if secs == 0 || secs > MAX_FETCH_TIMEOUT_SECS {
        return Err(ToolError::InvalidParameters(format!(
            "timeout_secs must be between 1 and {}",
            MAX_FETCH_TIMEOUT_SECS
        )));
    }

    Ok(Duration::from_secs(secs))
}

fn parse_result_count(params: &Value, default: usize) -> Result<usize, ToolError> {
    let value = params.get("count").map(|count| {
        count.as_u64().ok_or_else(|| {
            ToolError::InvalidParameters("count must be a positive integer".to_string())
        })
    });

    let parsed = match value {
        Some(result) => usize::try_from(result?).map_err(|_| {
            ToolError::InvalidParameters("count is too large for this platform".to_string())
        })?,
        None => default,
    };

    if !(1..=MAX_SEARCH_RESULTS).contains(&parsed) {
        return Err(ToolError::InvalidParameters(format!(
            "count must be between 1 and {}",
            MAX_SEARCH_RESULTS
        )));
    }

    Ok(parsed)
}

fn parse_provider_preference_value(
    value: Option<&Value>,
) -> Result<Option<SearchProviderPreference>, ToolError> {
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(raw)) => parse_provider_preference_env(Some(raw))
            .map(Some)
            .ok_or_else(|| {
                ToolError::InvalidParameters(
                    "provider must be one of: auto, tavily, brave".to_string(),
                )
            }),
        Some(_) => Err(ToolError::InvalidParameters(
            "provider must be a string".to_string(),
        )),
    }
}

fn parse_fallback_provider_value(
    value: Option<&Value>,
) -> Result<Option<SearchProvider>, ToolError> {
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(raw)) => {
            if raw.eq_ignore_ascii_case("none") {
                return Ok(None);
            }

            parse_optional_provider_env(Some(raw))
                .map(Some)
                .ok_or_else(|| {
                    ToolError::InvalidParameters(
                        "fallback_provider must be one of: none, tavily, brave".to_string(),
                    )
                })
        }
        Some(_) => Err(ToolError::InvalidParameters(
            "fallback_provider must be a string".to_string(),
        )),
    }
}

fn parse_provider_preference_env(value: Option<&str>) -> Option<SearchProviderPreference> {
    match value?.trim().to_ascii_lowercase().as_str() {
        "" => None,
        "auto" => Some(SearchProviderPreference::Auto),
        "tavily" => Some(SearchProviderPreference::Provider(SearchProvider::Tavily)),
        "brave" => Some(SearchProviderPreference::Provider(SearchProvider::Brave)),
        _ => None,
    }
}

fn parse_optional_provider_env(value: Option<&str>) -> Option<SearchProvider> {
    match value?.trim().to_ascii_lowercase().as_str() {
        "" | "none" => None,
        "tavily" => Some(SearchProvider::Tavily),
        "brave" => Some(SearchProvider::Brave),
        _ => None,
    }
}

fn provider_order(
    provider: SearchProviderPreference,
    fallback_provider: Option<SearchProvider>,
) -> Vec<SearchProvider> {
    let mut order = match provider {
        SearchProviderPreference::Auto => vec![SearchProvider::Tavily, SearchProvider::Brave],
        SearchProviderPreference::Provider(provider) => vec![provider],
    };
    if let Some(fallback_provider) = fallback_provider {
        order.push(fallback_provider);
    }
    order.dedup();
    order
}

fn map_search_tool_error(error: ToolError) -> SearchProviderError {
    match error {
        ToolError::Timeout(duration) => SearchProviderError {
            kind: "timeout",
            message: format!("provider timed out after {:?}", duration),
            retry_after_secs: None,
        },
        ToolError::RateLimited(retry_after) => SearchProviderError {
            kind: "rate_limited",
            message: "provider rate limited the request".to_string(),
            retry_after_secs: retry_after.map(|duration| duration.as_secs()),
        },
        ToolError::NotAuthorized(message) => SearchProviderError {
            kind: "blocked",
            message,
            retry_after_secs: None,
        },
        ToolError::ExternalService(message) => SearchProviderError {
            kind: "external_service",
            message,
            retry_after_secs: None,
        },
        ToolError::ExecutionFailed(message) => SearchProviderError {
            kind: "execution_failed",
            message,
            retry_after_secs: None,
        },
        ToolError::InvalidParameters(message) => SearchProviderError {
            kind: "config_error",
            message,
            retry_after_secs: None,
        },
        ToolError::Sandbox(message) => SearchProviderError {
            kind: "sandbox",
            message,
            retry_after_secs: None,
        },
    }
}

fn ensure_success_status(
    status: u16,
    provider_name: &str,
    headers: &HashMap<String, String>,
) -> Result<(), SearchProviderError> {
    if status < 400 {
        return Ok(());
    }

    let retry_after_secs = headers
        .get("retry-after")
        .and_then(|value| value.parse::<u64>().ok());

    let kind = match status {
        401 | 403 => "auth_failed",
        429 => "rate_limited",
        500..=599 => "upstream_error",
        _ => "http_error",
    };

    Err(SearchProviderError {
        kind,
        message: format!("{} returned HTTP {}", provider_name, status),
        retry_after_secs,
    })
}

fn response_content_type(headers: &HashMap<String, String>) -> Option<String> {
    headers.iter().find_map(|(key, value)| {
        if key.eq_ignore_ascii_case("content-type") {
            Some(value.to_ascii_lowercase())
        } else {
            None
        }
    })
}

fn is_html_content_type(content_type: &str) -> bool {
    content_type.contains("text/html") || content_type.contains("application/xhtml+xml")
}

fn looks_like_html(body: &str) -> bool {
    let trimmed = body.trim_start();
    trimmed.starts_with("<!doctype") || trimmed.starts_with("<html") || trimmed.starts_with("<HTML")
}

fn first_string<'a>(params: &'a Value, keys: &[&str]) -> Option<&'a str> {
    keys.iter()
        .find_map(|key| params.get(*key).and_then(Value::as_str))
}

fn first_u64(params: &Value, keys: &[&str]) -> Option<Result<u64, ToolError>> {
    keys.iter().find_map(|key| {
        params.get(*key).map(|value| match value {
            Value::Number(number) => number.as_u64().ok_or_else(|| {
                ToolError::InvalidParameters(format!("{} must be a positive integer", key))
            }),
            Value::String(raw) => raw.parse::<u64>().map_err(|_| {
                ToolError::InvalidParameters(format!("{} must be a positive integer", key))
            }),
            _ => Err(ToolError::InvalidParameters(format!(
                "{} must be a positive integer",
                key
            ))),
        })
    })
}

fn env_var_trimmed(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn collect_headers(headers: &reqwest::header::HeaderMap) -> HashMap<String, String> {
    headers
        .iter()
        .filter_map(|(key, value)| {
            value
                .to_str()
                .ok()
                .map(|value| (key.to_string(), value.to_string()))
        })
        .collect()
}

fn header_map_from_pairs(headers: &[(String, String)]) -> HashMap<String, String> {
    headers.iter().cloned().collect()
}

fn classify_tool_error(error: &ToolError) -> (&'static str, &'static str, Option<u64>) {
    match error {
        ToolError::Timeout(duration) => ("failed", "timeout", Some(duration.as_secs())),
        ToolError::RateLimited(retry_after) => (
            "failed",
            "rate_limited",
            retry_after.map(|duration| duration.as_secs()),
        ),
        ToolError::NotAuthorized(_) => ("blocked", "policy_blocked", None),
        ToolError::ExternalService(_) => ("failed", "external_service", None),
        ToolError::ExecutionFailed(_) => ("failed", "execution_failed", None),
        ToolError::InvalidParameters(_) => ("failed", "invalid_parameters", None),
        ToolError::Sandbox(_) => ("failed", "sandbox", None),
    }
}

fn search_attempt_to_value(attempt: SearchAttempt) -> Value {
    json!({
        "provider": attempt.provider.as_str(),
        "outcome": attempt.outcome,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    use crate::llm::recording::{HttpExchange, ReplayingHttpInterceptor};

    #[tokio::test]
    async fn web_fetch_returns_structured_markdown_with_evidence() {
        let tool = WebFetchTool::new();
        let ctx = JobContext {
            http_interceptor: Some(Arc::new(ReplayingHttpInterceptor::new(vec![HttpExchange {
                request: HttpExchangeRequest {
                    method: "GET".to_string(),
                    url: "https://example.com/article".to_string(),
                    headers: vec![],
                    body: None,
                },
                response: HttpExchangeResponse {
                    status: 200,
                    headers: vec![("content-type".to_string(), "text/html".to_string())],
                    body: "<html><body><article><h1>Title</h1><p>Alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu nu xi omicron pi rho sigma tau upsilon phi chi psi omega. This paragraph keeps going with more readable article text so the extractor has enough content to score the document as a real article instead of boilerplate filler.</p><p>Second paragraph with enough readable content for extraction to succeed in tests. It includes extra sentences about Rust systems programming, safety, performance, ergonomics, tooling, crates, ownership, borrowing, concurrency, networking, and structured evidence handling.</p><p>Third paragraph extends the body even more so the readability heuristic can confidently treat this as the dominant content block for markdown extraction in the unit test fixture.</p></article></body></html>".to_string(),
                },
            }]))),
            ..Default::default()
        };

        let output = tool
            .execute(
                json!({
                    "url": "https://example.com/article",
                    "extractMode": "markdown",
                    "maxChars": 5000
                }),
                &ctx,
            )
            .await
            .unwrap_or_else(|error| panic!("web_fetch failed unexpectedly: {error}"));

        assert_eq!(output.result["ok"], json!(true));
        assert_eq!(output.result["status"], json!("ok"));
        assert_eq!(output.result["extractMode"], json!("markdown"));
        assert_eq!(output.result["extractor"], json!("readability"));
        assert_eq!(output.result["evidence"]["kind"], json!("web_fetch"));
        assert_eq!(output.result["evidence"]["truncated"], json!(false));
    }

    #[tokio::test]
    async fn web_fetch_returns_blocked_json_for_disallowed_target() {
        let tool = WebFetchTool::new();
        let output = tool
            .execute(
                json!({
                    "url": "https://localhost/secret",
                }),
                &JobContext::default(),
            )
            .await
            .unwrap_or_else(|error| {
                panic!("web_fetch should structure policy blocks, got: {error}")
            });

        assert_eq!(output.result["ok"], json!(false));
        assert_eq!(output.result["status"], json!("blocked"));
        assert_eq!(output.result["error"]["kind"], json!("policy_blocked"));
    }

    #[tokio::test]
    async fn web_search_uses_fallback_provider_and_reports_metadata() {
        let tool = WebSearchTool::with_config(WebSearchConfig {
            provider: SearchProviderPreference::Provider(SearchProvider::Tavily),
            fallback_provider: Some(SearchProvider::Brave),
            max_results: 5,
            timeout: Duration::from_secs(DEFAULT_SEARCH_TIMEOUT_SECS),
            tavily_api_key: None,
            brave_api_key: Some("brave-test-key".to_string()),
            tavily_url: "https://api.tavily.com/search".to_string(),
            brave_url: "https://api.search.brave.com/res/v1/web/search".to_string(),
        });
        let ctx = JobContext {
            http_interceptor: Some(Arc::new(ReplayingHttpInterceptor::new(vec![HttpExchange {
                request: HttpExchangeRequest {
                    method: "GET".to_string(),
                    url: "https://api.search.brave.com/res/v1/web/search?q=rust&count=2"
                        .to_string(),
                    headers: vec![],
                    body: None,
                },
                response: HttpExchangeResponse {
                    status: 200,
                    headers: vec![("content-type".to_string(), "application/json".to_string())],
                    body: json!({
                        "web": {
                            "results": [
                                {
                                    "title": "Rust Language",
                                    "url": "https://www.rust-lang.org/",
                                    "description": "Fast and reliable systems programming language."
                                }
                            ]
                        }
                    })
                    .to_string(),
                },
            }]))),
            ..Default::default()
        };

        let output = tool
            .execute(json!({"query": "rust", "count": 2}), &ctx)
            .await
            .unwrap_or_else(|error| panic!("web_search failed unexpectedly: {error}"));

        assert_eq!(output.result["ok"], json!(true));
        assert_eq!(output.result["status"], json!("degraded"));
        assert_eq!(output.result["provider_used"], json!("brave"));
        assert_eq!(output.result["fallback_used"], json!(true));
        assert_eq!(output.result["results"][0]["title"], json!("Rust Language"));
    }

    #[tokio::test]
    async fn web_search_returns_blocked_when_no_provider_key_exists() {
        let tool = WebSearchTool::with_config(WebSearchConfig {
            provider: SearchProviderPreference::Provider(SearchProvider::Tavily),
            fallback_provider: None,
            max_results: 5,
            timeout: Duration::from_secs(DEFAULT_SEARCH_TIMEOUT_SECS),
            tavily_api_key: None,
            brave_api_key: None,
            tavily_url: "https://api.tavily.com/search".to_string(),
            brave_url: "https://api.search.brave.com/res/v1/web/search".to_string(),
        });

        let output = tool
            .execute(json!({"query": "rust"}), &JobContext::default())
            .await
            .unwrap_or_else(|error| {
                panic!("web_search should structure auth blocks, got: {error}")
            });

        assert_eq!(output.result["ok"], json!(false));
        assert_eq!(output.result["status"], json!("blocked"));
        assert_eq!(output.result["error"]["kind"], json!("auth_missing"));
    }
}
