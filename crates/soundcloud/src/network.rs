//! Immutable child-process network policy; never mutates the parent environment.
use std::process::Command;

#[derive(Clone, Default)]
pub struct NetworkPolicy {
    values: Vec<(String, String)>,
    diagnostic: &'static str,
}

impl std::fmt::Debug for NetworkPolicy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.diagnostic())
    }
}

impl NetworkPolicy {
    pub fn diagnostic(&self) -> &'static str {
        if self.diagnostic.is_empty() {
            "direct routing"
        } else {
            self.diagnostic
        }
    }

    pub fn detect() -> Self {
        let env: Vec<_> = std::env::vars().collect();
        Self::from_settings(&env, system_settings())
    }

    fn from_settings(env: &[(String, String)], settings: Result<SystemSettings, ()>) -> Self {
        let keys = ["http_proxy", "https_proxy", "all_proxy", "no_proxy"];
        let values: Vec<_> = keys
            .iter()
            .filter_map(|key| {
                env.iter()
                    .find(|(k, _)| k.eq_ignore_ascii_case(key))
                    .map(|(_, v)| (key.to_string(), v.clone()))
            })
            .collect();
        // Any explicit routing override selects the environment as a complete policy.
        if values.iter().any(|(k, _)| k != "no_proxy") {
            return Self {
                values,
                diagnostic: "explicit environment proxy policy",
            };
        }
        match settings {
            Ok(s) if s.proxy.is_some() => match parse_manual(s.proxy.as_deref().unwrap()) {
                Ok(mut policy) => {
                    let bypass = values
                        .iter()
                        .find(|(k, _)| k == "no_proxy")
                        .map(|(_, v)| v.clone())
                        .unwrap_or_else(|| bypass_list(&s.bypass));
                    policy.values.push(("no_proxy".into(), bypass));
                    policy
                }
                Err(()) => Self {
                    values,
                    diagnostic: "unsupported Windows manual proxy; direct routing",
                },
            },
            Ok(s) if s.automatic => Self {
                values,
                diagnostic: "Windows PAC/WPAD unsupported; direct routing",
            },
            Ok(_) => Self {
                values,
                diagnostic: "Windows proxy disabled; direct routing (including TUN)",
            },
            Err(()) => Self {
                values,
                diagnostic: "Windows proxy query failed; direct routing",
            },
        }
    }

    pub fn apply(&self, command: &mut Command) {
        for key in ["http_proxy", "https_proxy", "all_proxy", "no_proxy"] {
            command.env_remove(key).env_remove(key.to_ascii_uppercase());
        }
        for (key, value) in &self.values {
            command.env(key, value).env(key.to_ascii_uppercase(), value);
        }
    }
}

#[derive(Default)]
struct SystemSettings {
    proxy: Option<String>,
    bypass: String,
    automatic: bool,
}

fn endpoint(value: &str) -> Result<String, ()> {
    let value = value.trim();
    let authority = value.strip_prefix("http://").unwrap_or(value);
    if authority.is_empty()
        || authority.contains(['/', '\\', '?', '#', ';', '='])
        || authority.chars().any(char::is_whitespace)
    {
        return Err(());
    }
    Ok(format!("http://{authority}"))
}

fn parse_manual(value: &str) -> Result<NetworkPolicy, ()> {
    let mut values = Vec::new();
    if !value.contains('=') {
        let proxy = endpoint(value)?;
        values.push(("http_proxy".into(), proxy.clone()));
        values.push(("https_proxy".into(), proxy));
    } else {
        for part in value.split(';').filter(|p| !p.trim().is_empty()) {
            let (protocol, proxy) = part.trim().split_once('=').ok_or(())?;
            let key = match protocol.to_ascii_lowercase().as_str() {
                "http" => "http_proxy",
                "https" => "https_proxy",
                _ => return Err(()),
            };
            if values.iter().any(|(k, _)| k == key) {
                return Err(());
            }
            values.push((key.into(), endpoint(proxy)?));
        }
        if values.is_empty() {
            return Err(());
        }
    }
    Ok(NetworkPolicy {
        values,
        diagnostic: "Windows manual HTTP proxy policy",
    })
}

fn bypass_list(value: &str) -> String {
    let mut hosts = vec!["localhost".to_string(), "127.0.0.1".into(), "::1".into()];
    for host in value.split(';').map(str::trim).filter(|s| !s.is_empty()) {
        // <local> means unqualified names in Windows, which NO_PROXY cannot express.
        if host == "<local>" {
            continue;
        }
        hosts.push(
            host.strip_prefix("*.")
                .map(|h| format!(".{h}"))
                .unwrap_or_else(|| host.into()),
        );
    }
    hosts.join(",")
}

#[cfg(not(windows))]
fn system_settings() -> Result<SystemSettings, ()> {
    Ok(SystemSettings::default())
}

#[cfg(windows)]
fn system_settings() -> Result<SystemSettings, ()> {
    use std::ffi::c_void;
    #[repr(C)]
    struct Config {
        auto: i32,
        pac: *mut u16,
        proxy: *mut u16,
        bypass: *mut u16,
    }
    #[link(name = "winhttp")]
    unsafe extern "system" {
        fn WinHttpGetIEProxyConfigForCurrentUser(config: *mut Config) -> i32;
    }
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GlobalFree(ptr: *mut c_void) -> *mut c_void;
    }
    unsafe fn text(ptr: *const u16) -> String {
        if ptr.is_null() {
            return String::new();
        }
        let mut len = 0;
        // Windows owns a null-terminated UTF-16 allocation until GlobalFree below.
        unsafe {
            while *ptr.add(len) != 0 {
                len += 1;
            }
            String::from_utf16_lossy(std::slice::from_raw_parts(ptr, len))
        }
    }
    let mut config = Config {
        auto: 0,
        pac: std::ptr::null_mut(),
        proxy: std::ptr::null_mut(),
        bypass: std::ptr::null_mut(),
    };
    // API reads the current user's Internet Options, not service WinHTTP defaults.
    unsafe {
        let ok = WinHttpGetIEProxyConfigForCurrentUser(&mut config) != 0;
        let proxy = text(config.proxy);
        let result = SystemSettings {
            proxy: (!proxy.is_empty()).then_some(proxy),
            bypass: text(config.bypass),
            automatic: config.auto != 0 || !config.pac.is_null(),
        };
        for ptr in [config.pac, config.proxy, config.bypass] {
            if !ptr.is_null() {
                GlobalFree(ptr.cast());
            }
        }
        if ok { Ok(result) } else { Err(()) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn child_environment_is_explicit_and_shared() {
        let policy = NetworkPolicy::from_settings(
            &[],
            Ok(SystemSettings {
                proxy: Some("https=proxy.example:8080".into()),
                bypass: "localhost".into(),
                automatic: false,
            }),
        );
        let mut extractor = Command::new("extractor");
        let mut player = Command::new("player");
        policy.apply(&mut extractor);
        policy.apply(&mut player);
        let entries = |c: &Command| {
            c.get_envs()
                .map(|(k, v)| (k.to_owned(), v.map(std::ffi::OsStr::to_owned)))
                .collect::<Vec<_>>()
        };
        assert_eq!(entries(&extractor), entries(&player));
        assert!(extractor.get_envs().any(|(k, v)| k == "https_proxy"
            && v == Some(std::ffi::OsStr::new("http://proxy.example:8080"))));
        assert!(
            extractor
                .get_envs()
                .any(|(k, v)| k == "http_proxy" && v.is_none())
        );
    }
    #[test]
    fn mapping_https_and_bypass() {
        let policy = parse_manual("http=one:80;https=two:81").unwrap();
        assert_eq!(
            policy.values[1],
            ("https_proxy".into(), "http://two:81".into())
        );
        assert_eq!(
            bypass_list("<local>;*.example.org"),
            "localhost,127.0.0.1,::1,.example.org"
        );
        assert!(parse_manual("socks=host:9").is_err());
        assert!(parse_manual("https://host:9").is_err());
        assert!(parse_manual("http=").is_err());
    }
    #[test]
    fn precedence_direct_errors_and_no_credentials_in_debug() {
        let settings = || {
            Ok(SystemSettings {
                proxy: Some("system:80".into()),
                ..Default::default()
            })
        };
        let policy = NetworkPolicy::from_settings(
            &[("HtTpS_PrOxY".into(), "http://user:secret@host:8".into())],
            settings(),
        );
        assert_eq!(policy.values.len(), 1);
        assert!(!format!("{policy:?}").contains("secret"));
        let direct = NetworkPolicy::from_settings(&[], Ok(SystemSettings::default()));
        assert!(direct.values.is_empty());
        assert!(
            NetworkPolicy::from_settings(&[], Err(()))
                .diagnostic()
                .contains("failed")
        );
        let pac = NetworkPolicy::from_settings(
            &[],
            Ok(SystemSettings {
                automatic: true,
                ..Default::default()
            }),
        );
        assert!(pac.diagnostic().contains("unsupported"));
        let empty =
            NetworkPolicy::from_settings(&[("ALL_PROXY".into(), String::new())], settings());
        assert_eq!(empty.values[0].1, "");
    }
}
