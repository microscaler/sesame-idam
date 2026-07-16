//! # Pact Infrastructure Manager
//!
//! Sidecar container that manages Pact infrastructure lifecycle:
//! - Watches for Pact broker to start and port to be available
//! - Watches ConfigMap for changes and re-publishes pacts
//! - Publishes Pact contracts to broker once ready
//! - Monitors pod/container status

use anyhow::{Context, Result};
use axum::{extract::State, http::StatusCode, response::Json, routing::get, Router};
use futures::{pin_mut, StreamExt};
use k8s_openapi::api::core::v1::{ConfigMap, Pod};
use kube::{api::Api, Client};
use kube_runtime::watcher::{self, Config};
use serde_json::{json, Value};
use std::{
    collections::{HashMap, HashSet},
    path::Path,
    process::Command,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

/// Configuration for the manager
#[derive(Debug, Clone)]
struct ManagerConfig {
    broker_url: String,
    username: String,
    password: String,
    namespace: String,
    configmap_name: String,
    configmap_path: String,
    published_flag_path: String,
    git_branch: String,
    git_commit: String,
    broker_port: u16,
    broker_health_path: String,
    check_interval: Duration,
    broker_timeout: Duration,
}

impl ManagerConfig {
    fn from_env() -> Result<Self> {
        Ok(Self {
            broker_url: std::env::var("BROKER_URL")
                .unwrap_or_else(|_| "http://localhost:9292".to_string()),
            username: std::env::var("BROKER_USERNAME").unwrap_or_else(|_| "pact".to_string()),
            password: std::env::var("BROKER_PASSWORD").unwrap_or_else(|_| "pact".to_string()),
            namespace: std::env::var("NAMESPACE")
                .unwrap_or_else(|_| "secret-manager-controller-pact-broker".to_string()),
            configmap_name: std::env::var("CONFIGMAP_NAME")
                .unwrap_or_else(|_| "pact-contracts".to_string()),
            configmap_path: std::env::var("CONFIGMAP_PATH")
                .unwrap_or_else(|_| "/pacts-configmap".to_string()),
            published_flag_path: std::env::var("PUBLISHED_FLAG_PATH")
                .unwrap_or_else(|_| "/tmp/pacts-published.flag".to_string()),
            git_branch: std::env::var("GIT_BRANCH").unwrap_or_else(|_| "main".to_string()),
            git_commit: std::env::var("GIT_COMMIT").unwrap_or_else(|_| {
                format!(
                    "dev-{}",
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs()
                )
            }),
            broker_port: std::env::var("BROKER_PORT")
                .unwrap_or_else(|_| "9292".to_string())
                .parse()
                .context("Invalid BROKER_PORT")?,
            broker_health_path: std::env::var("BROKER_HEALTH_PATH")
                .unwrap_or_else(|_| "/diagnostic/status/heartbeat".to_string()),
            check_interval: Duration::from_secs(
                std::env::var("CHECK_INTERVAL_SECS")
                    .unwrap_or_else(|_| "2".to_string())
                    .parse()
                    .context("Invalid CHECK_INTERVAL_SECS")?,
            ),
            broker_timeout: Duration::from_secs(
                std::env::var("BROKER_TIMEOUT_SECS")
                    .unwrap_or_else(|_| "300".to_string())
                    .parse()
                    .context("Invalid BROKER_TIMEOUT_SECS")?,
            ),
        })
    }
}

/// Check if the Pact broker is ready by checking the health endpoint
async fn check_broker_ready(config: &ManagerConfig) -> Result<bool> {
    let client = reqwest::Client::new();
    let url = format!("{}{}", config.broker_url, config.broker_health_path);

    match client
        .get(&url)
        .basic_auth(&config.username, Some(&config.password))
        .timeout(Duration::from_secs(5))
        .send()
        .await
    {
        Ok(response) => Ok(response.status().is_success()),
        Err(e) => {
            warn!("Broker health check failed: {}", e);
            Ok(false)
        }
    }
}

/// Check if a port is available (listening)
async fn check_port_available(host: &str, port: u16) -> Result<bool> {
    use tokio::net::TcpStream;
    use tokio::time::timeout;

    // Try to connect to the port with a short timeout
    let addr = format!("{}:{}", host, port);
    match timeout(Duration::from_secs(2), TcpStream::connect(&addr)).await {
        Ok(Ok(_)) => Ok(true),
        Ok(Err(_)) | Err(_) => Ok(false),
    }
}

/// Check if a pod is running (reserved for future use).
#[allow(dead_code)]
async fn check_pod_running(client: &Client, namespace: &str, pod_name: &str) -> Result<bool> {
    let pods: Api<Pod> = Api::namespaced(client.clone(), namespace);
    match pods.get(pod_name).await {
        Ok(pod) => {
            let phase = pod.status.as_ref().and_then(|s| s.phase.as_deref());
            Ok(phase == Some("Running"))
        }
        Err(kube::Error::Api(e)) if e.code == 404 => {
            warn!("Pod {} not found", pod_name);
            Ok(false)
        }
        Err(e) => {
            warn!("Error checking pod {}: {}", pod_name, e);
            Ok(false)
        }
    }
}

/// Get list of providers and their Pact files from ConfigMap
fn get_providers_from_configmap(configmap_path: &Path) -> Result<HashMap<String, String>> {
    let mut providers = HashMap::new();

    // Provider name -> Pact file mapping
    let provider_mappings = vec![
        ("Searates-Terminal-Data", "Searates-Terminal-Data.json"),
        (
            "Searates-Supported-Terminals",
            "Searates-Supported-Terminals.json",
        ),
        ("Searates-Rates", "Searates-Rates.json"),
        ("Sesame-SSO-Broker", "Sesame-SSO-Broker.json"),
        ("Google-OAuth-Mock", "Sesame-OAuth-Google.json"),
        ("Microsoft-OAuth-Mock", "Sesame-OAuth-Microsoft.json"),
    ];

    for (provider_name, pact_file) in provider_mappings {
        let pact_path = configmap_path.join(pact_file);
        if pact_path.exists() {
            providers.insert(provider_name.to_string(), pact_file.to_string());
        }
    }

    Ok(providers)
}

/// Publish a single Pact contract
fn publish_pact(config: &ManagerConfig, provider_name: &str, pact_file: &str) -> Result<bool> {
    let pact_path = Path::new(&config.configmap_path).join(pact_file);
    if !pact_path.exists() {
        warn!(
            "Pact file not found: {} (skipping {})",
            pact_path.display(),
            provider_name
        );
        return Ok(false);
    }

    // Convert provider name to lowercase for version
    let provider_version = format!(
        "{}-{}-{}",
        provider_name.to_lowercase(),
        config.git_branch,
        config.git_commit
    );

    info!(
        "📤 Publishing pact contract: provider={}, file={}, version={}",
        provider_name, pact_file, provider_version
    );
    info!("   Broker URL: {}", config.broker_url);
    info!("   Pact file path: {}", pact_path.display());

    // Check if pact-broker CLI is available
    if which::which("pact-broker").is_err() {
        error!("❌ pact-broker CLI not found in PATH");
        error!("   Please ensure pact-broker-client gem is installed");
        error!(
            "   PATH: {}",
            std::env::var("PATH").unwrap_or_else(|_| "not set".to_string())
        );
        return Ok(false);
    }

    let output = Command::new("pact-broker")
        .arg("publish")
        .arg(pact_path.as_os_str())
        .arg("--consumer-app-version")
        .arg(&provider_version)
        .arg("--branch")
        .arg(&config.git_branch)
        .arg("--broker-base-url")
        .arg(&config.broker_url)
        .arg("--broker-username")
        .arg(&config.username)
        .arg("--broker-password")
        .arg(&config.password)
        .output()
        .with_context(|| {
            format!(
                "Failed to execute pact-broker publish command (provider={}, file={})",
                provider_name, pact_file
            )
        })?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        info!(
            "✅ Successfully published pact contract: provider={}, version={}",
            provider_name, provider_version
        );
        if !stdout.trim().is_empty() {
            info!("   Broker response: {}", stdout.trim());
        }
        Ok(true)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        error!(
            "❌ Failed to publish pact contract: provider={}, version={}",
            provider_name, provider_version
        );
        if !stderr.trim().is_empty() {
            error!("   Error details: {}", stderr.trim());
        } else {
            error!("   No error details available (stderr empty)");
        }
        if !stdout.trim().is_empty() {
            error!("   Output: {}", stdout.trim());
        }
        error!("   Exit code: {:?}", output.status.code());
        Ok(false)
    }
}

/// Publish all pacts from the ConfigMap
#[allow(dead_code)] // Kept for potential future use, but currently using publish_all_pacts_with_tracking
fn publish_all_pacts(config: &ManagerConfig) -> Result<(usize, usize)> {
    let configmap_path = Path::new(&config.configmap_path);
    let providers = get_providers_from_configmap(configmap_path)
        .context("Failed to get providers from ConfigMap")?;

    if providers.is_empty() {
        warn!(
            "⚠️  No Pact files found in ConfigMap at {}",
            config.configmap_path
        );
        return Ok((0, 0));
    }

    let mut published = 0;
    let mut failed = 0;

    for (provider_name, pact_file) in providers {
        match publish_pact(config, &provider_name, &pact_file) {
            Ok(true) => published += 1,
            Ok(false) => failed += 1,
            Err(e) => {
                error!("Error publishing {}: {}", provider_name, e);
                failed += 1;
            }
        }
    }

    info!(
        "📊 Publishing Summary: ✅ Published: {}, ❌ Failed: {}",
        published, failed
    );

    Ok((published, failed))
}

/// Publish all pacts and track which providers were published
async fn publish_all_pacts_with_tracking(
    config: &ManagerConfig,
    published_providers: &Arc<tokio::sync::RwLock<HashSet<String>>>,
) -> Result<(usize, usize)> {
    let configmap_path = Path::new(&config.configmap_path);
    let providers = get_providers_from_configmap(configmap_path)
        .context("Failed to get providers from ConfigMap")?;

    if providers.is_empty() {
        warn!(
            "⚠️  No Pact files found in ConfigMap at {}",
            config.configmap_path
        );
        return Ok((0, 0));
    }

    let mut published = 0;
    let mut failed = 0;
    let mut published_set = HashSet::new();
    let total = providers.len();

    info!("📋 Found {} pact contract(s) to publish", total);

    for (provider_name, pact_file) in providers {
        info!("");
        info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        match publish_pact(config, &provider_name, &pact_file) {
            Ok(true) => {
                published += 1;
                published_set.insert(provider_name.clone());
                info!(
                    "✅ [{}/{}] Successfully published: {}",
                    published + failed,
                    total,
                    provider_name
                );
            }
            Ok(false) => {
                failed += 1;
                info!(
                    "❌ [{}/{}] Failed to publish: {}",
                    published + failed,
                    total,
                    provider_name
                );
            }
            Err(e) => {
                failed += 1;
                error!(
                    "❌ [{}/{}] Error publishing {}: {}",
                    published + failed,
                    total,
                    provider_name,
                    e
                );
            }
        }
        info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    }

    // Update the published providers set
    {
        let mut providers_lock = published_providers.write().await;
        providers_lock.clear();
        providers_lock.extend(published_set.iter().cloned());
    }

    info!("");
    info!("📊 Publishing Summary:");
    info!("   Total contracts: {}", total);
    info!(
        "   ✅ Successfully published: {} ({})",
        published,
        published_set
            .iter()
            .map(|p| p.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    );
    info!("   ❌ Failed: {}", failed);
    if published > 0 {
        info!("   📦 Published providers: {:?}", published_set);
    }

    Ok((published, failed))
}

/// Wait for broker to be ready
async fn wait_for_broker(config: &ManagerConfig) -> Result<()> {
    let start_time = Instant::now();
    let host = config
        .broker_url
        .strip_prefix("http://")
        .or_else(|| config.broker_url.strip_prefix("https://"))
        .unwrap_or(&config.broker_url)
        .split(':')
        .next()
        .unwrap_or("localhost");

    info!("👀 Waiting for Pact broker at {}...", config.broker_url);

    loop {
        // Check if we've exceeded timeout
        if start_time.elapsed() > config.broker_timeout {
            return Err(anyhow::anyhow!(
                "Timeout waiting for broker after {} seconds",
                config.broker_timeout.as_secs()
            ));
        }

        // First check if port is available
        if check_port_available(host, config.broker_port).await? {
            // Port is available, check if broker is ready
            if check_broker_ready(config).await? {
                info!("✅ Broker is ready and port is available!");
                return Ok(());
            }
        }

        sleep(config.check_interval).await;
    }
}

/// Check if ConfigMap exists and has pact files
/// Returns Ok(true) if ConfigMap exists and has pacts
/// Returns Ok(false) if ConfigMap exists but has no pacts
/// Returns Err if there's an error (including RBAC/401 - caller should retry)
async fn check_configmap_has_pacts(
    client: &Client,
    namespace: &str,
    configmap_name: &str,
) -> Result<bool> {
    let configmaps: Api<ConfigMap> = Api::namespaced(client.clone(), namespace);
    match configmaps.get(configmap_name).await {
        Ok(cm) => {
            // Check if ConfigMap has data with pact files
            if let Some(data) = &cm.data {
                // Check for at least one pact file
                let has_pacts = data.keys().any(|key| key.ends_with(".json"));
                Ok(has_pacts)
            } else {
                Ok(false)
            }
        }
        Err(kube::Error::Api(e)) if e.code == 404 => {
            // ConfigMap doesn't exist yet - this is OK, not an error
            Ok(false)
        }
        Err(kube::Error::Api(e)) if e.code == 401 || e.code == 403 => {
            // RBAC not ready yet - return error so caller can retry
            Err(anyhow::anyhow!(
                "Unauthorized ({}): RBAC may not be ready yet - will retry",
                e.code
            ))
        }
        Err(e) => {
            // Other errors - return as error
            Err(anyhow::anyhow!(
                "Error checking ConfigMap {}/{}: {}",
                namespace,
                configmap_name,
                e
            ))
        }
    }
}

/// Wait for RBAC to be ready by checking ConfigMap access with retries
async fn wait_for_rbac_ready(
    client: &Client,
    namespace: &str,
    configmap_name: &str,
    max_attempts: u32,
) -> Result<()> {
    let mut attempt = 0;
    let mut delay = Duration::from_secs(2); // Start with 2 second delay

    while attempt < max_attempts {
        match check_configmap_has_pacts(client, namespace, configmap_name).await {
            Ok(_) => {
                // Successfully accessed ConfigMap - RBAC is ready
                info!("✅ RBAC is ready - can access ConfigMap");
                return Ok(());
            }
            Err(e)
                if e.to_string().contains("Unauthorized")
                    || e.to_string().contains("401")
                    || e.to_string().contains("403") =>
            {
                // RBAC not ready yet - retry with exponential backoff
                attempt += 1;
                if attempt % 5 == 0 {
                    info!(
                        "⏳ Waiting for RBAC to be ready... (attempt {}/{})",
                        attempt, max_attempts
                    );
                }
                sleep(delay).await;
                // Exponential backoff: 2s, 4s, 8s, 16s, max 30s
                delay = std::cmp::min(delay * 2, Duration::from_secs(30));
            }
            Err(e) => {
                // Other error - might be transient, retry a few times
                attempt += 1;
                if attempt < 3 {
                    warn!("⚠️  Error checking ConfigMap (will retry): {}", e);
                    sleep(delay).await;
                } else {
                    // After 3 attempts, give up on this check but continue
                    warn!("⚠️  Error checking ConfigMap (continuing anyway): {}", e);
                    return Ok(());
                }
            }
        }
    }

    warn!(
        "⚠️  RBAC check timed out after {} attempts, but continuing - watcher will retry",
        max_attempts
    );
    Ok(())
}

/// Watch ConfigMap for changes and republish pacts
/// Uses minimal kube-rs watcher with field selector for efficiency
async fn watch_configmap(
    client: Client,
    namespace: String,
    configmap_name: String,
    config: Arc<ManagerConfig>,
    pacts_published: Arc<AtomicBool>,
    published_providers: Arc<tokio::sync::RwLock<HashSet<String>>>,
) -> Result<()> {
    // Wait for RBAC to be ready before starting watch
    info!("🔐 Waiting for RBAC to be ready before starting ConfigMap watch...");
    if let Err(e) = wait_for_rbac_ready(&client, &namespace, &configmap_name, 30).await {
        warn!(
            "⚠️  RBAC readiness check failed: {} (will continue and retry)",
            e
        );
    }

    let configmaps: Api<ConfigMap> = Api::namespaced(client.clone(), &namespace);

    // Use field selector to only watch our specific ConfigMap (more efficient)
    let watcher_config = Config::default().fields(&format!("metadata.name={}", configmap_name));

    let watcher = watcher::watcher(configmaps, watcher_config);
    pin_mut!(watcher);

    info!(
        "👀 Watching ConfigMap {}/{} for changes...",
        namespace, configmap_name
    );

    // Verify ConfigMap exists before starting watch (with retry for RBAC)
    let mut check_attempts = 0;
    loop {
        match check_configmap_has_pacts(&client, &namespace, &configmap_name).await {
            Ok(true) => {
                info!(
                    "✅ ConfigMap {}/{} exists and contains pact files",
                    namespace, configmap_name
                );
                break;
            }
            Ok(false) => {
                info!(
                    "ℹ️  ConfigMap {}/{} exists but has no pact files yet (this is expected - pact files will be added when tests run)",
                    namespace, configmap_name
                );
                break;
            }
            Err(e)
                if e.to_string().contains("Unauthorized")
                    || e.to_string().contains("401")
                    || e.to_string().contains("403") =>
            {
                // RBAC not ready - retry
                check_attempts += 1;
                if check_attempts < 10 {
                    if check_attempts % 3 == 0 {
                        info!(
                            "⏳ RBAC not ready yet, retrying ConfigMap check... (attempt {})",
                            check_attempts
                        );
                    }
                    sleep(Duration::from_secs(2)).await;
                } else {
                    warn!(
                        "⚠️  RBAC still not ready after {} attempts, continuing anyway - watcher will handle retries",
                        check_attempts
                    );
                    break;
                }
            }
            Err(e) => {
                warn!(
                    "⚠️  Error checking ConfigMap: {} (will continue watching)",
                    e
                );
                break;
            }
        }
    }

    while let Some(event_result) = watcher.next().await {
        match event_result {
            Ok(event) => {
                match event {
                    kube::runtime::watcher::Event::Apply(cm) => {
                        // Field selector ensures this is our ConfigMap, but double-check
                        if cm.metadata.name.as_deref() == Some(&configmap_name) {
                            info!(
                                "📝 ConfigMap {} changed, checking for pact files...",
                                configmap_name
                            );

                            // Small delay to ensure mounted volume is updated
                            sleep(Duration::from_millis(500)).await;

                            // Check if broker is ready
                            if let Ok(true) = check_broker_ready(&config).await {
                                // Remove published flag to force re-publishing
                                let _ = std::fs::remove_file(&config.published_flag_path);

                                match publish_all_pacts_with_tracking(&config, &published_providers)
                                    .await
                                {
                                    Ok((published, failed)) => {
                                        if published > 0 {
                                            // Set flag to indicate pacts were published
                                            let _ = std::fs::write(&config.published_flag_path, "");
                                            pacts_published.store(true, Ordering::Relaxed);
                                            info!(
                                                "✅ Re-published {} pact(s) from ConfigMap",
                                                published
                                            );
                                        } else {
                                            pacts_published.store(false, Ordering::Relaxed);
                                            warn!(
                                                "⚠️  No pacts were published (no pact files found in ConfigMap)"
                                            );
                                        }
                                        if failed > 0 {
                                            warn!("⚠️  {} pact(s) failed to publish", failed);
                                        }
                                    }
                                    Err(e) => {
                                        pacts_published.store(false, Ordering::Relaxed);
                                        error!("Error re-publishing pacts: {}", e);
                                    }
                                }
                            } else {
                                warn!(
                                    "Broker not ready, skipping re-publish (will retry on next change)"
                                );
                            }
                        }
                    }
                    kube::runtime::watcher::Event::Delete(cm) => {
                        if cm.metadata.name.as_deref() == Some(&configmap_name) {
                            warn!("⚠️  ConfigMap {} was deleted", configmap_name);
                            pacts_published.store(false, Ordering::Relaxed);
                            // Clear published providers
                            let mut providers_lock = published_providers.write().await;
                            providers_lock.clear();
                            // Remove published flag since ConfigMap is gone
                            let _ = std::fs::remove_file(&config.published_flag_path);
                        }
                    }
                    kube::runtime::watcher::Event::Init
                    | kube::runtime::watcher::Event::InitApply(_)
                    | kube::runtime::watcher::Event::InitDone => {
                        // Initial watch events - ignore (we already published on startup)
                        debug!("Initial watch event received");
                    }
                }
            }
            Err(e) => {
                let error_str = e.to_string();
                // Check if this is an RBAC/authorization error
                if error_str.contains("Unauthorized")
                    || error_str.contains("401")
                    || error_str.contains("403")
                {
                    // RBAC might have been revoked or not ready - wait longer and retry
                    warn!(
                        "⚠️  RBAC error watching ConfigMap (will retry with backoff): {}",
                        e
                    );
                    sleep(Duration::from_secs(10)).await;
                } else {
                    error!("Error watching ConfigMap: {}", e);
                    // Continue watching - stream will retry automatically
                    // Add a small delay before continuing to avoid tight error loop
                    sleep(Duration::from_secs(5)).await;
                }
            }
        }
    }

    warn!("ConfigMap watch stream ended - this should not happen");
    Ok(())
}

/// Wait for the broker, retrying after each timeout instead of exiting the process.
async fn wait_for_broker_with_retries(config: &ManagerConfig, broker_healthy: &AtomicBool) {
    let retry_delay = Duration::from_secs(
        std::env::var("BROKER_RETRY_DELAY_SECS")
            .unwrap_or_else(|_| "30".to_string())
            .parse()
            .unwrap_or(30),
    );

    loop {
        match wait_for_broker(config).await {
            Ok(()) => {
                broker_healthy.store(true, Ordering::Relaxed);
                info!("✅ Broker is ready - health flag set");
                return;
            }
            Err(e) => {
                broker_healthy.store(false, Ordering::Relaxed);
                warn!(
                    "Broker not ready ({}). Retrying in {}s...",
                    e,
                    retry_delay.as_secs()
                );
                sleep(retry_delay).await;
            }
        }
    }
}

/// Publish pact contracts from the mounted ConfigMap (idempotent via flag file).
async fn publish_initial_pacts(
    config: &ManagerConfig,
    pacts_published: &AtomicBool,
    published_providers: &Arc<tokio::sync::RwLock<HashSet<String>>>,
) -> Result<()> {
    let published_flag = Path::new(&config.published_flag_path);
    if published_flag.exists() {
        let providers =
            get_providers_from_configmap(Path::new(&config.configmap_path)).unwrap_or_default();
        let mut providers_set = published_providers.write().await;
        for (provider_name, _) in providers {
            providers_set.insert(provider_name);
        }
        pacts_published.store(true, Ordering::Relaxed);
        info!("ℹ️  Pacts already published (flag exists), skipping initial publish");
        return Ok(());
    }

    info!("📦 Publishing Pact contracts...");
    match publish_all_pacts_with_tracking(config, published_providers).await {
        Ok((published, failed)) => {
            if published > 0 {
                let _ = std::fs::write(&config.published_flag_path, "");
                pacts_published.store(true, Ordering::Relaxed);
                info!("✅ All contracts published! Manager will continue running.");
            } else if failed > 0 {
                pacts_published.store(false, Ordering::Relaxed);
                warn!("⚠️  No contracts were published (all failed)");
            } else {
                pacts_published.store(false, Ordering::Relaxed);
                warn!("⚠️  No contracts were published (no Pact files found)");
            }
        }
        Err(e) => {
            pacts_published.store(false, Ordering::Relaxed);
            error!("Error publishing pacts: {}", e);
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    info!("🚀 Starting Pact infrastructure manager sidecar...");
    info!("📋 Manager responsibilities:");
    info!("   - Watch for Pact broker to start and port to be available");
    info!("   - Publish Pact contracts to broker once ready");
    info!("   - Watch ConfigMap for changes and re-publish pacts");
    info!("");

    let config = ManagerConfig::from_env().context("Failed to load configuration")?;
    let config = Arc::new(config);

    // Shared state for health checks (broker may not be up yet on cold cluster start).
    let broker_healthy = Arc::new(AtomicBool::new(false));
    let pacts_published = Arc::new(AtomicBool::new(false));
    let published_providers = Arc::new(tokio::sync::RwLock::new(HashSet::new()));

    let health_state = HealthState {
        broker_healthy: broker_healthy.clone(),
        pacts_published: pacts_published.clone(),
        published_providers: published_providers.clone(),
    };

    let app = Router::new()
        .route("/health", get(health_handler))
        .route("/healthz", get(health_handler))
        .route("/liveness", get(liveness_handler))
        .route("/readiness", get(readiness_handler))
        .route("/ready", get(ready_handler))
        .with_state(health_state);

    let health_port = std::env::var("HEALTH_PORT")
        .unwrap_or_else(|_| "1238".to_string())
        .parse()
        .unwrap_or(1238);

    info!("🏥 Starting health server on port {}...", health_port);
    let server_handle = tokio::spawn(async move {
        let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", health_port))
            .await
            .context("Failed to bind health server")?;
        axum::serve(listener, app)
            .await
            .context("Health server error")?;
        Ok::<(), anyhow::Error>(())
    });

    // Broker wait + initial publish run in the background so probes stay alive.
    let config_for_broker = config.clone();
    let broker_healthy_bg = broker_healthy.clone();
    let pacts_published_bg = pacts_published.clone();
    let published_providers_bg = published_providers.clone();
    tokio::spawn(async move {
        wait_for_broker_with_retries(&config_for_broker, &broker_healthy_bg).await;
        if let Err(e) = publish_initial_pacts(
            &config_for_broker,
            &pacts_published_bg,
            &published_providers_bg,
        )
        .await
        {
            error!("Initial pact publish failed: {}", e);
        }
    });

    // Initialize rustls CryptoProvider before creating Kubernetes client
    // This is required when using rustls with kube-rs
    rustls::crypto::CryptoProvider::install_default(rustls::crypto::ring::default_provider())
        .expect("Failed to install rustls crypto provider");

    // Create Kubernetes client for watching ConfigMap
    let client = Client::try_default()
        .await
        .context("Failed to create Kubernetes client")?;

    // Wait for RBAC to be ready before checking ConfigMap
    info!(
        "🔍 Checking for ConfigMap {}/{}...",
        config.namespace, config.configmap_name
    );

    // Wait for RBAC with retries
    let mut check_attempts = 0;
    loop {
        match check_configmap_has_pacts(&client, &config.namespace, &config.configmap_name).await {
            Ok(true) => {
                info!("✅ ConfigMap found with pact files");
                break;
            }
            Ok(false) => {
                info!(
                    "ℹ️  ConfigMap exists but has no pact files yet (this is expected - will publish when pact files are added to ConfigMap)"
                );
                break;
            }
            Err(e)
                if e.to_string().contains("Unauthorized")
                    || e.to_string().contains("401")
                    || e.to_string().contains("403") =>
            {
                // RBAC not ready - retry with exponential backoff
                check_attempts += 1;
                if check_attempts < 15 {
                    if check_attempts % 3 == 0 {
                        info!(
                            "⏳ Waiting for RBAC to be ready... (attempt {})",
                            check_attempts
                        );
                    }
                    let delay = std::cmp::min(
                        Duration::from_secs(2).saturating_mul(check_attempts),
                        Duration::from_secs(10),
                    );
                    sleep(delay).await;
                } else {
                    warn!(
                        "⚠️  RBAC still not ready after {} attempts, continuing anyway - watcher will handle retries",
                        check_attempts
                    );
                    break;
                }
            }
            Err(e) => {
                warn!(
                    "⚠️  Error checking ConfigMap: {} (will continue watching)",
                    e
                );
                break;
            }
        }
    }

    // Start watching ConfigMap in background
    let config_clone = config.clone();
    let namespace = config.namespace.clone();
    let configmap_name = config.configmap_name.clone();
    let pacts_published_clone = pacts_published.clone();
    let published_providers_clone = published_providers.clone();
    tokio::spawn(async move {
        if let Err(e) = watch_configmap(
            client,
            namespace,
            configmap_name,
            config_clone,
            pacts_published_clone,
            published_providers_clone,
        )
        .await
        {
            error!("ConfigMap watcher error: {}", e);
        }
    });

    // Start health check background task
    let broker_healthy_clone = broker_healthy.clone();
    let config_for_health = config.clone();
    tokio::spawn(async move {
        loop {
            match check_broker_ready(&config_for_health).await {
                Ok(true) => {
                    broker_healthy_clone.store(true, Ordering::Relaxed);
                }
                Ok(false) => {
                    broker_healthy_clone.store(false, Ordering::Relaxed);
                    warn!("⚠️  Broker health check failed");
                }
                Err(e) => {
                    broker_healthy_clone.store(false, Ordering::Relaxed);
                    warn!("⚠️  Broker health check error: {}", e);
                }
            }
            sleep(Duration::from_secs(30)).await;
        }
    });

    // Keep the manager running
    info!("✅ Manager is running. Monitoring for ConfigMap changes...");
    info!("   Health endpoint: http://0.0.0.0:{}/healthz", health_port);

    // Wait for server or ConfigMap watcher to exit
    tokio::select! {
        result = server_handle => {
            if let Err(e) = result {
                error!("Health server error: {}", e);
            }
        }
    }

    Ok(())
}

/// Health state shared between HTTP handlers and background tasks
#[derive(Clone)]
struct HealthState {
    broker_healthy: Arc<AtomicBool>,
    pacts_published: Arc<AtomicBool>,
    published_providers: Arc<tokio::sync::RwLock<HashSet<String>>>,
}

/// Health check handler - returns 200 if manager is running
async fn health_handler(State(state): State<HealthState>) -> (StatusCode, Json<Value>) {
    let response = json!({
        "status": "healthy",
        "broker_healthy": state.broker_healthy.load(Ordering::Relaxed),
        "pacts_published": state.pacts_published.load(Ordering::Relaxed),
    });

    // Consider healthy if manager is running (broker health is informational)
    (StatusCode::OK, Json(response))
}

/// Liveness probe handler - returns 200 if manager process is alive
/// Used by Kubernetes liveness probes
async fn liveness_handler(State(_state): State<HealthState>) -> (StatusCode, Json<Value>) {
    let response = json!({
        "status": "alive",
    });
    (StatusCode::OK, Json(response))
}

/// Readiness probe handler - returns 200 if manager is ready (broker healthy)
/// Used by Kubernetes readiness probes
/// Manager is ready if broker is healthy, even if pacts aren't published yet
async fn readiness_handler(State(state): State<HealthState>) -> (StatusCode, Json<Value>) {
    let broker_healthy = state.broker_healthy.load(Ordering::Relaxed);
    let response = json!({
        "status": if broker_healthy { "ready" } else { "not_ready" },
        "broker_healthy": broker_healthy,
    });

    // Manager is ready for Kubernetes if broker is healthy
    // This allows the manager to pass readiness probes even before pacts are available
    if broker_healthy {
        (StatusCode::OK, Json(response))
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, Json(response))
    }
}

/// Ready handler - returns 200 if manager is ready AND pacts are published
/// Used by mock servers to check if their specific pacts are available
async fn ready_handler(State(state): State<HealthState>) -> (StatusCode, Json<Value>) {
    let broker_healthy = state.broker_healthy.load(Ordering::Relaxed);
    let pacts_published = state.pacts_published.load(Ordering::Relaxed);
    let published_providers = state.published_providers.read().await;
    let providers_list: Vec<String> = published_providers.iter().cloned().collect();

    let response = json!({
        "status": if broker_healthy && pacts_published { "ready" } else { "not_ready" },
        "broker_healthy": broker_healthy,
        "pacts_published": pacts_published,
        "published_providers": providers_list,
    });

    // Ready if broker is healthy AND pacts are published
    // This is what mock servers check to know if they can start
    if broker_healthy && pacts_published {
        (StatusCode::OK, Json(response))
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, Json(response))
    }
}
