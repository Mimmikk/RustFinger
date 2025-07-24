use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::Json,
    routing::get,
    Router,
};
use serde::Deserialize;
use std::{collections::HashMap, net::SocketAddr, sync::Arc};
use tokio::signal;
use tower_http::cors::CorsLayer;
use tracing::{info, warn, debug};

mod config;
mod log;

use config::{Config, WebFinger, TenantData};

#[derive(Deserialize)]
struct WebFingerQuery {
    resource: String,
}

type TenantMap = HashMap<String, TenantData>;

async fn webfinger_handler(
    headers: HeaderMap,
    Query(params): Query<WebFingerQuery>,
    State(tenants): State<Arc<TenantMap>>,
) -> Result<Json<WebFinger>, StatusCode> {
    let resource = params.resource;
    
    // Get the host from headers
    let host = headers
        .get("host")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("localhost");
    
    // Remove port if present
    let domain = host.split(':').next().unwrap_or(host);
    
    debug!("WebFinger request: resource={}, domain={}", resource, domain);
    
    // Find the tenant for this domain
    let tenant = tenants.values()
        .find(|t| t.domain == domain)
        .ok_or_else(|| {
            warn!("No tenant found for domain: {}", domain);
            StatusCode::NOT_FOUND
        })?;
    
    // Look for exact user match first
    if let Some(finger) = tenant.fingers.get(&resource) {
        return Ok(Json(finger.clone()));
    }
    
    // Handle global domain matching for users
    if tenant.global {
        // Extract domain from resource (e.g., "acct:user@domain.com" -> "domain.com")
        if let Some(resource_domain) = extract_domain_from_resource(&resource) {
            if resource_domain == domain {
                if let Some(finger) = tenant.fingers.get(&format!("acct:*@{}", domain)) {
                    // Create a personalized response for the specific user
                    let mut personalized = finger.clone();
                    personalized.subject = resource;
                    return Ok(Json(personalized));
                }
            }
        }
    }
    
    warn!("WebFinger resource not found: {} for domain {}", resource, domain);
    Err(StatusCode::NOT_FOUND)
}

fn extract_domain_from_resource(resource: &str) -> Option<&str> {
    if resource.starts_with("acct:") {
        let email_part = &resource[5..]; // Remove "acct:" prefix
        email_part.split('@').nth(1) // Get domain part
    } else {
        None
    }
}

async fn health_handler() -> &'static str {
    "OK"
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    info!("Shutdown signal received");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize minimal logging
    log::init_logging();

    // Load configuration
    let config = Config::load().await?;
    info!("Loaded {} tenants with {} total webfingers", 
          config.tenants.len(),
          config.tenants.values().map(|t| t.fingers.len()).sum::<usize>());
    
    // Log tenant details for debugging
    for (name, tenant) in &config.tenants {
        info!("Tenant '{}': domain='{}', global={}, webfingers={}", 
              name, tenant.domain, tenant.global, tenant.fingers.len());
        for (resource, _) in &tenant.fingers {
            debug!("  - {}", resource);
        }
    }

    // Create shared state
    let tenants = Arc::new(config.tenants);

    // Build the router
    let app = Router::new()
        .route("/.well-known/webfinger", get(webfinger_handler))
        .route("/healthz", get(health_handler))
        .layer(CorsLayer::permissive())
        .with_state(tenants);

    // Bind to address
    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    info!("Starting server on {}", addr);

    // Start the server with graceful shutdown
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    info!("Server shutdown complete");
    Ok(())
}
