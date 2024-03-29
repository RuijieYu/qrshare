//! This module contains configuration and command-line interface
//! functionalities.

use std::{
    fmt::{self, Display, Formatter},
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
};

use either::Either;

use crate::{
    default,
    net::{get_first_net, is_global_4},
    unwrap_getter,
};

/// The configuration structure.  Should be able to be extracted from one or
/// more configuration files.
#[derive(Debug, Clone, clap::Args, serde::Deserialize, merge::Merge)]
#[cfg_attr(test, derive(PartialEq, Eq))]
pub struct Config {
    /// Image options.  Use PNG format or SVG format to produce the QR code, or
    /// skip producing the QR code at all.
    #[clap(short = 'I', long, value_enum)]
    pub image: Option<ImageOptions>,

    /// Quiet operation.  Do not warn about missing files.
    #[clap(short, long, value_parser)]
    pub quiet: Option<bool>,

    /// Strict mode.  When enabled, the server exits on any failure in path
    /// resolution and IO.
    #[clap(short, long, value_parser)]
    pub strict: Option<bool>,

    /// Bind options, containing the bound host(s) and port.
    #[clap(flatten)]
    #[serde(default)]
    pub bind: BindOptions,
}
default!(
    !Config = Self {
        image: None,
        quiet: None,
        strict: None,
        bind: BindOptions::default()
    }
);
unwrap_getter!(Config::image: ImageOptions);

/// Allowed image formats.
#[derive(Debug, Clone, Copy, serde::Deserialize, clap::ValueEnum)]
#[cfg_attr(test, derive(PartialEq, Eq))]
pub enum ImageOptions {
    Png,
    Svg,
    None,
}
default!(ImageOptions = Self::Png);

impl Display for ImageOptions {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        use clap::ValueEnum;
        f.write_str(self.to_possible_value().unwrap().get_name())
    }
}

/// Options for interface bindings.
#[derive(Debug, Clone, serde::Deserialize, clap::Args, merge::Merge)]
#[cfg_attr(test, derive(PartialEq, Eq))]
pub struct BindOptions {
    /// Sets custom bound host addresses.  When empty, use all available IPv4
    /// and IPv6 addresses.
    #[clap(short = 'H', long, value_parser)]
    #[serde(default = "BindOptions::default_hosts")]
    #[merge(strategy = merge::vec::overwrite_empty)]
    pub hosts: Vec<IpAddr>,

    /// Sets a custom port.  Default to 0, where an arbitrary available port is
    /// used.
    #[clap(short, long, value_parser)]
    pub port: Option<u16>,
}

default!(!BindOptions = Self { hosts: Self::default_hosts(), port: None });
unwrap_getter!(BindOptions::port: u16 = 0);

impl BindOptions {
    pub const UNSPECIFIED_HOSTS: [IpAddr; 2] =
        [IpAddr::V4(Ipv4Addr::UNSPECIFIED), IpAddr::V6(Ipv6Addr::UNSPECIFIED)];

    #[inline]
    pub(crate) fn default_hosts() -> Vec<IpAddr> {
        BindOptions::UNSPECIFIED_HOSTS.into()
    }

    pub fn hosts_iter(&self) -> impl Iterator<Item = IpAddr> {
        if self.hosts.is_empty() {
            Either::Right(Self::UNSPECIFIED_HOSTS.into_iter())
        } else {
            Either::Left(self.hosts.clone().into_iter())
        }
    }

    pub fn primary_host(&self) -> IpAddr {
        if self.hosts.is_empty() {
            get_first_net(is_global_4).unwrap_or(Self::UNSPECIFIED_HOSTS[0])
        } else {
            self.hosts[0]
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{fs::read_to_string, net::Ipv6Addr};

    use super::{BindOptions, Config};

    #[test]
    fn test_config() {
        let config: Config = toml::from_str("").unwrap();
        assert_eq!(config, Config::default());

        let config: Config = toml::toml! {
            [bind]
                hosts = ["1.2.3.4", "5.6.7.8", "::"]
        }
        .try_into()
        .unwrap();
        assert_eq!(
            config,
            Config {
                bind: BindOptions {
                    hosts: [
                        [1, 2, 3, 4].into(),
                        [5, 6, 7, 8].into(),
                        Ipv6Addr::UNSPECIFIED.into()
                    ]
                    .into_iter()
                    .collect(),
                    port: None
                },
                ..Config::default()
            }
        );
    }

    #[test]
    fn test_examples() {
        let config = read_to_string("../assets/empty.toml").unwrap();
        let config: Config = toml::from_str(&config).unwrap();
        _ = config;
    }
}
