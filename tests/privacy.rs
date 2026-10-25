//! Guards for the claim in `docs/security-and-privacy.md`: Pyfaro itself makes
//! no network connections and sends no telemetry. Network access belongs to
//! the tools it delegates to, and is documented there.
//!
//! The dependency checks read `Cargo.lock`, which lists every crate Pyfaro can
//! be built with on any platform, test-only crates included, so they cover
//! Linux, macOS and Windows at once from one test run.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use toml_edit::DocumentMut;

/// Crates that would give Pyfaro a way to talk to the network, report usage,
/// or replace itself. A new entry in `Cargo.lock` with one of these names fails
/// the test; adding one is a change to the privacy promise and must come with
/// an update to the docs, not just to this list.
const DENIED: &[(&str, &[&str])] = &[
    (
        "HTTP clients and servers",
        &[
            "reqwest",
            "hyper",
            "hyper-util",
            "ureq",
            "curl",
            "curl-sys",
            "isahc",
            "attohttpc",
            "surf",
            "minreq",
            "http-client",
            "ehttp",
            "h2",
            "h3",
            "awc",
            "actix-web",
            "axum",
            "warp",
            "tiny_http",
        ],
    ),
    (
        "TLS stacks (only useful for outbound HTTPS)",
        &[
            "rustls",
            "native-tls",
            "openssl",
            "openssl-sys",
            "schannel",
            "security-framework",
            "tokio-rustls",
            "tokio-native-tls",
            "hyper-rustls",
            "hyper-tls",
            "rustls-native-certs",
            "webpki-roots",
        ],
    ),
    (
        "name resolution and sockets above std",
        &["hickory-resolver", "trust-dns-resolver", "socket2", "quinn"],
    ),
    ("git and download transports", &["git2", "gix-transport", "gix-protocol"]),
    (
        "telemetry and error reporting",
        &[
            "sentry",
            "sentry-core",
            "opentelemetry",
            "tracing-opentelemetry",
            "posthog-rs",
            "rollbar",
            "bugsnag",
        ],
    ),
    (
        "self-update",
        &["self_update", "self-replace", "axoupdater", "cargo-dist"],
    ),
];

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `(name, dependency names)` of every package in `Cargo.lock`.
fn lock_packages() -> Vec<(String, Vec<String>)> {
    let text = fs::read_to_string(manifest_dir().join("Cargo.lock")).unwrap();
    let document = text.parse::<DocumentMut>().unwrap();
    document["package"]
        .as_array_of_tables()
        .unwrap()
        .iter()
        .map(|package| {
            let name = package["name"].as_str().unwrap().to_owned();
            let dependencies = package
                .get("dependencies")
                .and_then(|value| value.as_array())
                .into_iter()
                .flatten()
                .filter_map(|entry| entry.as_str())
                // Entries read "name" or "name version [(source)]".
                .map(|entry| entry.split(' ').next().unwrap().to_owned())
                .collect();
            (name, dependencies)
        })
        .collect()
}

#[test]
fn no_network_telemetry_or_self_update_crate_is_in_the_dependency_graph() {
    let present: BTreeSet<String> =
        lock_packages().into_iter().map(|(name, _)| name).collect();
    let mut found = Vec::new();
    for (category, names) in DENIED {
        for name in *names {
            if present.contains(*name) {
                found.push(format!("{name} ({category})"));
            }
        }
    }
    assert!(
        found.is_empty(),
        "Cargo.lock contains crates that would break the \"Pyfaro makes no network connections\" promise in docs/security-and-privacy.md: {found:?}"
    );
}

/// `tokio` is not on the deny list: it is a general runtime, and it is already
/// in the lock file because `subc-jobobject` (the Windows Job Object wrapper)
/// enables its `process` feature. A check on names cannot see features, so
/// instead pin who may depend on it. Pyfaro must not use it directly.
#[test]
fn tokio_is_only_a_dependency_of_the_windows_job_object_crate() {
    let dependents: BTreeSet<String> = lock_packages()
        .into_iter()
        .filter(|(_, dependencies)| {
            dependencies.iter().any(|name| name == "tokio")
        })
        .map(|(name, _)| name)
        .collect();
    let allowed = BTreeSet::from(["subc-jobobject".to_owned()]);
    assert_eq!(
        dependents, allowed,
        "tokio may only be pulled in by subc-jobobject; update docs/security-and-privacy.md before widening this"
    );
}

fn rust_files(directory: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(directory).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            rust_files(&path, files);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path);
        }
    }
}

/// The only socket Pyfaro opens is a `TcpListener` bound by `pyfaro dev` to see
/// whether the requested port is free. Outgoing connections and datagrams have
/// no place in the sources.
#[test]
fn sources_open_no_outgoing_sockets() {
    let mut files = Vec::new();
    rust_files(&manifest_dir().join("src"), &mut files);
    assert!(!files.is_empty());
    let mut found = Vec::new();
    for file in files {
        let text = fs::read_to_string(&file).unwrap();
        for needle in ["TcpStream", "UdpSocket", "UnixStream", "ToSocketAddrs"]
        {
            if text.contains(needle) {
                found.push(format!("{} uses {needle}", file.display()));
            }
        }
    }
    assert!(found.is_empty(), "{found:?}");
}
