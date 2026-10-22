//! Talking to an Odoo server: is it really Odoo, and which version?

use serde::Serialize;
use std::time::Duration;
use url::Url;

#[derive(Debug, Clone, Serialize)]
pub struct ServerInfo {
    /// Normalised origin, e.g. `https://odoo.example.org`.
    pub url: String,
    pub version: String,
    pub major: u32,
    /// Where Discuss lives on this version.
    pub discuss_path: String,
}

/// Turns what the user typed into an origin: adds `https://`, drops path, query and trailing slash.
pub fn normalize(input: &str) -> Result<Url, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err("Please type the address of your Odoo server.".into());
    }
    let with_scheme = if trimmed.contains("://") { trimmed.to_string() } else { format!("https://{trimmed}") };
    let mut url = Url::parse(&with_scheme).map_err(|_| "That does not look like a web address.".to_string())?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err("The address must start with http:// or https://".into());
    }
    if url.host_str().is_none() {
        return Err("That does not look like a web address.".into());
    }
    url.set_path("");
    url.set_query(None);
    url.set_fragment(None);
    Ok(url)
}

pub fn discuss_path_for(major: u32) -> &'static str {
    if major >= 18 {
        "/odoo/discuss"
    } else {
        "/web#action=mail.action_discuss"
    }
}

/// Reads `major` from Odoo's `server_version_info` (`[19, 0, 0, "final", 0, ""]`).
pub fn major_from_version_info(info: &serde_json::Value) -> Option<u32> {
    info.get(0)?.as_u64().map(|v| v as u32)
}

pub async fn probe(input: &str) -> Result<ServerInfo, String> {
    let typed_scheme = input.contains("://");
    let mut url = normalize(input)?;
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(12))
        .user_agent(concat!("Pigeodoon/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| e.to_string())?;

    let first = query_version(&client, &url).await;
    let result = match first {
        Err(ProbeError::Unreachable(_)) if !typed_scheme && url.scheme() == "https" => {
            // The user typed a bare host: be kind to servers that only speak http (e.g. a test machine).
            let mut http = url.clone();
            let _ = http.set_scheme("http");
            match query_version(&client, &http).await {
                Ok(v) => {
                    url = http;
                    Ok(v)
                }
                Err(_) => first,
            }
        }
        other => other,
    };

    match result {
        Ok(info) => {
            let major = major_from_version_info(&info["server_version_info"]).ok_or("The server answered, but not like Odoo.")?;
            Ok(ServerInfo {
                url: url.as_str().trim_end_matches('/').to_string(),
                version: info["server_version"].as_str().unwrap_or("?").to_string(),
                major,
                discuss_path: discuss_path_for(major).to_string(),
            })
        }
        Err(ProbeError::Unreachable(msg)) => Err(format!("Could not reach the server: {msg}")),
        Err(ProbeError::NotOdoo) => Err("This address answers, but it does not look like an Odoo server.".into()),
    }
}

#[derive(Debug)]
enum ProbeError {
    Unreachable(String),
    NotOdoo,
}

async fn query_version(client: &reqwest::Client, base: &Url) -> Result<serde_json::Value, ProbeError> {
    let endpoint = base.join("/web/webclient/version_info").map_err(|e| ProbeError::Unreachable(e.to_string()))?;
    let response = client
        .post(endpoint)
        .json(&serde_json::json!({"jsonrpc": "2.0", "method": "call", "params": {}}))
        .send()
        .await
        .map_err(|e| ProbeError::Unreachable(friendly(&e)))?;
    if !response.status().is_success() {
        return Err(ProbeError::NotOdoo);
    }
    let body: serde_json::Value = response.json().await.map_err(|_| ProbeError::NotOdoo)?;
    let result = body.get("result").cloned().ok_or(ProbeError::NotOdoo)?;
    if result.get("server_version_info").is_none() {
        return Err(ProbeError::NotOdoo);
    }
    Ok(result)
}

fn friendly(e: &reqwest::Error) -> String {
    if e.is_timeout() {
        "it took too long to answer".into()
    } else if e.is_connect() {
        "could not connect, check the address and your internet connection".into()
    } else {
        let text = e.to_string();
        if text.to_lowercase().contains("certificate") {
            "the security certificate is not valid".into()
        } else {
            text
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalises_what_people_type() {
        assert_eq!(normalize("odoo.example.org").unwrap().as_str(), "https://odoo.example.org/");
        assert_eq!(normalize("  https://odoo.example.org/web/login?x=1#y ").unwrap().as_str(), "https://odoo.example.org/");
        assert_eq!(normalize("http://192.168.1.5:8069/odoo").unwrap().as_str(), "http://192.168.1.5:8069/");
        assert!(normalize("").is_err());
        assert!(normalize("ftp://example.org").is_err());
    }

    #[test]
    fn discuss_path_depends_on_version() {
        assert_eq!(discuss_path_for(19), "/odoo/discuss");
        assert_eq!(discuss_path_for(18), "/odoo/discuss");
        assert_eq!(discuss_path_for(17), "/web#action=mail.action_discuss");
    }

    #[test]
    fn reads_major_version() {
        let info = serde_json::json!([19, 0, 0, "final", 0, ""]);
        assert_eq!(major_from_version_info(&info), Some(19));
        assert_eq!(major_from_version_info(&serde_json::json!(null)), None);
    }

    // Network tests: `cargo test -- --ignored`. Set PIGEON_TEST_SERVER to a real Odoo address.
    #[test]
    #[ignore]
    fn probes_a_real_odoo_server() {
        let server = std::env::var("PIGEON_TEST_SERVER").expect("set PIGEON_TEST_SERVER");
        let info = tauri::async_runtime::block_on(probe(&server)).expect("probe should succeed");
        println!("detected {} (major {}) at {} -> {}", info.version, info.major, info.url, info.discuss_path);
        assert!(info.major >= 14);
    }

    #[test]
    #[ignore]
    fn explains_when_the_site_is_not_odoo_or_unreachable() {
        let not_odoo = tauri::async_runtime::block_on(probe("https://example.com")).unwrap_err();
        println!("example.com -> {not_odoo}");
        assert!(not_odoo.contains("does not look like an Odoo"));
        let nowhere = tauri::async_runtime::block_on(probe("https://this-server-does-not-exist.invalid")).unwrap_err();
        println!("unreachable -> {nowhere}");
        assert!(nowhere.starts_with("Could not reach the server"));
    }
}
