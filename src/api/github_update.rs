use data_encoding::BASE64;
use serde::Deserialize;

#[derive(Debug, Clone, Copy)]
pub enum PackageKind {
    Plugin,
    Theme,
}

#[derive(Debug)]
struct GitHubSource {
    repo: String,
    subpath: Option<String>,
}

impl PackageKind {
    fn manifest_name(self) -> &'static str {
        match self {
            PackageKind::Plugin => "plugin.json",
            PackageKind::Theme => "theme.json",
        }
    }
}

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
}

#[derive(Debug, Deserialize)]
struct GitHubTag {
    name: String,
}

pub async fn fetch_latest_version(
    client: &reqwest::Client,
    repository: &str,
    kind: PackageKind,
) -> Result<Option<String>, String> {
    let source = parse_github_source(repository)
        .ok_or_else(|| "Invalid GitHub repository URL".to_string())?;

    if let Some(version) = fetch_latest_release_version(client, &source.repo).await? {
        return Ok(Some(version));
    }

    if source.subpath.is_none() {
        if let Some(version) = fetch_latest_tag_version(client, &source.repo).await? {
            return Ok(Some(version));
        }
    }

    fetch_manifest_version(client, &source, kind).await
}

pub fn is_newer_version(current: &str, latest: &str) -> bool {
    let current = normalize_version(current);
    let latest = normalize_version(latest);

    if current == latest {
        return false;
    }

    match (parse_semverish(&current), parse_semverish(&latest)) {
        (Some(current), Some(latest)) => latest > current,
        _ => latest != current,
    }
}

fn parse_github_source(input: &str) -> Option<GitHubSource> {
    let mut input = input.trim().trim_end_matches('/').trim_end_matches(".git");
    if let Some((without_query, _)) = input.split_once('?') {
        input = without_query.trim_end_matches('/');
    }
    if let Some((without_fragment, _)) = input.split_once('#') {
        input = without_fragment.trim_end_matches('/');
    }

    let path = if let Some((_, path)) = input.split_once("github.com/") {
        path
    } else {
        input
    };

    let parts: Vec<&str> = path.split('/').filter(|part| !part.is_empty()).collect();
    if parts.len() < 2 {
        return None;
    }

    let owner = parts[0];
    let repo = parts[1].trim_end_matches(".git");
    if owner.is_empty() || repo.is_empty() {
        return None;
    }

    let subpath = if parts.len() >= 5 && (parts[2] == "tree" || parts[2] == "blob") {
        Some(parts[4..].join("/"))
    } else {
        None
    };

    Some(GitHubSource {
        repo: format!("{}/{}", owner, repo),
        subpath,
    })
}

async fn fetch_latest_release_version(
    client: &reqwest::Client,
    repo: &str,
) -> Result<Option<String>, String> {
    let url = format!("https://api.github.com/repos/{}/releases/latest", repo);
    let response = client
        .get(&url)
        .header("Accept", "application/vnd.github.v3+json")
        .send()
        .await
        .map_err(|e| format!("GitHub release request failed: {}", e))?;

    if response.status().as_u16() == 404 {
        return Ok(None);
    }
    if !response.status().is_success() {
        return Err(format!(
            "GitHub release request returned {}",
            response.status()
        ));
    }

    let release: GitHubRelease = response
        .json()
        .await
        .map_err(|e| format!("GitHub release parse failed: {}", e))?;

    Ok(Some(normalize_version(&release.tag_name)))
}

async fn fetch_latest_tag_version(
    client: &reqwest::Client,
    repo: &str,
) -> Result<Option<String>, String> {
    let url = format!("https://api.github.com/repos/{}/tags?per_page=1", repo);
    let response = client
        .get(&url)
        .header("Accept", "application/vnd.github.v3+json")
        .send()
        .await
        .map_err(|e| format!("GitHub tag request failed: {}", e))?;

    if !response.status().is_success() {
        return Ok(None);
    }

    let tags: Vec<GitHubTag> = response
        .json()
        .await
        .map_err(|e| format!("GitHub tag parse failed: {}", e))?;

    Ok(tags.first().map(|tag| normalize_version(&tag.name)))
}

async fn fetch_manifest_version(
    client: &reqwest::Client,
    source: &GitHubSource,
    kind: PackageKind,
) -> Result<Option<String>, String> {
    let manifest_name = kind.manifest_name();
    let file_path = match &source.subpath {
        Some(subpath) => format!("{}/{}", subpath, manifest_name),
        None => manifest_name.to_string(),
    };
    let encoded_file_path = file_path
        .split('/')
        .map(|segment| urlencoding::encode(segment).into_owned())
        .collect::<Vec<_>>()
        .join("/");
    let url = format!(
        "https://api.github.com/repos/{}/contents/{}",
        source.repo, encoded_file_path
    );
    let response = client
        .get(&url)
        .header("Accept", "application/vnd.github.v3+json")
        .send()
        .await
        .map_err(|e| format!("GitHub manifest request failed: {}", e))?;

    if !response.status().is_success() {
        return Ok(None);
    }

    let json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("GitHub manifest parse failed: {}", e))?;
    let encoded = json
        .get("content")
        .and_then(|value| value.as_str())
        .ok_or_else(|| "GitHub manifest response missing content".to_string())?;
    let cleaned: String = encoded.chars().filter(|c| !c.is_whitespace()).collect();
    let decoded = BASE64
        .decode(cleaned.as_bytes())
        .map_err(|e| format!("GitHub manifest decode failed: {}", e))?;
    let manifest: serde_json::Value = serde_json::from_slice(&decoded)
        .map_err(|e| format!("GitHub manifest JSON parse failed: {}", e))?;

    Ok(manifest
        .get("version")
        .and_then(|value| value.as_str())
        .map(normalize_version))
}

fn normalize_version(value: &str) -> String {
    value
        .trim()
        .trim_start_matches('v')
        .trim_start_matches('V')
        .to_string()
}

fn parse_semverish(value: &str) -> Option<(Vec<u64>, bool, String)> {
    let value = value.split('+').next().unwrap_or(value);
    let (core, pre_release) = match value.split_once('-') {
        Some((core, pre)) => (core, Some(pre.to_string())),
        None => (value, None),
    };

    let mut parts = Vec::new();
    for part in core.split('.') {
        if part.is_empty() {
            return None;
        }
        parts.push(part.parse::<u64>().ok()?);
    }

    while parts.len() < 3 {
        parts.push(0);
    }

    Some((
        parts,
        pre_release.is_none(),
        pre_release.unwrap_or_default(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compares_semverish_versions() {
        assert!(!is_newer_version("1.2.0", "v1.2.0"));
        assert!(is_newer_version("1.9.0", "1.10.0"));
        assert!(is_newer_version("1.2.0-beta", "1.2.0"));
        assert!(!is_newer_version("1.2.0", "1.2.0-beta"));
    }

    #[test]
    fn parses_github_urls_with_optional_subpaths() {
        let source = parse_github_source("https://github.com/owner/repo/tree/main/plugins/demo")
            .expect("valid GitHub URL");
        assert_eq!(source.repo, "owner/repo");
        assert_eq!(source.subpath.as_deref(), Some("plugins/demo"));

        let source = parse_github_source("owner/repo").expect("valid owner/repo");
        assert_eq!(source.repo, "owner/repo");
        assert!(source.subpath.is_none());
    }
}
