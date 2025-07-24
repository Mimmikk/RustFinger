use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use url::Url;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Link {
    pub rel: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub href: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WebFinger {
    pub subject: String,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub links: Vec<Link>,
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    pub properties: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
pub struct TenantConfig {
    pub domain: String,
    #[serde(default)]
    pub users: HashMap<String, HashMap<String, String>>,
    #[serde(default)]
    pub global: bool,
    #[serde(default)]
    pub openid: Option<String>,
}

type URNAliases = HashMap<String, String>;
type TenantsConfig = HashMap<String, TenantConfig>;

#[derive(Debug)]
pub struct TenantData {
    pub domain: String,
    pub global: bool,
    pub fingers: HashMap<String, WebFinger>,
}

pub struct Config {
    pub tenants: HashMap<String, TenantData>,
}

impl Config {
    pub async fn load() -> Result<Self, Box<dyn std::error::Error>> {
        // Load URN aliases
        let urn_aliases = load_urn_aliases().await?;
        
        // Load tenant configurations from config directory
        let tenants = load_tenants().await?;
        
        // Process configurations into tenant data
        let tenant_data = process_tenants(tenants, urn_aliases)?;
        
        Ok(Config { tenants: tenant_data })
    }
}

async fn load_urn_aliases() -> Result<URNAliases, Box<dyn std::error::Error>> {
    let content = match tokio::fs::read_to_string("urns.yml").await {
        Ok(content) => content,
        Err(_) => return Ok(HashMap::new()), // Default empty if file doesn't exist
    };
    
    let aliases: URNAliases = serde_yaml::from_str(&content)?;
    Ok(aliases)
}

async fn load_tenants() -> Result<TenantsConfig, Box<dyn std::error::Error>> {
    let mut tenants = HashMap::new();
    
    // Try to read config directory
    let mut dir = match tokio::fs::read_dir("config").await {
        Ok(dir) => dir,
        Err(_) => return Ok(tenants), // Return empty if no config dir
    };
    
    // Read all .yml files in config directory
    while let Some(entry) = dir.next_entry().await? {
        let path = entry.path();
        if let Some(ext) = path.extension() {
            if ext == "yml" || ext == "yaml" {
                let content = tokio::fs::read_to_string(&path).await?;
                let tenant_config: TenantsConfig = serde_yaml::from_str(&content)?;
                tenants.extend(tenant_config);
            }
        }
    }
    
    Ok(tenants)
}

fn process_tenants(
    tenants: TenantsConfig,
    urn_aliases: URNAliases,
) -> Result<HashMap<String, TenantData>, Box<dyn std::error::Error>> {
    let mut tenant_map = HashMap::new();
    
    for (tenant_name, tenant_config) in tenants {
        let mut fingers = HashMap::new();
        
        // Process defined users for this tenant
        for (user_id, user_data) in tenant_config.users {
            let subject = normalize_subject(&user_id)?;
            let finger = create_webfinger(subject.clone(), user_data, &urn_aliases)?;
            fingers.insert(subject, finger);
        }
        
        // Handle global configuration (accept any user for the domain)
        if tenant_config.global {
            if let Some(openid) = tenant_config.openid {
                let global_data = [("openid".to_string(), openid)].into_iter().collect();
                let subject = format!("acct:*@{}", tenant_config.domain);
                let finger = create_webfinger(subject.clone(), global_data, &urn_aliases)?;
                fingers.insert(subject, finger);
            }
        }
        
        // Create tenant data
        let tenant_data = TenantData {
            domain: tenant_config.domain.clone(),
            global: tenant_config.global,
            fingers,
        };
        
        println!("Loaded tenant '{}' for domain '{}' with {} webfingers (global: {})", 
                 tenant_name, tenant_config.domain, tenant_data.fingers.len(), tenant_config.global);
        
        tenant_map.insert(tenant_name, tenant_data);
    }
    
    Ok(tenant_map)
}

fn normalize_subject(user_id: &str) -> Result<String, Box<dyn std::error::Error>> {
    let subject = if user_id.starts_with("acct:") {
        user_id[5..].to_string()
    } else {
        user_id.to_string()
    };
    
    // Validate as email or URL
    let email_regex = Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap();
    if email_regex.is_match(&subject) {
        Ok(format!("acct:{}", subject))
    } else if Url::parse(&subject).is_ok() {
        Ok(subject)
    } else {
        Err(format!("Invalid subject format: {}", user_id).into())
    }
}

fn create_webfinger(
    subject: String,
    user_data: HashMap<String, String>,
    urn_aliases: &URNAliases,
) -> Result<WebFinger, Box<dyn std::error::Error>> {
    let mut links = Vec::new();
    let mut properties = HashMap::new();
    
    for (key, value) in user_data {
        // Resolve URN alias if exists
        let urn = urn_aliases.get(&key).cloned().unwrap_or(key);
        
        // Check if value is a valid URL (add to links) or property
        if Url::parse(&value).is_ok() {
            links.push(Link {
                rel: urn,
                href: Some(value),
            });
        } else {
            properties.insert(urn, value);
        }
    }
    
    Ok(WebFinger {
        subject,
        links,
        properties,
    })
}
