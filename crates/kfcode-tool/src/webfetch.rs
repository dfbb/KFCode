//! Tool for fetching web content and returning it as text, markdown, or HTML.
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use crate::{Tool, ToolContext, ToolError, ToolResult};

const MAX_RESPONSE_SIZE: usize = 5 * 1024 * 1024;
const DEFAULT_TIMEOUT_SECS: u64 = 30;
const MAX_TIMEOUT_SECS: u64 = 120;

/// Fetches a URL and returns its content in the requested format.
pub struct WebFetchTool {
    client: Client,
}

impl WebFetchTool {
    /// Creates a `WebFetchTool` with a browser-like user-agent and a long timeout.
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/143.0.0.0 Safari/537.36")
                .timeout(std::time::Duration::from_secs(MAX_TIMEOUT_SECS))
                .build()
                .unwrap(),
        }
    }
}

/// Deserialized input for a web-fetch request.
#[derive(Debug, Serialize, Deserialize)]
struct WebFetchInput {
    url: String,
    #[serde(default = "default_format")]
    format: String,
    #[serde(default)]
    timeout: Option<u64>,
}

fn default_format() -> String {
    "markdown".to_string()
}

fn is_private_ipv4(ip: Ipv4Addr) -> bool {
    ip.is_loopback()           // 127.0.0.0/8
        || ip.is_private()     // 10/8, 172.16/12, 192.168/16
        || ip.is_link_local()  // 169.254/16 — AWS/GCP/Azure metadata
        || ip.is_broadcast()   // 255.255.255.255
        || ip.is_documentation() // 192.0.2/24, 198.51.100/24, 203.0.113/24
        || ip.is_unspecified() // 0.0.0.0
        || ip.is_multicast()
        // CGNAT 100.64.0.0/10
        || (ip.octets()[0] == 100 && (ip.octets()[1] & 0xc0) == 0x40)
}

fn ipv6_to_v4_mapped(ip: Ipv6Addr) -> Option<Ipv4Addr> {
    let s = ip.segments();
    if s[0] == 0 && s[1] == 0 && s[2] == 0 && s[3] == 0 && s[4] == 0 && s[5] == 0xffff {
        Some(Ipv4Addr::new(
            (s[6] >> 8) as u8,
            (s[6] & 0xff) as u8,
            (s[7] >> 8) as u8,
            (s[7] & 0xff) as u8,
        ))
    } else {
        None
    }
}

fn is_private_ipv6(ip: Ipv6Addr) -> bool {
    ip.is_loopback()           // ::1
        || ip.is_unspecified() // ::
        || ip.is_multicast()
        // ULA fc00::/7
        || (ip.segments()[0] & 0xfe00) == 0xfc00
        // Link-local fe80::/10
        || (ip.segments()[0] & 0xffc0) == 0xfe80
        // IPv4-mapped: 检查内嵌 v4
        || ipv6_to_v4_mapped(ip).map(is_private_ipv4).unwrap_or(false)
}

fn is_blocked_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => is_private_ipv4(v4),
        IpAddr::V6(v6) => is_private_ipv6(v6),
    }
}

async fn validate_public_url(url: &str) -> Result<(), ToolError> {
    let parsed = url::Url::parse(url)
        .map_err(|e| ToolError::InvalidArguments(format!("invalid URL: {e}")))?;

    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(ToolError::InvalidArguments(
            "only http/https URLs are allowed".to_string(),
        ));
    }

    let host = parsed
        .host_str()
        .ok_or_else(|| ToolError::InvalidArguments("URL missing host".to_string()))?;

    // 如果 host 是 IP 字面量，直接判断
    // url crate 对 IPv6 字面量会去掉方括号，可直接 parse
    if let Ok(ip) = host.parse::<IpAddr>() {
        if is_blocked_ip(ip) {
            return Err(ToolError::InvalidArguments(format!(
                "blocked: {ip} is a loopback/private/link-local address"
            )));
        }
        return Ok(());
    }

    // 域名 → DNS 解析，检查所有结果
    let port = parsed.port_or_known_default().unwrap_or(80);
    let addrs: Vec<_> = tokio::net::lookup_host(format!("{host}:{port}"))
        .await
        .map_err(|e| ToolError::InvalidArguments(format!("DNS lookup failed: {e}")))?
        .collect();

    if addrs.is_empty() {
        return Err(ToolError::InvalidArguments(
            "DNS returned no addresses".to_string(),
        ));
    }

    for addr in &addrs {
        if is_blocked_ip(addr.ip()) {
            return Err(ToolError::InvalidArguments(format!(
                "blocked: {} resolves to {} which is a loopback/private/link-local address",
                host,
                addr.ip()
            )));
        }
    }

    Ok(())
}

#[async_trait]
impl Tool for WebFetchTool {
    fn id(&self) -> &str {
        "webfetch"
    }

    fn description(&self) -> &str {
        "Fetch content from a URL. Returns the content in the specified format (text, markdown, or html). Defaults to markdown."
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "The URL to fetch content from"
                },
                "format": {
                    "type": "string",
                    "enum": ["text", "markdown", "html"],
                    "default": "markdown",
                    "description": "The format to return the content in (text, markdown, or html). Defaults to markdown."
                },
                "timeout": {
                    "type": "number",
                    "description": "Optional timeout in seconds (max 120)"
                }
            },
            "required": ["url"]
        })
    }

    async fn execute(
        &self,
        args: serde_json::Value,
        ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let input: WebFetchInput =
            serde_json::from_value(args).map_err(|e| ToolError::InvalidArguments(e.to_string()))?;

        let url = input.url.clone();

        validate_public_url(&url).await?;

        ctx.ask_permission(
            crate::PermissionRequest::new("webfetch")
                .with_pattern(&url)
                .always_allow(),
        )
        .await?;

        let timeout_secs = input
            .timeout
            .unwrap_or(DEFAULT_TIMEOUT_SECS)
            .min(MAX_TIMEOUT_SECS);

        let accept_header = match input.format.as_str() {
            "markdown" => "text/markdown;q=1.0, text/x-markdown;q=0.9, text/plain;q=0.8, text/html;q=0.7, */*;q=0.1",
            "text" => "text/plain;q=1.0, text/markdown;q=0.9, text/html;q=0.8, */*;q=0.1",
            "html" => "text/html;q=1.0, application/xhtml+xml;q=0.9, text/plain;q=0.8, text/markdown;q=0.7, */*;q=0.1",
            _ => "*/*",
        };

        let response = tokio::select! {
            result = self.fetch_with_retry(&url, accept_header, timeout_secs) => result,
            _ = tokio::time::sleep(std::time::Duration::from_secs(timeout_secs)) => {
                return Err(ToolError::Timeout(format!("Request timed out after {} seconds", timeout_secs)));
            }
            _ = ctx.abort.cancelled() => {
                return Err(ToolError::Cancelled);
            }
        };

        let response = response?;

        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();

        let content_length = response
            .headers()
            .get("content-length")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<usize>().ok());

        if let Some(len) = content_length {
            if len > MAX_RESPONSE_SIZE {
                return Err(ToolError::ExecutionError(
                    "Response too large (exceeds 5MB limit)".to_string(),
                ));
            }
        }

        let bytes = response
            .bytes()
            .await
            .map_err(|e| ToolError::ExecutionError(format!("Failed to read response: {}", e)))?;

        if bytes.len() > MAX_RESPONSE_SIZE {
            return Err(ToolError::ExecutionError(
                "Response too large (exceeds 5MB limit)".to_string(),
            ));
        }

        let mime = content_type
            .split(';')
            .next()
            .unwrap_or("")
            .trim()
            .to_lowercase();
        let title = format!("{} ({})", url, content_type);

        let is_image = mime.starts_with("image/")
            && mime != "image/svg+xml"
            && mime != "image/vnd.fastbidsheet";

        if is_image {
            let base64_content =
                base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &bytes);
            let data_url = format!("data:{};base64,{}", mime, base64_content);
            let output = format!(
                "Image fetched successfully.\n\n<attachment type=\"image\" mimeType=\"{}\" url=\"{}\" size=\"{}\" data=\"{}\" />",
                mime, url, bytes.len(), data_url
            );
            let mut metadata = std::collections::HashMap::new();
            metadata.insert("url".to_string(), serde_json::json!(url));
            metadata.insert("mimeType".to_string(), serde_json::json!(mime));
            metadata.insert("size".to_string(), serde_json::json!(bytes.len()));
            metadata.insert("data".to_string(), serde_json::json!(data_url));
            metadata.insert(
                "attachment".to_string(),
                serde_json::json!({
                    "type": "image",
                    "mimeType": mime,
                    "url": url,
                    "size": bytes.len(),
                    "data": data_url
                }),
            );
            return Ok(ToolResult {
                title,
                output,
                metadata,
                truncated: false,
            });
        }

        let content = String::from_utf8_lossy(&bytes).to_string();

        let output = match input.format.as_str() {
            "markdown" => {
                if content_type.contains("text/html") {
                    convert_html_to_markdown(&content)
                } else {
                    content
                }
            }
            "text" => {
                if content_type.contains("text/html") {
                    strip_html(&content)
                } else {
                    content
                }
            }
            "html" | _ => content,
        };

        let mut metadata = std::collections::HashMap::new();
        metadata.insert("url".to_string(), serde_json::json!(url));
        metadata.insert("format".to_string(), serde_json::json!(input.format));
        metadata.insert("mimeType".to_string(), serde_json::json!(mime));
        metadata.insert("size".to_string(), serde_json::json!(output.len()));

        Ok(ToolResult {
            title,
            output,
            metadata,
            truncated: false,
        })
    }
}

impl WebFetchTool {
    async fn fetch_with_retry(
        &self,
        url: &str,
        accept_header: &str,
        _timeout_secs: u64,
    ) -> Result<reqwest::Response, ToolError> {
        let response = self
            .client
            .get(url)
            .header("Accept", accept_header)
            .header("Accept-Language", "en-US,en;q=0.9")
            .send()
            .await
            .map_err(|e| ToolError::ExecutionError(format!("Failed to fetch URL: {}", e)))?;

        if response.status() == 403 {
            let cf_mitigated = response
                .headers()
                .get("cf-mitigated")
                .and_then(|v| v.to_str().ok());

            if cf_mitigated == Some("challenge") {
                return self
                    .client
                    .get(url)
                    .header("Accept", accept_header)
                    .header("User-Agent", "kfcode")
                    .send()
                    .await
                    .map_err(|e| ToolError::ExecutionError(format!("Failed to fetch URL: {}", e)));
            }
        }

        if !response.status().is_success() {
            return Err(ToolError::ExecutionError(format!(
                "Request failed with status code: {}",
                response.status()
            )));
        }

        Ok(response)
    }
}

fn convert_html_to_markdown(html: &str) -> String {
    html2md::parse_html(html)
}

fn strip_html(html: &str) -> String {
    let mut result = String::new();
    let mut in_tag = false;
    let mut in_script = false;
    let mut in_style = false;
    let chars: Vec<char> = html.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        let c = chars[i];

        if c == '<' {
            if i + 7 <= len {
                let tag: String = chars[i..i + 7].iter().collect();
                let tag_lower = tag.to_lowercase();
                if tag_lower.starts_with("<script") {
                    in_script = true;
                } else if tag_lower.starts_with("<style") {
                    in_style = true;
                }
            }
            in_tag = true;
            i += 1;
            continue;
        }

        if c == '>' {
            if in_script {
                if i >= 8 {
                    let end_tag: String = chars[i - 8..=i].iter().collect();
                    if end_tag.to_lowercase() == "</script>" {
                        in_script = false;
                    }
                }
            } else if in_style {
                if i >= 7 {
                    let end_tag: String = chars[i - 7..=i].iter().collect();
                    if end_tag.to_lowercase() == "</style>" {
                        in_style = false;
                    }
                }
            }
            in_tag = false;
            i += 1;
            continue;
        }

        if !in_tag && !in_script && !in_style {
            if c == '&' {
                if i + 4 <= len {
                    let entity: String = chars[i..i + 4].iter().collect();
                    match entity.as_str() {
                        "&lt;" => {
                            result.push('<');
                            i += 4;
                            continue;
                        }
                        "&gt;" => {
                            result.push('>');
                            i += 4;
                            continue;
                        }
                        "&amp;" => {
                            result.push('&');
                            i += 5;
                            continue;
                        }
                        _ => {}
                    }
                }
                if i + 6 <= len {
                    let entity: String = chars[i..i + 6].iter().collect();
                    if entity == "&nbsp;" {
                        result.push(' ');
                        i += 6;
                        continue;
                    }
                }
            }
            result.push(c);
        }

        i += 1;
    }

    let result = result
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n");

    result
}

impl Default for WebFetchTool {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

    #[test]
    fn blocks_loopback_v4() {
        assert!(is_blocked_ip(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))));
    }

    #[test]
    fn blocks_link_local_v4() {
        // 169.254.169.254 是 AWS/Azure metadata 服务，必须 block
        assert!(is_blocked_ip(IpAddr::V4(Ipv4Addr::new(169, 254, 169, 254))));
    }

    #[test]
    fn blocks_private_ranges_v4() {
        assert!(is_blocked_ip(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1))));
        assert!(is_blocked_ip(IpAddr::V4(Ipv4Addr::new(172, 16, 0, 1))));
        assert!(is_blocked_ip(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1))));
    }

    #[test]
    fn blocks_unspecified_v4() {
        assert!(is_blocked_ip(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0))));
    }

    #[test]
    fn blocks_loopback_v6() {
        assert!(is_blocked_ip(IpAddr::V6(Ipv6Addr::LOCALHOST)));
    }

    #[test]
    fn blocks_cgnat() {
        // 100.64.0.0/10
        assert!(is_blocked_ip(IpAddr::V4(Ipv4Addr::new(100, 64, 0, 1))));
        assert!(is_blocked_ip(IpAddr::V4(Ipv4Addr::new(100, 127, 255, 255))));
    }

    #[test]
    fn blocks_ipv6_ula() {
        // fc00::/7
        let ip: Ipv6Addr = "fc00::1".parse().unwrap();
        assert!(is_blocked_ip(IpAddr::V6(ip)));
        let ip2: Ipv6Addr = "fd00::1".parse().unwrap();
        assert!(is_blocked_ip(IpAddr::V6(ip2)));
    }

    #[test]
    fn blocks_ipv6_link_local() {
        // fe80::/10
        let ip: Ipv6Addr = "fe80::1".parse().unwrap();
        assert!(is_blocked_ip(IpAddr::V6(ip)));
    }

    #[test]
    fn blocks_ipv4_mapped_private() {
        // ::ffff:192.168.1.1
        let ip: Ipv6Addr = "::ffff:192.168.1.1".parse().unwrap();
        assert!(is_blocked_ip(IpAddr::V6(ip)));
    }

    #[test]
    fn allows_public_v4() {
        assert!(!is_blocked_ip(IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8))));
        assert!(!is_blocked_ip(IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1))));
    }

    #[tokio::test]
    async fn rejects_loopback_url() {
        let res = validate_public_url("http://127.0.0.1/").await;
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn rejects_metadata_url() {
        let res = validate_public_url("http://169.254.169.254/latest/meta-data/").await;
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn rejects_non_http_scheme() {
        let res = validate_public_url("file:///etc/passwd").await;
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn rejects_ipv6_loopback_url() {
        let res = validate_public_url("http://[::1]/").await;
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn rejects_private_ipv4_url() {
        let res = validate_public_url("http://10.0.0.1/").await;
        assert!(res.is_err());
        let res2 = validate_public_url("http://192.168.1.1/").await;
        assert!(res2.is_err());
    }
}
