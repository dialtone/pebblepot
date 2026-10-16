//! Settings, read from environment variables. Every setting has a default, so
//! `cargo run` works with no setup.

use std::fmt;
use std::net::IpAddr;
use std::path::PathBuf;

use jiff::tz::TimeZone;

pub const LISTEN_ADDR_VAR: &str = "PEBBLEPOT_LISTEN_ADDR";
pub const PORT_VAR: &str = "PEBBLEPOT_PORT";
pub const DATA_DIR_VAR: &str = "PEBBLEPOT_DATA_DIR";
pub const TIME_ZONE_VAR: &str = "PEBBLEPOT_TIME_ZONE";

#[derive(Clone, Debug)]
pub struct Config {
    pub listen_addr: IpAddr,
    pub port: u16,
    pub data_dir: PathBuf,
    /// The household's time zone; it decides when "today" starts and ends.
    pub time_zone: TimeZone,
}

#[derive(Debug)]
pub struct ConfigError(String);

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ConfigError {}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        Self::from_lookup(|name| std::env::var(name).ok())
    }

    pub fn from_lookup(get: impl Fn(&str) -> Option<String>) -> Result<Self, ConfigError> {
        let listen_addr = match get(LISTEN_ADDR_VAR) {
            Some(v) => v
                .parse()
                .map_err(|_| ConfigError(format!("{LISTEN_ADDR_VAR}: not an IP address: {v:?}")))?,
            None => IpAddr::from([127, 0, 0, 1]),
        };
        let port = match get(PORT_VAR) {
            Some(v) => v
                .parse()
                .map_err(|_| ConfigError(format!("{PORT_VAR}: not a port number: {v:?}")))?,
            None => 8070,
        };
        let data_dir = get(DATA_DIR_VAR).map_or_else(|| PathBuf::from("./data"), PathBuf::from);
        let time_zone = match get(TIME_ZONE_VAR) {
            Some(v) => TimeZone::get(&v).map_err(|e| {
                ConfigError(format!("{TIME_ZONE_VAR}: unknown time zone {v:?}: {e}"))
            })?,
            None => TimeZone::system(),
        };
        Ok(Config {
            listen_addr,
            port,
            data_dir,
            time_zone,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn config(vars: &[(&str, &str)]) -> Result<Config, ConfigError> {
        let vars: HashMap<String, String> = vars
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        Config::from_lookup(|name| vars.get(name).cloned())
    }

    #[test]
    fn defaults_need_no_setup() {
        let c = config(&[]).unwrap();
        assert_eq!(c.listen_addr, IpAddr::from([127, 0, 0, 1]));
        assert_eq!(c.port, 8070);
        assert_eq!(c.data_dir, PathBuf::from("./data"));
        assert_eq!(c.time_zone, TimeZone::system());
    }

    #[test]
    fn every_setting_can_be_overridden() {
        let c = config(&[
            (LISTEN_ADDR_VAR, "0.0.0.0"),
            (PORT_VAR, "9000"),
            (DATA_DIR_VAR, "/var/lib/pebblepot"),
            (TIME_ZONE_VAR, "Europe/Rome"),
        ])
        .unwrap();
        assert_eq!(c.listen_addr, IpAddr::from([0, 0, 0, 0]));
        assert_eq!(c.port, 9000);
        assert_eq!(c.data_dir, PathBuf::from("/var/lib/pebblepot"));
        assert_eq!(c.time_zone.iana_name(), Some("Europe/Rome"));
    }

    #[test]
    fn bad_values_are_reported() {
        assert!(config(&[(LISTEN_ADDR_VAR, "kitchen")]).is_err());
        assert!(config(&[(PORT_VAR, "99999")]).is_err());
        assert!(config(&[(TIME_ZONE_VAR, "Mars/Olympus")]).is_err());
    }
}
