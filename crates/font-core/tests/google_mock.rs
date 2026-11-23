//! The Google Fonts client against a local server that answers like Google:
//! same JSON shape, same error bodies, real font files.

use font_core::google::{self, Endpoint, GoogleError, GoogleFamily};
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::path::PathBuf;

const GOOD_KEY: &str = "good-key";
const DISABLED_KEY: &str = "api-not-enabled";

fn some_font() -> Option<Vec<u8>> {
    ["/usr/share/fonts", "/mnt/skills/examples/canvas-design/canvas-fonts"]
        .iter()
        .flat_map(|d| walkdir_like(PathBuf::from(d)))
        .find(|p| p.extension().is_some_and(|e| e == "ttf"))
        .and_then(|p| std::fs::read(p).ok())
}

fn walkdir_like(dir: PathBuf) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![dir];
    while let Some(d) = stack.pop() {
        for e in std::fs::read_dir(&d).into_iter().flatten().flatten() {
            let p = e.path();
            if p.is_dir() { stack.push(p) } else { out.push(p) }
        }
    }
    out
}

/// Serve: the catalog (key-checked), `/files/font.ttf`, `/files/not-a-font.ttf`.
fn serve(font: Vec<u8>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let files = base.clone();
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut line = String::new();
            reader.read_line(&mut line).ok();
            loop {
                let mut h = String::new();
                if reader.read_line(&mut h).unwrap_or(0) <= 2 { break; }
            }
            let path = line.split_whitespace().nth(1).unwrap_or("/").to_string();
            let (status, ctype, body): (&str, &str, Vec<u8>) = if path.starts_with("/webfonts") {
                if path.contains(&format!("key={GOOD_KEY}")) {
                    let catalog = format!(
                        r#"{{"kind":"webfonts#webfontList","items":[
                          {{"family":"Mock Sans","variants":["regular","700"],"subsets":["latin"],
                            "version":"v2","lastModified":"2026-01-01","category":"sans-serif",
                            "files":{{"regular":"{files}/files/font.ttf","700":"{files}/files/font.ttf"}},
                            "menu":"{files}/files/font.ttf"}},
                          {{"family":"Broken Serif","variants":["regular","italic"],"version":"v1",
                            "files":{{"regular":"{files}/files/font.ttf","italic":"{files}/files/not-a-font.ttf"}}}},
                          {{"family":"Elsewhere","variants":["regular"],"version":"v1",
                            "files":{{"regular":"https://evil.example/x.ttf"}}}}
                        ]}}"#
                    );
                    ("200 OK", "application/json", catalog.into_bytes())
                } else if path.contains(&format!("key={DISABLED_KEY}")) {
                    ("403 Forbidden", "application/json",
                     br#"{"error":{"code":403,"message":"Web Fonts Developer API has not been used in project 1 before or it is disabled.","status":"PERMISSION_DENIED"}}"#.to_vec())
                } else {
                    ("400 Bad Request", "application/json",
                     br#"{"error":{"code":400,"message":"API key not valid. Please pass a valid API key.","status":"INVALID_ARGUMENT"}}"#.to_vec())
                }
            } else if path == "/files/font.ttf" {
                ("200 OK", "font/ttf", font.clone())
            } else if path == "/files/not-a-font.ttf" {
                ("200 OK", "font/ttf", b"<html>captive portal</html>".to_vec())
            } else {
                ("404 Not Found", "text/plain", b"nope".to_vec())
            };
            let mut s = stream;
            let _ = write!(s, "HTTP/1.1 {status}\r\nContent-Type: {ctype}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len());
            let _ = s.write_all(&body);
        }
    });
    base
}

fn endpoint(base: &str) -> Endpoint {
    Endpoint { api_url: format!("{base}/webfonts/v1/webfonts"), font_hosts: vec!["127.0.0.1".into()] }
}

fn family<'a>(cat: &'a [GoogleFamily], name: &str) -> &'a GoogleFamily {
    cat.iter().find(|f| f.family == name).unwrap()
}

#[test]
fn keys_are_checked_and_errors_classified() {
    let Some(font) = some_font() else { return };
    let ep = endpoint(&serve(font));

    assert!(google::check_key(&ep, GOOD_KEY).is_ok());
    assert!(google::check_key(&ep, &format!("  {GOOD_KEY}\n")).is_ok(), "pasted whitespace is trimmed");
    assert!(matches!(google::check_key(&ep, "typo"), Err(GoogleError::InvalidKey)));
    match google::check_key(&ep, DISABLED_KEY) {
        Err(GoogleError::Forbidden(msg)) => assert!(msg.contains("disabled")),
        other => panic!("expected Forbidden, got {other:?}"),
    }

    let dead = Endpoint { api_url: "http://127.0.0.1:1/webfonts".into(), font_hosts: vec![] };
    let secret = "AIzaSyTHIS-MUST-NEVER-BE-SHOWN-0000000";
    match google::check_key(&dead, secret) {
        Err(e @ GoogleError::Network(_)) => {
            let shown = e.to_string();
            assert!(!shown.contains(secret), "key leaked into error: {shown}");
            assert!(!shown.contains("key="), "URL leaked into error: {shown}");
        }
        other => panic!("expected Network, got {other:?}"),
    }
}

#[test]
fn families_install_and_reinstall_atomically() {
    let Some(font) = some_font() else { return };
    let ep = endpoint(&serve(font));
    let cat = google::fetch_catalog(&ep, GOOD_KEY).unwrap();
    assert_eq!(cat.len(), 3);

    let root = tempfile::tempdir().unwrap();
    let dir = google::install_family(&ep, root.path(), family(&cat, "Mock Sans")).unwrap();
    let fonts: Vec<_> = std::fs::read_dir(&dir).unwrap().flatten()
        .filter(|e| e.path().extension().is_some_and(|x| x == "ttf")).collect();
    assert_eq!(fonts.len(), 1, "two variants share one URL: downloaded once");
    assert_eq!(google::downloaded_version(root.path(), "Mock Sans").as_deref(), Some("v2"));

    // The downloaded folder scans like any other library folder.
    let report = font_core::scan::scan(std::slice::from_ref(&dir), &Default::default());
    assert_eq!(report.failures.len(), 0);
    assert!(!report.faces.is_empty());

    // A family with one bad file must not be installed at all.
    let broken = family(&cat, "Broken Serif");
    assert!(matches!(google::install_family(&ep, root.path(), broken), Err(GoogleError::NotAFont(_))));
    assert!(!google::family_dir(root.path(), "Broken Serif").exists());

    // An update that fails leaves the existing copy untouched.
    let mut bad_update = family(&cat, "Mock Sans").clone();
    bad_update.version = "v3".into();
    bad_update.files.insert("italic".into(), format!("{}/files/not-a-font.ttf", ep.api_url.replace("/webfonts/v1/webfonts", "")));
    assert!(google::install_family(&ep, root.path(), &bad_update).is_err());
    assert_eq!(google::downloaded_version(root.path(), "Mock Sans").as_deref(), Some("v2"));
    assert!(dir.join(fonts[0].file_name()).exists());

    // A catalog entry pointing off Google's servers is refused.
    assert!(matches!(
        google::install_family(&ep, root.path(), family(&cat, "Elsewhere")),
        Err(GoogleError::DisallowedHost(_))
    ));

    // No staging debris in any case.
    assert!(std::fs::read_dir(root.path()).unwrap().flatten()
        .all(|e| !e.file_name().to_string_lossy().starts_with(".staging")));
}
