//! The addresses timers can connect to: every non-loopback IPv4 address of
//! the machine, labelled LAN or VPN.

use std::net::Ipv4Addr;

/// The kind of network an address is on, as shown next to its URL.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Network {
    /// A local network. Listed first.
    Lan,
    /// A VPN or tunnel, like Tailscale or WireGuard.
    Vpn,
}

impl Network {
    /// The label shown next to the URL.
    pub fn label(self) -> &'static str {
        match self {
            Self::Lan => "LAN",
            Self::Vpn => "VPN",
        }
    }
}

/// An IPv4 address of the machine, and the kind of network it is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NetworkAddress {
    pub network: Network,
    pub ip: Ipv4Addr,
}

impl NetworkAddress {
    /// The URL LiveSplit One connects to, on `port`.
    pub fn url(&self, port: u16) -> String {
        format!("ws://{}:{port}", self.ip)
    }
}

/// Interface name prefixes of VPNs and tunnels: Tailscale, WireGuard, OpenVPN
/// and other tun and tap devices, macOS tunnels, ZeroTier and PPP.
const VPN_PREFIXES: [&str; 7] = ["tailscale", "wg", "tun", "utun", "zt", "ppp", "tap"];

/// The machine's non-loopback IPv4 addresses, LAN first. Empty if the
/// interfaces can't be read.
pub fn network_addresses() -> Vec<NetworkAddress> {
    let interfaces = if_addrs::get_if_addrs().unwrap_or_default();
    classify(
        interfaces
            .iter()
            .filter_map(|interface| match interface.addr {
                if_addrs::IfAddr::V4(ref v4) => Some((interface.name.as_str(), v4.ip)),
                if_addrs::IfAddr::V6(_) => None,
            }),
    )
}

/// Labels each interface address LAN or VPN, leaving out loopback and
/// duplicates. LAN addresses come first, then VPN, each in address order, so
/// the list doesn't reorder between refreshes.
fn classify<'a>(interfaces: impl IntoIterator<Item = (&'a str, Ipv4Addr)>) -> Vec<NetworkAddress> {
    let mut addresses: Vec<NetworkAddress> = interfaces
        .into_iter()
        .filter(|(_, ip)| !ip.is_loopback() && !ip.is_unspecified())
        .map(|(name, ip)| NetworkAddress {
            network: network(name, ip),
            ip,
        })
        .collect();
    addresses.sort_by_key(|address| (address.network, address.ip));
    addresses.dedup_by_key(|address| address.ip);
    addresses
}

/// Whether an address is on a VPN: its interface is named like a tunnel, or
/// it is in 100.64.0.0/10, the shared address space Tailscale uses.
fn network(interface: &str, ip: Ipv4Addr) -> Network {
    let name = interface.to_lowercase();
    let tunnel = VPN_PREFIXES.iter().any(|prefix| name.starts_with(prefix)) || name.contains("vpn");
    let [first, second, ..] = ip.octets();
    let shared_space = first == 100 && (64..128).contains(&second);
    if tunnel || shared_space {
        Network::Vpn
    } else {
        Network::Lan
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ip(text: &str) -> Ipv4Addr {
        text.parse().unwrap()
    }

    #[test]
    fn urls_use_the_port() {
        let address = NetworkAddress {
            network: Network::Lan,
            ip: ip("192.168.1.20"),
        };
        assert_eq!(address.url(16834), "ws://192.168.1.20:16834");
        assert_eq!(address.url(9000), "ws://192.168.1.20:9000");
    }

    #[test]
    fn loopback_is_left_out() {
        let addresses = classify([("lo", ip("127.0.0.1")), ("eth0", ip("192.168.1.20"))]);
        assert_eq!(
            addresses,
            [NetworkAddress {
                network: Network::Lan,
                ip: ip("192.168.1.20"),
            }]
        );
    }

    #[test]
    fn tunnels_are_vpns() {
        for name in [
            "tailscale0",
            "wg0",
            "tun0",
            "utun3",
            "ztabcdef",
            "ppp0",
            "tap0",
            "ProtonVPN",
        ] {
            assert_eq!(network(name, ip("10.8.0.2")), Network::Vpn, "{name}");
        }
        for name in ["eth0", "enp3s0", "wlan0", "en0", "Ethernet", "Wi-Fi"] {
            assert_eq!(network(name, ip("10.8.0.2")), Network::Lan, "{name}");
        }
    }

    #[test]
    fn tailscale_addresses_are_vpns_whatever_the_interface() {
        assert_eq!(network("Ethernet 2", ip("100.101.7.3")), Network::Vpn);
        assert_eq!(network("eth0", ip("100.64.0.1")), Network::Vpn);
        assert_eq!(network("eth0", ip("100.127.255.254")), Network::Vpn);
        assert_eq!(network("eth0", ip("100.128.0.1")), Network::Lan);
        assert_eq!(network("eth0", ip("100.63.255.255")), Network::Lan);
    }

    #[test]
    fn lan_comes_first_in_address_order() {
        let addresses = classify([
            ("tailscale0", ip("100.101.7.3")),
            ("wlan0", ip("192.168.1.30")),
            ("eth0", ip("192.168.1.20")),
            ("wg0", ip("10.0.0.2")),
        ]);
        let listed: Vec<_> = addresses
            .iter()
            .map(|address| (address.network.label(), address.url(16834)))
            .collect();
        assert_eq!(
            listed,
            [
                ("LAN", "ws://192.168.1.20:16834".to_owned()),
                ("LAN", "ws://192.168.1.30:16834".to_owned()),
                ("VPN", "ws://10.0.0.2:16834".to_owned()),
                ("VPN", "ws://100.101.7.3:16834".to_owned()),
            ]
        );
    }

    #[test]
    fn an_address_is_listed_once() {
        let addresses = classify([("eth0", ip("192.168.1.20")), ("eth0", ip("192.168.1.20"))]);
        assert_eq!(addresses.len(), 1);
    }
}
