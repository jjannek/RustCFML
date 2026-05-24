//! Peer discovery strategies for the clustered session store.
//!
//! Three methods are supported:
//!
//! - **Static** — fixed `host:port` list. Useful for tests and small fixed
//!   clusters. No periodic refresh.
//! - **DNS** — periodically resolves a hostname's A/AAAA records and yields
//!   the resulting addresses. This is the strategy to use on Fly.io
//!   (`<FLY_APP_NAME>.internal`), Kubernetes headless services, ECS Service
//!   Discovery, etc.
//! - **Multicast** — UDP multicast announce/listen on an admin-scoped group.
//!   Works on LANs, bare metal, VMware, and on Kubernetes CNIs that carry
//!   multicast (Calico VXLAN, Weave, Flannel VXLAN). Does **not** work on
//!   AWS VPC CNI, Fly.io 6PN, or most cloud-default networks.
//!
//! `Discovery::discover()` returns the currently-known peer addresses;
//! `ClusterStore` calls it on a timer and feeds new entries into
//! `memberlist.join_many()`.

use std::{
    collections::HashSet,
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::Duration,
};

const DEFAULT_DNS_INTERVAL: Duration = Duration::from_secs(10);
const DEFAULT_MCAST_INTERVAL: Duration = Duration::from_secs(5);
const MCAST_ANNOUNCE_TAG: &[u8] = b"RCFM1";

#[derive(Clone)]
pub enum Discovery {
    Static(StaticSeeds),
    Dns(DnsDiscovery),
    Multicast(MulticastDiscovery),
}

impl Discovery {
    pub async fn discover(&self) -> Vec<SocketAddr> {
        match self {
            Discovery::Static(s) => s.snapshot(),
            Discovery::Dns(d) => d.resolve().await,
            Discovery::Multicast(m) => m.snapshot(),
        }
    }

    /// Periodic re-discovery interval. `None` means "one-shot, never refresh".
    pub fn interval(&self) -> Option<Duration> {
        match self {
            Discovery::Static(_) => None,
            Discovery::Dns(d) => Some(d.interval),
            Discovery::Multicast(m) => Some(m.interval),
        }
    }

    /// Short human-readable label for log lines.
    pub fn label(&self) -> String {
        match self {
            Discovery::Static(s) => format!("static[{}]", s.seeds.len()),
            Discovery::Dns(d) => format!("dns({}:{})", d.name, d.port),
            Discovery::Multicast(m) => format!("multicast({}:{})", m.group, m.port),
        }
    }
}

// ─────────────────────────────────────────────
// Static
// ─────────────────────────────────────────────

#[derive(Clone)]
pub struct StaticSeeds {
    seeds: Vec<SocketAddr>,
}

impl StaticSeeds {
    pub fn new(raw: &[String]) -> Self {
        let seeds = raw
            .iter()
            .filter_map(|s| match s.parse::<SocketAddr>() {
                Ok(sa) => Some(sa),
                Err(e) => {
                    eprintln!("[session/cluster] discovery: bad seed '{}': {}", s, e);
                    None
                }
            })
            .collect();
        Self { seeds }
    }

    fn snapshot(&self) -> Vec<SocketAddr> {
        self.seeds.clone()
    }
}

// ─────────────────────────────────────────────
// DNS
// ─────────────────────────────────────────────

#[derive(Clone)]
pub struct DnsDiscovery {
    pub name: String,
    pub port: u16,
    pub interval: Duration,
}

impl DnsDiscovery {
    pub fn new(name: String, port: u16, interval_secs: u64) -> Self {
        let interval = if interval_secs == 0 {
            DEFAULT_DNS_INTERVAL
        } else {
            Duration::from_secs(interval_secs)
        };
        Self { name, port, interval }
    }

    async fn resolve(&self) -> Vec<SocketAddr> {
        let target = format!("{}:{}", self.name, self.port);
        let result = tokio::net::lookup_host(target.clone()).await;
        match result {
            Ok(iter) => iter.collect(),
            Err(e) => {
                eprintln!(
                    "[session/cluster] discovery: DNS lookup of '{}' failed: {}",
                    target, e
                );
                Vec::new()
            }
        }
    }
}

// ─────────────────────────────────────────────
// Multicast
// ─────────────────────────────────────────────
//
// We send our own listen address as a small UDP datagram to a multicast
// group on a schedule, and listen for the same kind of datagram from
// peers. Discovered addresses accumulate in a Mutex<HashSet> which
// `snapshot()` returns. A peer is only forgotten when this process
// restarts — memberlist will mark genuinely-dead peers as failed and we
// don't need to expire them here.

#[derive(Clone)]
pub struct MulticastDiscovery {
    pub group: String,
    pub port: u16,
    pub interval: Duration,
    seen: Arc<Mutex<HashSet<SocketAddr>>>,
}

impl MulticastDiscovery {
    /// Start the announcer + listener tasks. `self_addr` is what we
    /// advertise to peers — must be reachable from them (so don't
    /// advertise `0.0.0.0:7946`; pass the real bind / advertise addr).
    pub fn start(
        group: String,
        port: u16,
        interval_secs: u64,
        self_addr: SocketAddr,
    ) -> Result<Self, String> {
        use socket2::{Domain, Protocol, Socket, Type};
        use std::net::{Ipv4Addr, SocketAddrV4};

        let interval = if interval_secs == 0 {
            DEFAULT_MCAST_INTERVAL
        } else {
            Duration::from_secs(interval_secs)
        };

        let group_ip: Ipv4Addr = group
            .parse()
            .map_err(|e| format!("invalid multicast group '{}': {}", group, e))?;
        if !group_ip.is_multicast() {
            return Err(format!("{} is not a multicast address", group_ip));
        }

        // Build the socket via socket2 so we can SO_REUSEADDR + join group
        // before handing the fd to tokio.
        let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))
            .map_err(|e| format!("multicast socket() failed: {}", e))?;
        socket
            .set_reuse_address(true)
            .map_err(|e| format!("SO_REUSEADDR failed: {}", e))?;
        #[cfg(unix)]
        socket
            .set_reuse_port(true)
            .map_err(|e| format!("SO_REUSEPORT failed: {}", e))?;
        let bind_addr = SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, port);
        socket
            .bind(&bind_addr.into())
            .map_err(|e| format!("multicast bind {} failed: {}", bind_addr, e))?;
        socket
            .join_multicast_v4(&group_ip, &Ipv4Addr::UNSPECIFIED)
            .map_err(|e| format!("join_multicast_v4 {} failed: {}", group_ip, e))?;
        socket
            .set_multicast_loop_v4(true)
            .map_err(|e| format!("set_multicast_loop_v4 failed: {}", e))?;
        socket
            .set_nonblocking(true)
            .map_err(|e| format!("set_nonblocking failed: {}", e))?;

        let std_sock: std::net::UdpSocket = socket.into();
        let tokio_sock = tokio::net::UdpSocket::from_std(std_sock)
            .map_err(|e| format!("tokio UdpSocket::from_std failed: {}", e))?;
        let tokio_sock = Arc::new(tokio_sock);

        let seen = Arc::new(Mutex::new(HashSet::new()));

        // Announcer task: send `MCAST_ANNOUNCE_TAG || self_addr.to_string()`
        // to the group every `interval`.
        let send_sock = tokio_sock.clone();
        let group_target: SocketAddr = SocketAddr::new(group_ip.into(), port);
        let self_label = self_addr.to_string();
        tokio::spawn(async move {
            let mut payload = Vec::with_capacity(MCAST_ANNOUNCE_TAG.len() + self_label.len());
            payload.extend_from_slice(MCAST_ANNOUNCE_TAG);
            payload.extend_from_slice(self_label.as_bytes());
            let mut tick = tokio::time::interval(interval);
            tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            loop {
                tick.tick().await;
                if let Err(e) = send_sock.send_to(&payload, group_target).await {
                    eprintln!(
                        "[session/cluster] discovery: multicast send to {} failed: {}",
                        group_target, e
                    );
                }
            }
        });

        // Listener task: parse `MCAST_ANNOUNCE_TAG || addr_str` and stash.
        let recv_sock = tokio_sock.clone();
        let seen_w = seen.clone();
        let self_addr_filter = self_addr;
        tokio::spawn(async move {
            let mut buf = [0u8; 256];
            loop {
                let (n, _from) = match recv_sock.recv_from(&mut buf).await {
                    Ok(v) => v,
                    Err(e) => {
                        eprintln!(
                            "[session/cluster] discovery: multicast recv failed: {}",
                            e
                        );
                        tokio::time::sleep(Duration::from_secs(1)).await;
                        continue;
                    }
                };
                if n < MCAST_ANNOUNCE_TAG.len() {
                    continue;
                }
                if &buf[..MCAST_ANNOUNCE_TAG.len()] != MCAST_ANNOUNCE_TAG {
                    continue;
                }
                let addr_bytes = &buf[MCAST_ANNOUNCE_TAG.len()..n];
                let addr_str = match std::str::from_utf8(addr_bytes) {
                    Ok(s) => s,
                    Err(_) => continue,
                };
                let parsed: SocketAddr = match addr_str.parse() {
                    Ok(a) => a,
                    Err(_) => continue,
                };
                if parsed == self_addr_filter {
                    continue;
                }
                if let Ok(mut s) = seen_w.lock() {
                    s.insert(parsed);
                }
            }
        });

        Ok(Self {
            group,
            port,
            interval,
            seen,
        })
    }

    fn snapshot(&self) -> Vec<SocketAddr> {
        match self.seen.lock() {
            Ok(s) => s.iter().copied().collect(),
            Err(_) => Vec::new(),
        }
    }
}
