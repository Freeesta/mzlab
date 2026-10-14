//! Notice of a newer release (`src/release.rs`): version comparison and the request against a local HTTP server.

use mzlab_desktop::release::{check, newer_tag, parse_version, PAGE};
use std::io::Read;
use std::io::Write;
use std::net::TcpListener;

fn rel(tag: &str, draft: bool, pre: bool) -> Vec<u8> {
    format!(r#"{{"tag_name":"{tag}","draft":{draft},"prerelease":{pre},"name":"x"}}"#).into_bytes()
}

#[test]
fn versions_parse_strictly() {
    assert_eq!(parse_version("v1.2.3"), Some([1, 2, 3]));
    assert_eq!(parse_version("1.10.0"), Some([1, 10, 0]));
    for bad in [
        "",
        "1.2",
        "1.2.3.4",
        "1.2.x",
        "v1.2.3-rc1",
        "1..3",
        "latest",
    ] {
        assert_eq!(parse_version(bad), None, "{bad}");
    }
}

#[test]
fn only_a_newer_stable_release_is_announced() {
    assert_eq!(
        newer_tag(&rel("v1.0.1", false, false), "1.0.0").as_deref(),
        Some("1.0.1")
    );
    assert_eq!(
        newer_tag(&rel("v1.10.0", false, false), "1.9.9").as_deref(),
        Some("1.10.0")
    );
    assert_eq!(newer_tag(&rel("v1.0.0", false, false), "1.0.0"), None);
    assert_eq!(newer_tag(&rel("v0.9.0", false, false), "1.0.0"), None);
    assert_eq!(newer_tag(&rel("v2.0.0", true, false), "1.0.0"), None);
    assert_eq!(newer_tag(&rel("v2.0.0", false, true), "1.0.0"), None);
    assert_eq!(newer_tag(&rel("v2.0.0-rc1", false, false), "1.0.0"), None);
    assert_eq!(newer_tag(b"not json", "1.0.0"), None);
    assert_eq!(newer_tag(&rel("v2.0.0", false, false), "dev"), None);
}

fn serve_once(response: &'static str) -> String {
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/", l.local_addr().unwrap());
    std::thread::spawn(move || {
        if let Ok((mut s, _)) = l.accept() {
            let mut buf = [0u8; 2048];
            let _ = s.read(&mut buf);
            let _ = s.write_all(response.as_bytes());
        }
    });
    url
}

#[test]
fn check_reports_newer_and_stays_silent_on_errors() {
    let body = r#"{"tag_name":"v3.0.0","draft":false,"prerelease":false}"#;
    let ok: &'static str = Box::leak(
        format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
        .into_boxed_str(),
    );
    let v: serde_json::Value = serde_json::from_str(&check(&serve_once(ok), "1.0.0")).unwrap();
    assert_eq!(v["status"], "newer");
    assert_eq!(v["version"], "3.0.0");
    assert_eq!(v["url"], PAGE);
    let err =
        serve_once("HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
    assert_eq!(check(&err, "1.0.0"), r#"{"status":"none"}"#);
    assert_eq!(
        check("http://127.0.0.1:1/", "1.0.0"),
        r#"{"status":"none"}"#
    );
}
