use crate::config::load_config;
use crate::remote::{RemoteFormat, RemotePatterns};
use crate::runtime;
use std::path::Path;
use std::time::Duration;

// Cap the whole request — connect, TLS, headers, body — so a wedged or
// slow-trickle remote can't hang the daemon's update path indefinitely.
const FETCH_TIMEOUT: Duration = Duration::from_secs(30);

// 32 MiB is well above the actual ClearURLs payload (~0.5 MB today) but small
// enough that a hostile or wedged endpoint can't OOM the daemon by streaming
// gigabytes into the update path.
const FETCH_MAX_BODY_BYTES: u64 = 32 * 1024 * 1024;

// ureq follows redirects by default. Cap it so a redirect loop can't spin the
// daemon forever, and so a server can't bounce the fetch through dozens of
// hosts before we even decide whether to keep the bytes.
const FETCH_MAX_REDIRECTS: u32 = 5;

// Refuse non-https remote URLs upfront. Plaintext HTTP would let a network
// attacker swap the rule set (privacy regression or hostile redirections);
// `file://` / `data:` / etc. would let a hand-edited config read or exfiltrate
// local files through the same code path.
fn validate_remote_url(url: &str) -> Result<(), String> {
    let parsed = url::Url::parse(url).map_err(|e| format!("invalid remote URL '{url}': {e}"))?;
    if parsed.scheme() != "https" {
        return Err(format!(
            "remote URL must use https, got scheme '{}': {url}",
            parsed.scheme()
        ));
    }
    Ok(())
}

fn build_agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(FETCH_TIMEOUT))
        .max_redirects(FETCH_MAX_REDIRECTS)
        .build()
        .into()
}

fn validation_warnings(patterns: &RemotePatterns) -> Vec<String> {
    patterns
        .providers
        .iter()
        .flat_map(|(name, cfg)| crate::provider::check_provider(name, cfg))
        .collect()
}

fn fetch_remote(agent: &ureq::Agent, url: &str, byte_limit: u64) -> Result<String, String> {
    agent
        .get(url)
        .call()
        .map_err(|e| format!("Failed to fetch remote patterns: {e}"))?
        .body_mut()
        .with_config()
        .limit(byte_limit)
        .read_to_string()
        .map_err(|e| format!("Failed to read response body: {e}"))
}

pub fn execute(config_path: &Path, write_snapshot: Option<&Path>) -> Result<(), String> {
    let cfg = load_config(config_path)?;

    let remote = cfg.remote.ok_or(
        "No [remote] section in config.\n\
         Add a [remote] section with url and format to use `clink update`.\n\
         Example:\n\n\
         [remote]\n\
         url = 'https://rules2.clearurls.xyz/data.min.json'\n\
         format = 'clearurls'"
            .to_string(),
    )?;

    validate_remote_url(&remote.url)?;

    println!("Fetching patterns from {}", remote.url);

    let agent = build_agent();
    let body = fetch_remote(&agent, &remote.url, FETCH_MAX_BODY_BYTES)?;

    let patterns = match remote.format {
        RemoteFormat::ClearUrls => translate_clearurls(&body)?,
        RemoteFormat::Clink => parse_clink_toml(&body)?,
    };

    let warnings = validation_warnings(&patterns);
    if !warnings.is_empty() {
        eprintln!(
            "Skipped {} invalid pattern(s) (cache will still be written; daemon will skip these at load time):",
            warnings.len()
        );
        for w in &warnings {
            eprintln!("  - {w}");
        }
    }

    if let Some(snapshot_path) = write_snapshot {
        let (provider_count, rule_count) = write_patterns_to(snapshot_path, &patterns)?;
        println!(
            "Wrote snapshot with {provider_count} providers and {rule_count} rules to {}",
            snapshot_path.display()
        );
    } else {
        let cache_path = runtime::data_dir().join("remote_patterns.toml");
        let (provider_count, rule_count) = write_patterns_to(&cache_path, &patterns)?;
        println!(
            "Cached {provider_count} providers with {rule_count} rules to {}",
            cache_path.display()
        );
    }

    Ok(())
}

fn write_patterns_to(path: &Path, patterns: &RemotePatterns) -> Result<(usize, usize), String> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create directory {}: {e}", parent.display()))?;
        }
    }
    let content = toml::to_string_pretty(patterns)
        .map_err(|e| format!("Failed to serialize patterns: {e}"))?;
    runtime::write_atomic(path, &content)?;

    let provider_count = patterns.providers.len();
    let rule_count: usize = patterns.providers.values().map(|p| p.rules.len()).sum();
    Ok((provider_count, rule_count))
}

fn translate_clearurls(body: &str) -> Result<RemotePatterns, String> {
    let result = crate::clearurls::translate(body)?;

    println!(
        "Translated {} providers with {} rules",
        result.providers.len(),
        result.rules_translated
    );
    if result.complete_providers_skipped > 0 {
        println!(
            "Skipped {} ClearURLs `completeProvider` entries (block-the-whole-site rules — clink only strips params, not full domains)",
            result.complete_providers_skipped
        );
    }
    println!(
        "ClearURLs data provided by the ClearURLs project (LGPLv3) — https://docs.clearurls.xyz"
    );

    Ok(RemotePatterns {
        providers: result.providers,
    })
}

fn parse_clink_toml(body: &str) -> Result<RemotePatterns, String> {
    toml::from_str::<RemotePatterns>(body).map_err(|e| format!("Failed to parse clink TOML: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_remote_url_accepts_https() {
        assert!(validate_remote_url("https://rules2.clearurls.xyz/data.min.json").is_ok());
    }

    #[test]
    fn validate_remote_url_rejects_http() {
        let err = validate_remote_url("http://rules2.clearurls.xyz/data.min.json").unwrap_err();
        assert!(
            err.to_lowercase().contains("https"),
            "rejection must mention https requirement, got: {err}"
        );
    }

    #[test]
    fn validate_remote_url_rejects_file_scheme() {
        let err = validate_remote_url("file:///etc/passwd").unwrap_err();
        assert!(
            err.to_lowercase().contains("https"),
            "rejection must mention https requirement, got: {err}"
        );
    }

    #[test]
    fn validate_remote_url_rejects_garbage() {
        assert!(validate_remote_url("not a url").is_err());
    }

    fn spawn_oversized_server(body_size: usize) -> String {
        use std::io::{Read, Write};
        use std::net::TcpListener;
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        std::thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buf = [0u8; 4096];
                let _ = stream.read(&mut buf);
                let _ = write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Length: {body_size}\r\nConnection: close\r\n\r\n"
                );
                let _ = stream.write_all(&vec![b'x'; body_size]);
            }
        });
        format!("http://127.0.0.1:{port}/")
    }

    fn spawn_redirect_loop_server() -> String {
        use std::io::{Read, Write};
        use std::net::TcpListener;
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let url = format!("http://127.0.0.1:{port}/");
        let target = url.clone();
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let mut s = stream;
                let mut buf = [0u8; 4096];
                let _ = s.read(&mut buf);
                let _ = write!(
                    s,
                    "HTTP/1.1 302 Found\r\nLocation: {target}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                );
            }
        });
        url
    }

    #[test]
    fn fetch_remote_rejects_oversized_body() {
        let url = spawn_oversized_server(8192);
        let agent = build_agent();
        let result = fetch_remote(&agent, &url, 1024);
        assert!(
            result.is_err(),
            "body larger than the byte limit must be rejected, got Ok"
        );
    }

    #[test]
    fn validation_warnings_empty_for_clean_patterns() {
        let mut providers = std::collections::HashMap::new();
        providers.insert(
            "ok".to_string(),
            crate::provider::ProviderConfig {
                url_pattern: Some(r"^https?://example\.com".into()),
                rules: vec!["fbclid".into(), "(?:ref_?)?src".into()],
                ..Default::default()
            },
        );
        let patterns = RemotePatterns { providers };
        assert!(validation_warnings(&patterns).is_empty());
    }

    #[test]
    fn validation_warnings_flags_bad_url_pattern() {
        let mut providers = std::collections::HashMap::new();
        providers.insert(
            "bad".to_string(),
            crate::provider::ProviderConfig {
                url_pattern: Some("[unclosed".into()),
                ..Default::default()
            },
        );
        let patterns = RemotePatterns { providers };
        let warnings = validation_warnings(&patterns);
        assert_eq!(warnings.len(), 1, "expected one warning, got {warnings:?}");
        assert!(warnings[0].contains("bad"));
    }

    #[test]
    fn validation_warnings_flags_bad_rule_regex() {
        let mut providers = std::collections::HashMap::new();
        providers.insert(
            "scoped".to_string(),
            crate::provider::ProviderConfig {
                url_pattern: Some(r"^https?://x\.com".into()),
                rules: vec!["[bad".into()],
                ..Default::default()
            },
        );
        let patterns = RemotePatterns { providers };
        let warnings = validation_warnings(&patterns);
        assert_eq!(warnings.len(), 1, "expected one warning, got {warnings:?}");
    }

    #[test]
    fn fetch_remote_caps_redirect_chain() {
        let url = spawn_redirect_loop_server();
        let agent = build_agent();
        let result = fetch_remote(&agent, &url, 8192);
        assert!(
            result.is_err(),
            "infinite redirect loop must be capped at FETCH_MAX_REDIRECTS"
        );
    }

    #[test]
    fn test_parse_clink_toml_valid() {
        let toml = r#"
[providers.global]
rules = ['fbclid', 'gclid']

[providers.exitsc]
url_pattern = '^https?://exit\.sc'
redirections = ['^https?://exit\.sc/\?.*?url=([^&]+)']
"#;
        let result = parse_clink_toml(toml).unwrap();
        assert_eq!(result.providers.len(), 2);
        assert_eq!(result.providers["global"].rules.len(), 2);
    }

    #[test]
    fn test_parse_clink_toml_invalid() {
        let result = parse_clink_toml("not valid toml [[[");
        assert!(result.is_err());
    }

    #[test]
    fn test_translate_clearurls_valid() {
        let json = r#"{
            "providers": {
                "test": {
                    "urlPattern": "^https?://test\\.com",
                    "completeProvider": false,
                    "rules": ["fbclid", "gclid"],
                    "referralMarketing": [],
                    "rawRules": [],
                    "exceptions": [],
                    "redirections": [],
                    "forceRedirection": false
                }
            }
        }"#;
        let result = translate_clearurls(json).unwrap();
        let test_provider = &result.providers["test"];
        assert!(test_provider.rules.contains(&"fbclid".to_string()));
        assert!(test_provider.rules.contains(&"gclid".to_string()));
        assert_eq!(
            test_provider.url_pattern.as_deref(),
            Some("^https?://test\\.com")
        );
    }

    #[test]
    fn test_translate_clearurls_invalid() {
        let result = translate_clearurls("not json");
        assert!(result.is_err());
    }

    #[test]
    fn test_write_patterns_to_writes_toml() {
        let dir = std::env::temp_dir().join("clink_test_write_patterns_to");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("out.toml");

        let mut providers = std::collections::HashMap::new();
        providers.insert(
            "global".to_string(),
            crate::provider::ProviderConfig {
                rules: vec!["fbclid".into(), "gclid".into()],
                ..Default::default()
            },
        );
        let patterns = RemotePatterns { providers };

        let (provider_count, rule_count) = write_patterns_to(&path, &patterns).unwrap();

        assert_eq!(provider_count, 1);
        assert_eq!(rule_count, 2);
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("fbclid"));
        assert!(content.contains("gclid"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_write_patterns_to_is_atomic_no_tmp_leftover() {
        let dir = std::env::temp_dir().join("clink_test_write_patterns_atomic");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("out.toml");
        let tmp_path = dir.join("out.toml.tmp");

        let mut providers = std::collections::HashMap::new();
        providers.insert(
            "global".to_string(),
            crate::provider::ProviderConfig {
                rules: vec!["fbclid".into()],
                ..Default::default()
            },
        );
        let patterns = RemotePatterns { providers };

        write_patterns_to(&path, &patterns).unwrap();

        assert!(path.is_file(), "target file must exist after write");
        assert!(
            !tmp_path.exists(),
            "no .tmp sibling should remain after a successful atomic write"
        );
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("fbclid"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_write_patterns_to_overwrites_existing_target() {
        let dir = std::env::temp_dir().join("clink_test_write_patterns_overwrite");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("out.toml");

        std::fs::write(&path, "stale = 'content'\n").unwrap();

        let mut providers = std::collections::HashMap::new();
        providers.insert(
            "global".to_string(),
            crate::provider::ProviderConfig {
                rules: vec!["fresh_rule".into()],
                ..Default::default()
            },
        );
        let patterns = RemotePatterns { providers };

        write_patterns_to(&path, &patterns).unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(
            content.contains("fresh_rule"),
            "target must be replaced with new content"
        );
        assert!(
            !content.contains("stale"),
            "stale content must be gone after atomic rename"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_write_patterns_to_creates_parent_dir() {
        let dir = std::env::temp_dir().join("clink_test_write_patterns_parent/nested");
        let _ =
            std::fs::remove_dir_all(std::env::temp_dir().join("clink_test_write_patterns_parent"));
        let path = dir.join("out.toml");

        let patterns = RemotePatterns {
            providers: std::collections::HashMap::new(),
        };
        write_patterns_to(&path, &patterns).unwrap();
        assert!(path.is_file());

        let _ =
            std::fs::remove_dir_all(std::env::temp_dir().join("clink_test_write_patterns_parent"));
    }
}
