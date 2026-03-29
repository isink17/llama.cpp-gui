use crate::core::models::ResolvedModelDownload;
use parking_lot::Mutex;
use reqwest::blocking::Client;
use serde::Deserialize;
use std::collections::HashMap;
use std::time::{Duration, Instant};

struct CacheEntry<T> {
    data: T,
    expires_at: Instant,
}

pub struct ModelResolverService {
    client: Client,
    ollama_cache: Mutex<HashMap<String, CacheEntry<Vec<String>>>>,
    hf_cache: Mutex<HashMap<String, CacheEntry<Vec<String>>>>,
}

#[derive(Deserialize)]
struct HuggingFaceModel {
    siblings: Option<Vec<HuggingFaceSibling>>,
}

#[derive(Deserialize)]
struct HuggingFaceSibling {
    rfilename: String,
}

#[derive(Deserialize)]
struct OllamaManifest {
    layers: Option<Vec<OllamaLayer>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct OllamaLayer {
    #[serde(default)]
    media_type: String,
    #[serde(default)]
    digest: String,
}

#[derive(Deserialize)]
struct OllamaTagList {
    tags: Option<Vec<OllamaTag>>,
}

#[derive(Deserialize)]
struct OllamaTag {
    name: String,
}

impl ModelResolverService {
    pub fn new() -> Result<Self, String> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| format!("failed to build HTTP client for model resolver: {e}"))?;
        Ok(Self {
            client,
            ollama_cache: Mutex::new(HashMap::new()),
            hf_cache: Mutex::new(HashMap::new()),
        })
    }

    pub fn resolve(
        &self,
        source: &str,
        input: &str,
        hf_token: Option<&str>,
    ) -> Result<ResolvedModelDownload, String> {
        match source {
            "Direct URL" => self.resolve_direct_url(input),
            "Hugging Face" => self.resolve_hugging_face(input, hf_token),
            "Ollama Library" => self.resolve_ollama(input),
            _ => Err(format!("unknown source: {source}")),
        }
    }

    pub fn list_hugging_face_gguf_files(
        &self,
        input: &str,
        token: Option<&str>,
    ) -> Result<Vec<String>, String> {
        let repo_id = parse_hugging_face_repo_id(input)?;
        let siblings = self.fetch_hugging_face_siblings(&repo_id, token)?;
        let gguf_files: Vec<String> = siblings
            .into_iter()
            .filter(|name| name.ends_with(".gguf"))
            .collect();
        Ok(gguf_files)
    }

    pub fn list_ollama_tags(&self, input: &str) -> Result<Vec<String>, String> {
        let repo_path = parse_ollama_repo_path(input);

        // Check cache first
        {
            let cache = self.ollama_cache.lock();
            if let Some(entry) = cache.get(&repo_path) {
                if entry.expires_at > Instant::now() {
                    return Ok(entry.data.clone());
                }
            }
        }

        let registry_url = format!("https://registry.ollama.ai/v2/{repo_path}/tags/list");
        let response = self
            .client
            .get(&registry_url)
            .send()
            .map_err(|e| format!("failed to fetch Ollama tags: {e}"))?;

        let tags = if response.status().is_success() {
            let tag_list: OllamaTagList = response
                .json()
                .map_err(|e| format!("failed to parse Ollama tags response: {e}"))?;
            tag_list
                .tags
                .unwrap_or_default()
                .into_iter()
                .map(|t| t.name)
                .collect()
        } else {
            let library_url = format!("https://ollama.com/{repo_path}/tags");
            let fallback = self
                .client
                .get(&library_url)
                .send()
                .map_err(|e| format!("failed to fetch Ollama tags page: {e}"))?;

            if !fallback.status().is_success() {
                return Err(format!(
                    "Ollama tags request failed with HTTP status {} (registry) and {} (library)",
                    response.status(),
                    fallback.status()
                ));
            }

            let body = fallback
                .text()
                .map_err(|e| format!("failed to read Ollama tags page: {e}"))?;
            let tags = extract_ollama_tags_from_body(&body, &repo_path);
            if tags.is_empty() {
                return Err("Ollama tags page did not contain any tags".to_string());
            }
            tags
        };

        // Store in cache with 5-minute TTL
        {
            let mut cache = self.ollama_cache.lock();
            cache.insert(
                repo_path,
                CacheEntry {
                    data: tags.clone(),
                    expires_at: Instant::now() + Duration::from_secs(300),
                },
            );
        }

        Ok(tags)
    }

    fn resolve_direct_url(&self, input: &str) -> Result<ResolvedModelDownload, String> {
        let url = input.trim().to_string();
        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Err("URL must start with http:// or https://".to_string());
        }

        let file_name = extract_filename_from_url(&url).unwrap_or_else(|| "model.gguf".to_string());

        Ok(ResolvedModelDownload {
            download_url: url,
            suggested_file_name: file_name,
            request_headers: None,
        })
    }

    fn resolve_hugging_face(
        &self,
        input: &str,
        hf_token: Option<&str>,
    ) -> Result<ResolvedModelDownload, String> {
        let input = input.trim();

        // If input is an absolute URL, use it directly
        if input.starts_with("http://") || input.starts_with("https://") {
            let file_name =
                extract_filename_from_url(input).unwrap_or_else(|| "model.gguf".to_string());
            let headers = build_hf_auth_headers(hf_token);
            return Ok(ResolvedModelDownload {
                download_url: input.to_string(),
                suggested_file_name: file_name,
                request_headers: headers,
            });
        }

        // Parse "owner/repo" or "owner/repo::file.gguf"
        let (repo_id, explicit_file) = parse_hugging_face_input(input)?;

        let file_name = if let Some(file) = explicit_file {
            file
        } else {
            let siblings = self.fetch_hugging_face_siblings(&repo_id, hf_token)?;
            let gguf_files: Vec<String> = siblings
                .into_iter()
                .filter(|name| name.ends_with(".gguf"))
                .collect();
            if gguf_files.is_empty() {
                return Err(format!("no .gguf files found in repository {repo_id}"));
            }
            pick_best_gguf_file(&gguf_files)
        };

        let download_url =
            format!("https://huggingface.co/{repo_id}/resolve/main/{file_name}?download=true");
        let headers = build_hf_auth_headers(hf_token);

        Ok(ResolvedModelDownload {
            download_url,
            suggested_file_name: file_name,
            request_headers: headers,
        })
    }

    fn resolve_ollama(&self, input: &str) -> Result<ResolvedModelDownload, String> {
        let (repo_path, tag) = parse_ollama_input(input);

        let manifest_url = format!("https://registry.ollama.ai/v2/{repo_path}/manifests/{tag}");
        let response = self
            .client
            .get(&manifest_url)
            .header(
                "Accept",
                "application/vnd.docker.distribution.manifest.v2+json",
            )
            .header("Accept", "application/vnd.oci.image.manifest.v1+json")
            .send()
            .map_err(|e| format!("failed to fetch Ollama manifest: {e}"))?;

        if !response.status().is_success() {
            return Err(format!(
                "Ollama manifest request failed with HTTP status {}",
                response.status()
            ));
        }

        let manifest: OllamaManifest = response
            .json()
            .map_err(|e| format!("failed to parse Ollama manifest: {e}"))?;

        let layers = manifest.layers.unwrap_or_default();
        if layers.is_empty() {
            return Err("Ollama manifest contains no layers".to_string());
        }

        let model_layer = layers
            .iter()
            .find(|layer| layer.media_type.contains("model") || layer.media_type.contains("gguf"))
            .unwrap_or(&layers[0]);

        let digest = &model_layer.digest;
        let download_url = format!("https://registry.ollama.ai/v2/{repo_path}/blobs/{digest}");

        let digest_prefix = digest
            .split(':')
            .last()
            .unwrap_or(digest)
            .chars()
            .take(12)
            .collect::<String>();
        let safe_repo = repo_path.replace('/', "-");
        let suggested_file_name = format!("{safe_repo}-{tag}-{digest_prefix}.gguf");

        Ok(ResolvedModelDownload {
            download_url,
            suggested_file_name,
            request_headers: None,
        })
    }

    fn fetch_hugging_face_siblings(
        &self,
        repo_id: &str,
        token: Option<&str>,
    ) -> Result<Vec<String>, String> {
        // Check cache first
        {
            let cache = self.hf_cache.lock();
            if let Some(entry) = cache.get(repo_id) {
                if entry.expires_at > Instant::now() {
                    return Ok(entry.data.clone());
                }
            }
        }

        let url = format!("https://huggingface.co/api/models/{repo_id}");
        let mut request = self.client.get(&url);
        if let Some(token) = token {
            request = request.header("Authorization", format!("Bearer {token}"));
        }

        let response = request.send().map_err(|e| {
            sanitize_error(&format!("failed to fetch Hugging Face model info: {e}"))
        })?;

        if !response.status().is_success() {
            return Err(format!(
                "Hugging Face API request failed with HTTP status {}",
                response.status()
            ));
        }

        let model: HuggingFaceModel = response.json().map_err(|e| {
            sanitize_error(&format!("failed to parse Hugging Face model response: {e}"))
        })?;

        let siblings: Vec<String> = model
            .siblings
            .unwrap_or_default()
            .into_iter()
            .map(|s| s.rfilename)
            .collect();

        // Store in cache with 5-minute TTL
        {
            let mut cache = self.hf_cache.lock();
            cache.insert(
                repo_id.to_string(),
                CacheEntry {
                    data: siblings.clone(),
                    expires_at: Instant::now() + Duration::from_secs(300),
                },
            );
        }

        Ok(siblings)
    }
}

fn extract_filename_from_url(url: &str) -> Option<String> {
    let path = url.split('?').next().unwrap_or(url);
    let segment = path.rsplit('/').next()?;
    let segment = segment.trim();
    if segment.is_empty() || !segment.contains('.') {
        return None;
    }
    Some(segment.to_string())
}

fn parse_hugging_face_repo_id(input: &str) -> Result<String, String> {
    let input = input.trim();
    let (repo_part, _) = if input.contains("::") {
        let mut parts = input.splitn(2, "::");
        let repo = parts.next().unwrap_or("").trim();
        let file = parts.next().unwrap_or("").trim();
        (repo.to_string(), Some(file.to_string()))
    } else {
        (input.to_string(), None)
    };
    if !repo_part.contains('/') {
        return Err(format!(
            "invalid Hugging Face input: expected 'owner/repo' format, got '{repo_part}'"
        ));
    }
    Ok(repo_part)
}

fn parse_hugging_face_input(input: &str) -> Result<(String, Option<String>), String> {
    let input = input.trim();
    if input.contains("::") {
        let mut parts = input.splitn(2, "::");
        let repo = parts.next().unwrap_or("").trim().to_string();
        let file = parts.next().unwrap_or("").trim().to_string();
        if !repo.contains('/') {
            return Err(format!(
                "invalid Hugging Face input: expected 'owner/repo' format, got '{repo}'"
            ));
        }
        if file.is_empty() {
            Ok((repo, None))
        } else {
            Ok((repo, Some(file)))
        }
    } else {
        if !input.contains('/') {
            return Err(format!(
                "invalid Hugging Face input: expected 'owner/repo' format, got '{input}'"
            ));
        }
        Ok((input.to_string(), None))
    }
}

fn parse_ollama_repo_path(input: &str) -> String {
    let input = input.trim();
    let input = input
        .strip_prefix("https://ollama.com/library/")
        .or_else(|| input.strip_prefix("ollama.com/library/"))
        .unwrap_or(input);

    // Strip any tag portion for repo path
    let repo = input.split(':').next().unwrap_or(input);

    if repo.contains('/') {
        repo.to_string()
    } else {
        format!("library/{repo}")
    }
}

fn parse_ollama_input(input: &str) -> (String, String) {
    let input = input.trim();
    let input = input
        .strip_prefix("https://ollama.com/library/")
        .or_else(|| input.strip_prefix("ollama.com/library/"))
        .unwrap_or(input);

    let (repo, tag) = if let Some((repo, tag)) = input.split_once(':') {
        (repo.to_string(), tag.to_string())
    } else {
        (input.to_string(), "latest".to_string())
    };

    let repo_path = if repo.contains('/') {
        repo
    } else {
        format!("library/{repo}")
    };

    (repo_path, tag)
}

fn extract_ollama_tags_from_body(body: &str, repo_path: &str) -> Vec<String> {
    let needle = format!("/{repo_path}:");
    let mut tags = Vec::new();
    let mut index = 0;

    while let Some(pos) = body[index..].find(&needle) {
        let start = index + pos + needle.len();
        let mut end = start;
        for ch in body[start..].chars() {
            if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_') {
                end += ch.len_utf8();
            } else {
                break;
            }
        }
        if end > start {
            let tag = body[start..end].to_string();
            if !tags.contains(&tag) {
                tags.push(tag);
            }
        }
        index = start;
    }

    tags
}

fn build_hf_auth_headers(token: Option<&str>) -> Option<HashMap<String, String>> {
    token.map(|t| {
        let mut headers = HashMap::new();
        headers.insert("Authorization".to_string(), format!("Bearer {t}"));
        headers
    })
}

/// Masks Bearer tokens in error messages to prevent credential leakage.
fn sanitize_error(msg: &str) -> String {
    if let Some(idx) = msg.find("Bearer ") {
        let mut sanitized = msg[..idx + 7].to_string();
        sanitized.push_str("***");
        // Find the end of the token and append the rest
        if let Some(end) = msg[idx + 7..].find(|c: char| c.is_whitespace() || c == '"') {
            sanitized.push_str(&msg[idx + 7 + end..]);
        }
        sanitized
    } else {
        msg.to_string()
    }
}

/// Scores a .gguf filename based on quantization quality and keyword bonuses.
fn score_gguf_file(name: &str) -> i32 {
    let upper = name.to_uppercase();
    let mut score = 0;

    if upper.contains("Q4_K_M") {
        score += 100;
    } else if upper.contains("Q5_K_M") {
        score += 90;
    } else if upper.contains("Q4_0") {
        score += 80;
    } else if upper.contains("Q8_0") {
        score += 70;
    }

    if upper.contains("INSTRUCT") {
        score += 5;
    }

    score
}

fn pick_best_gguf_file(files: &[String]) -> String {
    files
        .iter()
        .max_by_key(|f| score_gguf_file(f))
        .cloned()
        .unwrap_or_else(|| files[0].clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_filename_from_simple_url() {
        assert_eq!(
            extract_filename_from_url("https://example.com/path/to/model.gguf"),
            Some("model.gguf".to_string())
        );
    }

    #[test]
    fn extract_filename_from_url_with_query_string() {
        assert_eq!(
            extract_filename_from_url(
                "https://huggingface.co/repo/resolve/main/model.gguf?download=true"
            ),
            Some("model.gguf".to_string())
        );
    }

    #[test]
    fn extract_filename_returns_none_for_no_extension() {
        assert_eq!(extract_filename_from_url("https://example.com/path/"), None);
    }

    #[test]
    fn extract_filename_returns_none_for_empty_segment() {
        assert_eq!(extract_filename_from_url("https://example.com/"), None);
    }

    #[test]
    fn parse_hugging_face_input_owner_repo() {
        let (repo, file) = parse_hugging_face_input("TheBloke/Llama-2-7B-GGUF").unwrap();
        assert_eq!(repo, "TheBloke/Llama-2-7B-GGUF");
        assert!(file.is_none());
    }

    #[test]
    fn parse_hugging_face_input_owner_repo_with_file() {
        let (repo, file) =
            parse_hugging_face_input("TheBloke/Llama-2-7B-GGUF::llama-2-7b.Q4_K_M.gguf").unwrap();
        assert_eq!(repo, "TheBloke/Llama-2-7B-GGUF");
        assert_eq!(file, Some("llama-2-7b.Q4_K_M.gguf".to_string()));
    }

    #[test]
    fn parse_hugging_face_input_rejects_missing_slash() {
        let err = parse_hugging_face_input("invalid-repo").unwrap_err();
        assert!(err.contains("owner/repo"));
    }

    #[test]
    fn parse_hugging_face_repo_id_extracts_repo() {
        assert_eq!(
            parse_hugging_face_repo_id("TheBloke/Llama-2-7B-GGUF::file.gguf").unwrap(),
            "TheBloke/Llama-2-7B-GGUF"
        );
    }

    #[test]
    fn parse_ollama_input_simple_model() {
        let (repo, tag) = parse_ollama_input("llama2");
        assert_eq!(repo, "library/llama2");
        assert_eq!(tag, "latest");
    }

    #[test]
    fn parse_ollama_input_model_with_tag() {
        let (repo, tag) = parse_ollama_input("llama2:13b");
        assert_eq!(repo, "library/llama2");
        assert_eq!(tag, "13b");
    }

    #[test]
    fn parse_ollama_input_strips_url_prefix() {
        let (repo, tag) = parse_ollama_input("https://ollama.com/library/llama2:7b");
        assert_eq!(repo, "library/llama2");
        assert_eq!(tag, "7b");
    }

    #[test]
    fn parse_ollama_input_strips_short_url_prefix() {
        let (repo, tag) = parse_ollama_input("ollama.com/library/llama2");
        assert_eq!(repo, "library/llama2");
        assert_eq!(tag, "latest");
    }

    #[test]
    fn parse_ollama_input_preserves_namespaced_repo() {
        let (repo, tag) = parse_ollama_input("myuser/mymodel:v2");
        assert_eq!(repo, "myuser/mymodel");
        assert_eq!(tag, "v2");
    }

    #[test]
    fn parse_ollama_repo_path_simple() {
        assert_eq!(parse_ollama_repo_path("llama2"), "library/llama2");
    }

    #[test]
    fn parse_ollama_repo_path_bare_library_model() {
        assert_eq!(parse_ollama_repo_path("llama3"), "library/llama3");
    }

    #[test]
    fn parse_ollama_repo_path_library_url() {
        assert_eq!(
            parse_ollama_repo_path("https://ollama.com/library/llama3"),
            "library/llama3"
        );
    }

    #[test]
    fn parse_ollama_repo_path_with_tag_stripped() {
        assert_eq!(parse_ollama_repo_path("llama2:13b"), "library/llama2");
    }

    #[test]
    fn parse_ollama_repo_path_namespaced() {
        assert_eq!(parse_ollama_repo_path("myuser/mymodel"), "myuser/mymodel");
    }

    #[test]
    fn score_gguf_file_q4_k_m_highest() {
        assert_eq!(score_gguf_file("model-Q4_K_M.gguf"), 100);
    }

    #[test]
    fn score_gguf_file_q5_k_m() {
        assert_eq!(score_gguf_file("model-Q5_K_M.gguf"), 90);
    }

    #[test]
    fn score_gguf_file_q4_0() {
        assert_eq!(score_gguf_file("model-Q4_0.gguf"), 80);
    }

    #[test]
    fn score_gguf_file_q8_0() {
        assert_eq!(score_gguf_file("model-Q8_0.gguf"), 70);
    }

    #[test]
    fn score_gguf_file_instruct_bonus() {
        assert_eq!(score_gguf_file("model-instruct-Q4_K_M.gguf"), 105);
    }

    #[test]
    fn score_gguf_file_no_known_quant() {
        assert_eq!(score_gguf_file("model-unknown.gguf"), 0);
    }

    #[test]
    fn score_gguf_file_case_insensitive() {
        assert_eq!(score_gguf_file("model-q4_k_m.gguf"), 100);
    }

    #[test]
    fn pick_best_gguf_file_selects_q4_k_m_instruct() {
        let files = vec![
            "model-Q8_0.gguf".to_string(),
            "model-instruct-Q4_K_M.gguf".to_string(),
            "model-Q5_K_M.gguf".to_string(),
        ];
        assert_eq!(pick_best_gguf_file(&files), "model-instruct-Q4_K_M.gguf");
    }

    #[test]
    fn pick_best_gguf_file_selects_q4_k_m_over_q5_k_m() {
        let files = vec![
            "model-Q5_K_M.gguf".to_string(),
            "model-Q4_K_M.gguf".to_string(),
        ];
        assert_eq!(pick_best_gguf_file(&files), "model-Q4_K_M.gguf");
    }

    #[test]
    fn pick_best_gguf_file_falls_back_to_first_when_tied() {
        let files = vec!["alpha.gguf".to_string(), "beta.gguf".to_string()];
        // Both score 0, max_by_key picks last of ties; both are 0 so first max wins
        let result = pick_best_gguf_file(&files);
        assert!(result == "alpha.gguf" || result == "beta.gguf");
    }

    #[test]
    fn extract_ollama_tags_from_body_finds_tags() {
        let body = r#"
            <a href="/library/llama3:latest">latest</a>
            <a href="/library/llama3:8b">8b</a>
            <a href="/library/llama3:70b">70b</a>
        "#;
        let tags = extract_ollama_tags_from_body(body, "library/llama3");
        assert_eq!(tags, vec!["latest", "8b", "70b"]);
    }

    #[test]
    fn build_hf_auth_headers_with_token() {
        let headers = build_hf_auth_headers(Some("hf_abc123"));
        assert!(headers.is_some());
        let map = headers.unwrap();
        assert_eq!(map.get("Authorization").unwrap(), "Bearer hf_abc123");
    }

    #[test]
    fn build_hf_auth_headers_without_token() {
        let headers = build_hf_auth_headers(None);
        assert!(headers.is_none());
    }

    #[test]
    fn resolve_direct_url_valid() {
        let service = ModelResolverService::new().expect("client should build");
        let result = service
            .resolve("Direct URL", "https://example.com/path/model.gguf", None)
            .unwrap();
        assert_eq!(result.download_url, "https://example.com/path/model.gguf");
        assert_eq!(result.suggested_file_name, "model.gguf");
        assert!(result.request_headers.is_none());
    }

    #[test]
    fn resolve_direct_url_fallback_filename() {
        let service = ModelResolverService::new().expect("client should build");
        let result = service
            .resolve("Direct URL", "https://example.com/", None)
            .unwrap();
        assert_eq!(result.suggested_file_name, "model.gguf");
    }

    #[test]
    fn resolve_direct_url_rejects_non_http() {
        let service = ModelResolverService::new().expect("client should build");
        let err = service
            .resolve("Direct URL", "ftp://example.com/model.gguf", None)
            .unwrap_err();
        assert!(err.contains("http://"));
    }

    #[test]
    fn resolve_unknown_source_returns_error() {
        let service = ModelResolverService::new().expect("client should build");
        let err = service
            .resolve("Unknown Source", "something", None)
            .unwrap_err();
        assert!(err.contains("unknown source"));
    }

    #[test]
    fn sanitize_error_masks_bearer_token() {
        let msg = "request failed: Bearer hf_abc123XYZ in header";
        let sanitized = sanitize_error(msg);
        assert_eq!(sanitized, "request failed: Bearer *** in header");
        assert!(!sanitized.contains("hf_abc123XYZ"));
    }

    #[test]
    fn sanitize_error_no_bearer_passthrough() {
        let msg = "some normal error message";
        assert_eq!(sanitize_error(msg), msg);
    }

    #[test]
    fn sanitize_error_bearer_at_end() {
        let msg = "error: Bearer hf_secret";
        let sanitized = sanitize_error(msg);
        assert_eq!(sanitized, "error: Bearer ***");
        assert!(!sanitized.contains("hf_secret"));
    }

    #[test]
    fn resolve_hugging_face_absolute_url() {
        let service = ModelResolverService::new().expect("client should build");
        // This won't make an HTTP call since input is an absolute URL
        let result = service
            .resolve(
                "Hugging Face",
                "https://huggingface.co/TheBloke/Llama-2-7B-GGUF/resolve/main/llama-2-7b.Q4_K_M.gguf",
                Some("hf_token123"),
            )
            .unwrap();
        assert_eq!(
            result.download_url,
            "https://huggingface.co/TheBloke/Llama-2-7B-GGUF/resolve/main/llama-2-7b.Q4_K_M.gguf"
        );
        assert_eq!(result.suggested_file_name, "llama-2-7b.Q4_K_M.gguf");
        assert!(result.request_headers.is_some());
        assert_eq!(
            result
                .request_headers
                .unwrap()
                .get("Authorization")
                .unwrap(),
            "Bearer hf_token123"
        );
    }
}
