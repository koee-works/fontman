//! Google Fonts: catalog, name previews, and downloads.
//!
//! Everything network-facing for Google Fonts lives here, behind the app's
//! Rust side. The webview never contacts Google: it would need the API key and
//! a looser content-security policy, and the host allowlist below could not be
//! enforced.
//!
//! Endpoint and response format: <https://developers.google.com/fonts/docs/developer_api>.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub const API_URL: &str = "https://www.googleapis.com/webfonts/v1/webfonts";

/// Hosts downloads may come from. The catalog is data from the network; it
/// must not be able to make the app fetch from anywhere else.
pub const FONT_HOSTS: &[&str] = &["fonts.gstatic.com"];

/// Largest font file we will download. Google's largest families are tens of
/// MB; anything bigger is not a font.
const MAX_FONT_BYTES: u64 = 64 * 1024 * 1024;
const MAX_CATALOG_BYTES: u64 = 32 * 1024 * 1024;

#[derive(Debug, thiserror::Error)]
pub enum GoogleError {
    /// Google rejected the key itself.
    #[error("Google didn't accept this API key. Check that it was copied in full.")]
    InvalidKey,
    /// The key is valid but not allowed to use this API: almost always the
    /// API isn't enabled on the key's project, or the key is restricted to
    /// other APIs.
    #[error(
        "Google accepted the key but won't let it use the Google Fonts API. Check that the Web \
         Fonts Developer API is enabled for the key's project, and that the key's API \
         restrictions include it. Google said: {0}"
    )]
    Forbidden(String),
    #[error("Couldn't reach Google: {0}")]
    Network(String),
    #[error("Google returned an unexpected response: {0}")]
    BadResponse(String),
    #[error("Refused to download from {0}; only Google's font servers are allowed.")]
    DisallowedHost(String),
    #[error("{0} is not a font file")]
    NotAFont(String),
    #[error("{0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, GoogleError>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Axis {
    pub tag: String,
    pub start: f32,
    pub end: f32,
}

/// One family as the API returns it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GoogleFamily {
    pub family: String,
    #[serde(default)]
    pub variants: Vec<String>,
    #[serde(default)]
    pub subsets: Vec<String>,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub last_modified: String,
    /// Variant → file URL.
    #[serde(default)]
    pub files: BTreeMap<String, String>,
    /// A tiny file containing only the glyphs of the family name.
    pub menu: Option<String>,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub axes: Vec<Axis>,
    /// Google's own tags, with `capability=FAMILY_TAGS`.
    #[serde(default)]
    pub tags: Vec<FamilyTag>,
}

/// A tag Google gives a family, such as `/Sans/Humanist`, with a strength
/// from 0 to 100.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FamilyTag {
    /// Google's documentation shows this under both `name` and `tag`.
    #[serde(alias = "tag")]
    pub name: String,
    #[serde(default, deserialize_with = "number_or_string")]
    pub weight: f32,
}

fn number_or_string<'de, D: serde::Deserializer<'de>>(d: D) -> std::result::Result<f32, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum N {
        Num(f32),
        Str(String),
    }
    Ok(match N::deserialize(d)? {
        N::Num(n) => n,
        N::Str(s) => s.trim().parse().unwrap_or(0.0),
    })
}

/// Strength presets for importing Google's tags (0 to 100).
pub const TAG_STRENGTHS: [f32; 3] = [1.0, 50.0, 80.0];

/// Google tags for the library families that are in the catalog, matched by
/// family name ignoring case: downloaded families, and local copies of Google
/// fonts installed any other way. Only tags at least `min_weight` strong.
///
/// A name Google uses in more than one group ("Humanist" under both Sans
/// and Serif) is qualified with its group everywhere, so the two read
/// differently in the sidebar.
pub fn taggings(
    catalog: &[GoogleFamily],
    library: &std::collections::HashSet<String>,
    min_weight: f32,
) -> Vec<crate::db::GoogleTagging> {
    use std::collections::{HashMap, HashSet};
    let mut groups_per_name: HashMap<String, HashSet<String>> = HashMap::new();
    for f in catalog {
        for t in &f.tags {
            if let Some((g, n)) = split_tag_path(&t.name) {
                groups_per_name.entry(n.to_lowercase()).or_default().insert(g);
            }
        }
    }
    let mut by_path: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for f in catalog {
        let key = crate::family::group_key(&f.family);
        if !library.contains(&key) {
            continue;
        }
        for t in f.tags.iter().filter(|t| t.weight >= min_weight) {
            let fams = by_path.entry(t.name.clone()).or_default();
            if !fams.contains(&key) {
                fams.push(key.clone());
            }
        }
    }
    by_path
        .into_iter()
        .filter_map(|(path, families)| {
            let (group, name) = split_tag_path(&path)?;
            let shared = groups_per_name.get(&name.to_lowercase()).is_some_and(|g| g.len() > 1);
            let name = if shared && !group.is_empty() { format!("{name} ({group})") } else { name };
            Some(crate::db::GoogleTagging { path, group, name, families })
        })
        .collect()
}

/// A Google tag path split into the group it's listed under and its own
/// name: `/Sans/Humanist` is "Humanist" in "Sans". Deeper paths keep the rest
/// in the name: `/Theme/Seasonal/Winter` is "Seasonal / Winter" in "Theme".
pub fn split_tag_path(path: &str) -> Option<(String, String)> {
    let parts: Vec<&str> = path.split('/').map(str::trim).filter(|p| !p.is_empty()).collect();
    match parts.as_slice() {
        [] => None,
        [only] => Some((String::new(), (*only).to_string())),
        [group, rest @ ..] => Some(((*group).to_string(), rest.join(" / "))),
    }
}

/// Where requests go and which hosts files may come from. Production uses
/// [`Endpoint::default`]; tests point it at a local mock server.
#[derive(Debug, Clone)]
pub struct Endpoint {
    pub api_url: String,
    pub font_hosts: Vec<String>,
}

impl Default for Endpoint {
    fn default() -> Self {
        Self {
            api_url: API_URL.to_string(),
            font_hosts: FONT_HOSTS.iter().map(|s| s.to_string()).collect(),
        }
    }
}

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(10))
        .timeout(Duration::from_secs(60))
        .user_agent(concat!("font-manager/", env!("CARGO_PKG_VERSION")))
        .build()
}

/// Google's JSON error body: `{"error": {"code": 400, "message": "..."}}`.
fn error_message(body: &str) -> String {
    serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|v| v["error"]["message"].as_str().map(str::to_string))
        .unwrap_or_else(|| body.chars().take(200).collect())
}

fn classify(err: ureq::Error) -> GoogleError {
    match err {
        ureq::Error::Status(code, resp) => {
            let body = resp.into_string().unwrap_or_default();
            let msg = error_message(&body);
            match code {
                // An invalid key is a 400 whose message says so; other 400s
                // are our own malformed request and should read as such.
                400 if msg.to_lowercase().contains("api key") => GoogleError::InvalidKey,
                401 => GoogleError::InvalidKey,
                403 => GoogleError::Forbidden(msg),
                _ => GoogleError::BadResponse(format!("HTTP {code}: {msg}")),
            }
        }
        // Not `t.to_string()`: ureq's message includes the request URL, and
        // the URL carries the API key.
        ureq::Error::Transport(t) => {
            use std::error::Error as _;
            let mut msg = t.kind().to_string();
            if let Some(cause) = t.source() {
                msg.push_str(": ");
                msg.push_str(&cause.to_string());
            }
            GoogleError::Network(msg)
        }
    }
}

/// Remove the key from an error before it can reach the UI or a log. A second
/// line of defence behind not formatting URLs into messages at all.
fn redact(err: GoogleError, key: &str) -> GoogleError {
    let key = key.trim();
    if key.len() < 8 {
        return err; // too short to redact without mangling ordinary words
    }
    let scrub = |s: String| s.replace(key, "[API key]");
    match err {
        GoogleError::Forbidden(s) => GoogleError::Forbidden(scrub(s)),
        GoogleError::Network(s) => GoogleError::Network(scrub(s)),
        GoogleError::BadResponse(s) => GoogleError::BadResponse(scrub(s)),
        other => other,
    }
}

fn read_limited(resp: ureq::Response, limit: u64) -> Result<Vec<u8>> {
    let mut buf = Vec::new();
    resp.into_reader().take(limit + 1).read_to_end(&mut buf)?;
    if buf.len() as u64 > limit {
        return Err(GoogleError::BadResponse(format!("response larger than {limit} bytes")));
    }
    Ok(buf)
}

/// The full catalog, most popular first, with variable font files where a
/// family has them.
///
/// Google's tags come too. If the API refuses the tags capability (the
/// documentation lists it inconsistently), the catalog is fetched without
/// them rather than not at all.
pub fn fetch_catalog(ep: &Endpoint, key: &str) -> Result<Vec<GoogleFamily>> {
    match fetch(ep, key, &[("sort", "popularity"), ("capability", "VF"), ("capability", "FAMILY_TAGS")]) {
        Err(GoogleError::BadResponse(_)) => {
            fetch(ep, key, &[("sort", "popularity"), ("capability", "VF")])
        }
        other => other,
    }
}

/// Check a key with the smallest useful request: one family.
pub fn check_key(ep: &Endpoint, key: &str) -> Result<()> {
    let found = fetch(ep, key, &[("family", "Roboto")])?;
    if found.is_empty() {
        return Err(GoogleError::BadResponse("no families returned".into()));
    }
    Ok(())
}

fn fetch(ep: &Endpoint, key: &str, params: &[(&str, &str)]) -> Result<Vec<GoogleFamily>> {
    #[derive(Deserialize)]
    struct Body {
        #[serde(default)]
        items: Vec<GoogleFamily>,
    }
    let mut req = agent().get(&ep.api_url).query("key", key.trim());
    for (k, v) in params {
        req = req.query(k, v);
    }
    let resp = req.call().map_err(|e| redact(classify(e), key))?;
    let bytes = read_limited(resp, MAX_CATALOG_BYTES).map_err(|e| redact(e, key))?;
    let body: Body = serde_json::from_slice(&bytes)
        .map_err(|e| GoogleError::BadResponse(format!("catalog: {e}")))?;
    Ok(body.items)
}

/// Normalise a file URL from the catalog and check its host.
///
/// The API returns `http://fonts.gstatic.com/...`; fetch over HTTPS instead.
/// Plain HTTP is only kept for allowlisted hosts that are loopback (tests).
pub fn checked_url(ep: &Endpoint, raw: &str) -> Result<String> {
    let parsed = url::Url::parse(raw).map_err(|_| GoogleError::DisallowedHost(raw.to_string()))?;
    let host = parsed.host_str().unwrap_or_default().to_string();
    if !ep.font_hosts.iter().any(|h| h == &host) {
        return Err(GoogleError::DisallowedHost(host));
    }
    let loopback = host == "127.0.0.1" || host == "localhost";
    let mut url = parsed;
    if url.scheme() == "http" && !loopback {
        url.set_scheme("https").ok();
    }
    if url.scheme() != "https" && !loopback {
        return Err(GoogleError::DisallowedHost(raw.to_string()));
    }
    Ok(url.to_string())
}

/// Download a font file, verifying it is actually a font.
pub fn download_font(ep: &Endpoint, raw_url: &str) -> Result<Vec<u8>> {
    let url = checked_url(ep, raw_url)?;
    let resp = agent().get(&url).call().map_err(classify)?;
    let bytes = read_limited(resp, MAX_FONT_BYTES)?;
    if !looks_like_font(&bytes) {
        return Err(GoogleError::NotAFont(url));
    }
    Ok(bytes)
}

fn looks_like_font(b: &[u8]) -> bool {
    matches!(b.get(0..4), Some(b"\0\x01\0\0" | b"OTTO" | b"true" | b"ttcf"))
        && ttf_parser::Face::parse(b, 0).is_ok()
}

// ---------------------------------------------------------------------------
// The managed download folder
// ---------------------------------------------------------------------------

/// File recording which catalog version of a family is on disk.
const VERSION_FILE: &str = ".google-fonts-version";

/// Folder name for a family. Family names are plain words, but this is data
/// from the network and becomes a path, so keep only safe characters.
pub fn family_dir(root: &Path, family: &str) -> PathBuf {
    let safe: String = family
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == ' ' || c == '-' { c } else { '_' })
        .collect();
    let safe = safe.trim().trim_start_matches('.').to_string();
    root.join(if safe.is_empty() { "_".to_string() } else { safe })
}

/// The catalog version downloaded for `family`, if any.
pub fn downloaded_version(root: &Path, family: &str) -> Option<String> {
    std::fs::read_to_string(family_dir(root, family).join(VERSION_FILE))
        .ok()
        .map(|s| s.trim().to_string())
}

fn variant_file_name(family: &str, variant: &str, url: &str) -> String {
    let ext = Path::new(url.split('?').next().unwrap_or(url))
        .extension()
        .and_then(|e| e.to_str())
        .filter(|e| matches!(*e, "ttf" | "otf"))
        .unwrap_or("ttf");
    let stem: String = format!("{family}-{variant}")
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' { c } else { '_' })
        .collect();
    format!("{stem}.{ext}")
}

/// Download every file of `fam` into its folder under `root`.
///
/// All files are fetched into a staging folder first and only swapped in when
/// every one succeeded, so a failed download or update never leaves a family
/// half-replaced. Returns the family's folder.
pub fn install_family(ep: &Endpoint, root: &Path, fam: &GoogleFamily) -> Result<PathBuf> {
    if fam.files.is_empty() {
        return Err(GoogleError::BadResponse(format!("{} has no files", fam.family)));
    }
    let dest = family_dir(root, &fam.family);
    let staging = dest.with_file_name(format!(
        ".staging-{}",
        dest.file_name().and_then(|n| n.to_str()).unwrap_or("family")
    ));
    let _ = std::fs::remove_dir_all(&staging);
    std::fs::create_dir_all(&staging)?;

    let result = (|| {
        // Variable families list the same file under several variants.
        let mut seen = std::collections::HashSet::new();
        for (variant, url) in &fam.files {
            if !seen.insert(url.clone()) {
                continue;
            }
            let bytes = download_font(ep, url)?;
            std::fs::write(staging.join(variant_file_name(&fam.family, variant, url)), bytes)?;
        }
        std::fs::write(staging.join(VERSION_FILE), &fam.version)?;
        Ok(())
    })();

    if let Err(e) = result {
        let _ = std::fs::remove_dir_all(&staging);
        return Err(e);
    }
    if dest.exists() {
        std::fs::remove_dir_all(&dest)?;
    }
    std::fs::rename(&staging, &dest)?;
    Ok(dest)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ep() -> Endpoint {
        Endpoint::default()
    }

    #[test]
    fn catalog_urls_are_upgraded_to_https() {
        assert_eq!(
            checked_url(&ep(), "http://fonts.gstatic.com/s/inter/v13/a.ttf").unwrap(),
            "https://fonts.gstatic.com/s/inter/v13/a.ttf"
        );
    }

    #[test]
    fn other_hosts_are_refused() {
        for bad in [
            "https://evil.example/a.ttf",
            "https://fonts.gstatic.com.evil.example/a.ttf",
            "file:///etc/passwd",
            "not a url",
        ] {
            assert!(checked_url(&ep(), bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn plain_http_only_survives_for_loopback_test_hosts() {
        let test = Endpoint { api_url: String::new(), font_hosts: vec!["127.0.0.1".into()] };
        assert!(checked_url(&test, "http://127.0.0.1:8765/a.ttf").unwrap().starts_with("http://"));
    }

    #[test]
    fn family_names_cannot_escape_the_download_folder() {
        let root = Path::new("/lib/google");
        for name in ["../../etc", "..", "/abs", "a/b", "C:\\x"] {
            let d = family_dir(root, name);
            assert_eq!(d.parent(), Some(root), "{name} → {}", d.display());
        }
        assert_eq!(family_dir(root, "Noto Sans JP"), root.join("Noto Sans JP"));
    }

    #[test]
    fn google_error_bodies_are_read() {
        let body = r#"{"error":{"code":400,"message":"API key not valid. Please pass a valid API key.","status":"INVALID_ARGUMENT"}}"#;
        assert_eq!(error_message(body), "API key not valid. Please pass a valid API key.");
        assert_eq!(error_message("<html>gateway</html>"), "<html>gateway</html>");
    }

    #[test]
    fn catalog_json_deserializes() {
        let json = r#"{"kind":"webfonts#webfontList","items":[{"family":"Inter","variants":["regular","italic"],
            "subsets":["latin"],"version":"v13","lastModified":"2024-01-01",
            "files":{"regular":"http://fonts.gstatic.com/s/inter/v13/a.ttf","italic":"http://fonts.gstatic.com/s/inter/v13/b.ttf"},
            "category":"sans-serif","kind":"webfonts#webfont","menu":"http://fonts.gstatic.com/s/inter/v13/m.ttf",
            "axes":[{"tag":"wght","start":100,"end":900}]}]}"#;
        #[derive(Deserialize)]
        struct Body {
            items: Vec<GoogleFamily>,
        }
        let b: Body = serde_json::from_str(json).unwrap();
        assert_eq!(b.items[0].family, "Inter");
        assert_eq!(b.items[0].axes[0].end, 900.0);
        assert_eq!(b.items[0].last_modified, "2024-01-01");
    }

    #[test]
    fn tags_deserialize_under_either_key() {
        let json = r#"[{"name":"/Expressive/Business","weight":58},{"tag":"/Sans/Humanist","weight":"100"}]"#;
        let tags: Vec<FamilyTag> = serde_json::from_str(json).unwrap();
        assert_eq!(tags[0], FamilyTag { name: "/Expressive/Business".into(), weight: 58.0 });
        assert_eq!(tags[1], FamilyTag { name: "/Sans/Humanist".into(), weight: 100.0 });
    }

    #[test]
    fn tag_paths_split_into_group_and_name() {
        assert_eq!(split_tag_path("/Sans/Humanist"), Some(("Sans".into(), "Humanist".into())));
        assert_eq!(
            split_tag_path("/Theme/Seasonal/Winter"),
            Some(("Theme".into(), "Seasonal / Winter".into()))
        );
        assert_eq!(split_tag_path("Cute"), Some((String::new(), "Cute".into())));
        assert_eq!(split_tag_path("//"), None);
    }

    #[test]
    fn taggings_match_library_families_above_a_strength() {
        let fam = |name: &str, tags: &[(&str, f32)]| GoogleFamily {
            family: name.into(),
            variants: vec![],
            subsets: vec![],
            version: String::new(),
            last_modified: String::new(),
            files: BTreeMap::new(),
            menu: None,
            category: String::new(),
            axes: vec![],
            tags: tags.iter().map(|(n, w)| FamilyTag { name: (*n).into(), weight: *w }).collect(),
        };
        let catalog = vec![
            fam("Open Sans", &[("/Sans/Humanist", 100.0), ("/Expressive/Calm", 30.0)]),
            fam("Lora", &[("/Serif/Humanist", 90.0), ("/Expressive/Calm", 70.0)]),
            fam("Not Here", &[("/Sans/Geometric", 100.0)]),
        ];
        let library: std::collections::HashSet<String> = ["open sans", "lora"].map(String::from).into();
        let t = taggings(&catalog, &library, 50.0);
        let summary: Vec<(&str, &str, Vec<&str>)> = t
            .iter()
            .map(|t| (t.path.as_str(), t.name.as_str(), t.families.iter().map(String::as_str).collect()))
            .collect();
        assert_eq!(
            summary,
            [
                ("/Expressive/Calm", "Calm", vec!["lora"]),
                ("/Sans/Humanist", "Humanist (Sans)", vec!["open sans"]),
                ("/Serif/Humanist", "Humanist (Serif)", vec!["lora"]),
            ]
        );
        assert_eq!(taggings(&catalog, &library, 1.0).iter().find(|t| t.name == "Calm").unwrap().families.len(), 2);
    }

    #[test]
    fn variant_file_names_are_safe() {
        assert_eq!(
            variant_file_name("Noto Sans", "700italic", "https://x/a.ttf?v=1"),
            "Noto_Sans-700italic.ttf"
        );
    }
}
