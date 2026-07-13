//! Production HTTP configuration and static hosting for the native server.

use std::{
    env,
    net::{IpAddr, SocketAddr},
    path::{Path, PathBuf},
};

use axum::http::{HeaderValue, Uri, header::ORIGIN};

pub const DEFAULT_PORT: u16 = 8787;
const DEFAULT_BIND_ADDR: &str = "127.0.0.1";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServerConfig {
    pub listen_addr: SocketAddr,
    pub web_dist: Option<PathBuf>,
    pub public_images: Option<PathBuf>,
    pub allowed_origins: AllowedOrigins,
}

impl ServerConfig {
    pub fn from_env() -> Result<Self, String> {
        Self::from_lookup(|key| env::var(key).ok())
    }

    fn from_lookup(mut lookup: impl FnMut(&str) -> Option<String>) -> Result<Self, String> {
        let bind_addr = lookup("BIND_ADDR").unwrap_or_else(|| DEFAULT_BIND_ADDR.to_owned());
        let bind_addr = bind_addr
            .parse::<IpAddr>()
            .map_err(|err| format!("BIND_ADDR must be an IP address: {err}"))?;

        let port = lookup("PORT")
            .map(|port| {
                port.parse::<u16>()
                    .map_err(|err| format!("PORT must be an integer from 1 to 65535: {err}"))
                    .and_then(|port| {
                        (port != 0)
                            .then_some(port)
                            .ok_or_else(|| "PORT must be an integer from 1 to 65535".to_owned())
                    })
            })
            .transpose()?
            .unwrap_or(DEFAULT_PORT);

        let web_dist =
            optional_directory(lookup("CAT_SERVER_WEB_DIST_DIR"), "CAT_SERVER_WEB_DIST_DIR")?;
        if let Some(dist) = &web_dist {
            let index = dist.join("index.html");
            if !index.is_file() {
                return Err(format!(
                    "CAT_SERVER_WEB_DIST_DIR must contain index.html: {}",
                    index.display()
                ));
            }
        }

        let public_images = optional_directory(
            lookup("CAT_SERVER_PUBLIC_IMAGES_DIR"),
            "CAT_SERVER_PUBLIC_IMAGES_DIR",
        )?;
        let allowed_origins = AllowedOrigins::parse(
            lookup("CAT_SERVER_ALLOWED_ORIGINS"),
            "CAT_SERVER_ALLOWED_ORIGINS",
        )?;

        Ok(Self {
            listen_addr: SocketAddr::new(bind_addr, port),
            web_dist,
            public_images,
            allowed_origins,
        })
    }
}

fn optional_directory(value: Option<String>, variable: &str) -> Result<Option<PathBuf>, String> {
    let Some(value) = value else {
        return Ok(None);
    };
    if value.trim().is_empty() {
        return Err(format!("{variable} cannot be empty when set"));
    }

    let path = PathBuf::from(value);
    if !path.is_dir() {
        return Err(format!("{variable} is not a directory: {}", path.display()));
    }
    Ok(Some(path))
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AllowedOrigins(Vec<HeaderValue>);

impl AllowedOrigins {
    pub(crate) fn parse(value: Option<String>, variable: &str) -> Result<Self, String> {
        let Some(value) = value else {
            return Ok(Self::default());
        };
        if value.trim().is_empty() {
            return Err(format!("{variable} cannot be empty when set"));
        }

        let mut origins = Vec::new();
        for origin in value.split(',').map(str::trim) {
            validate_origin(origin).map_err(|err| format!("invalid {variable} entry: {err}"))?;
            origins.push(
                HeaderValue::from_str(origin)
                    .map_err(|err| format!("invalid {variable} header value: {err}"))?,
            );
        }
        origins.sort_unstable_by(|left, right| left.as_bytes().cmp(right.as_bytes()));
        origins.dedup();
        Ok(Self(origins))
    }

    pub fn is_restricted(&self) -> bool {
        !self.0.is_empty()
    }

    pub fn allows(&self, origin: Option<&HeaderValue>) -> bool {
        self.0.is_empty() || origin.is_some_and(|origin| self.0.binary_search(origin).is_ok())
    }

    pub fn request_origin_allowed(&self, headers: &axum::http::HeaderMap) -> bool {
        self.allows(headers.get(ORIGIN))
    }
}

fn validate_origin(origin: &str) -> Result<(), String> {
    if origin.ends_with('/') {
        return Err(format!(
            "{origin:?} must not have a trailing slash (browser Origin headers omit it)"
        ));
    }
    let uri = origin
        .parse::<Uri>()
        .map_err(|err| format!("{origin:?} is not a valid URI: {err}"))?;
    let scheme = uri
        .scheme_str()
        .ok_or_else(|| format!("{origin:?} has no scheme"))?;
    if !matches!(scheme, "http" | "https") {
        return Err(format!("{origin:?} must use http or https"));
    }
    if uri.authority().is_none()
        || uri
            .authority()
            .is_some_and(|authority| authority.as_str().contains('@'))
        || (uri.path() != "/" && !uri.path().is_empty())
        || uri.query().is_some()
    {
        return Err(format!(
            "{origin:?} must be an origin only (for example https://cats.example)"
        ));
    }
    Ok(())
}

pub fn index_path(dist: &Path) -> PathBuf {
    dist.join("index.html")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{collections::BTreeMap, fs};

    struct TempDir(PathBuf);

    impl TempDir {
        fn new(name: &str) -> Self {
            let path = env::temp_dir().join(format!(
                "cat-server-hosting-{name}-{}-{}",
                std::process::id(),
                crate::now_ms()
            ));
            fs::create_dir_all(&path).expect("create temp dir");
            Self(path)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn config(values: &[(&str, &str)]) -> Result<ServerConfig, String> {
        let values = values
            .iter()
            .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
            .collect::<BTreeMap<_, _>>();
        ServerConfig::from_lookup(|key| values.get(key).cloned())
    }

    #[test]
    fn local_defaults_do_not_require_static_files_or_an_origin() {
        let config = config(&[]).expect("default config");
        assert_eq!(
            config.listen_addr,
            SocketAddr::from(([127, 0, 0, 1], DEFAULT_PORT))
        );
        assert!(config.web_dist.is_none());
        assert!(config.public_images.is_none());
        assert!(!config.allowed_origins.is_restricted());
        assert!(config.allowed_origins.allows(None));
    }

    #[test]
    fn production_paths_bind_address_and_origins_are_validated() {
        let dist = TempDir::new("dist");
        let images = TempDir::new("images");
        fs::write(dist.0.join("index.html"), "<!doctype html>").expect("write index");
        let config = config(&[
            ("BIND_ADDR", "0.0.0.0"),
            ("PORT", "9000"),
            ("CAT_SERVER_WEB_DIST_DIR", &dist.0.to_string_lossy()),
            ("CAT_SERVER_PUBLIC_IMAGES_DIR", &images.0.to_string_lossy()),
            (
                "CAT_SERVER_ALLOWED_ORIGINS",
                "https://cats.example, http://localhost:8080,https://cats.example",
            ),
        ])
        .expect("production config");

        assert_eq!(config.listen_addr, SocketAddr::from(([0, 0, 0, 0], 9000)));
        assert_eq!(config.web_dist.as_deref(), Some(dist.0.as_path()));
        assert_eq!(config.public_images.as_deref(), Some(images.0.as_path()));
        assert!(config.allowed_origins.is_restricted());
        assert!(
            config
                .allowed_origins
                .allows(Some(&HeaderValue::from_static("https://cats.example")))
        );
        assert!(!config.allowed_origins.allows(None));
        assert!(
            !config
                .allowed_origins
                .allows(Some(&HeaderValue::from_static("https://intruder.example")))
        );
    }

    #[test]
    fn malformed_production_configuration_fails_fast() {
        assert!(config(&[("BIND_ADDR", "localhost")]).is_err());
        assert!(config(&[("PORT", "0")]).is_err());
        assert!(config(&[("PORT", "cats")]).is_err());
        assert!(config(&[("CAT_SERVER_WEB_DIST_DIR", "/definitely/missing")]).is_err());
        assert!(config(&[("CAT_SERVER_ALLOWED_ORIGINS", "https://cats.example/path")]).is_err());
        assert!(config(&[("CAT_SERVER_ALLOWED_ORIGINS", "https://cats.example/")]).is_err());
        assert!(config(&[("CAT_SERVER_ALLOWED_ORIGINS", "")]).is_err());
    }
}
