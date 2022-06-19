use std::net::IpAddr;

use get_if_addrs::get_if_addrs;

pub fn get_first_net<F>(f: F) -> Option<IpAddr>
where
    F: FnMut(&IpAddr) -> bool,
{
    get_if_addrs()
        .unwrap_or_default()
        .into_iter()
        .map(|i| i.ip())
        .find(f)
}

/// While [`std::net::IpAddr::is_global`] is still unstable after 7 years, here
/// is my approach to implement the predicate for [`std::net::Ipv4Addr`].
pub const fn is_global_4(addr: &IpAddr) -> bool {
    if let IpAddr::V4(addr) = addr {
        !(addr.is_unspecified()
            || addr.is_loopback()
            || addr.is_link_local()
            || addr.is_multicast()
            || addr.is_broadcast())
    } else {
        false
    }
}
