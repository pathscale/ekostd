use crate::clone::Clone;
use crate::cmp::{Eq, PartialEq};
use crate::default::Default;
use crate::fmt;
use crate::hash::{Hash, Hasher};
use crate::marker::Copy;

pub struct Ipv4Addr {
    octets: [u8; 4],
}

impl Copy for Ipv4Addr {}
impl Clone for Ipv4Addr {
    fn clone(&self) -> Self {
        *self
    }
}
impl PartialEq for Ipv4Addr {
    fn eq(&self, other: &Self) -> bool {
        self.octets == other.octets
    }
}
impl Eq for Ipv4Addr {}
impl Default for Ipv4Addr {
    fn default() -> Self {
        Self::UNSPECIFIED
    }
}
impl Hash for Ipv4Addr {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.octets.hash(state);
    }
}
impl fmt::Debug for Ipv4Addr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl fmt::Display for Ipv4Addr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}.{}.{}.{}",
            self.octets[0], self.octets[1], self.octets[2], self.octets[3]
        )
    }
}

impl Ipv4Addr {
    pub const UNSPECIFIED: Self = Self {
        octets: [0, 0, 0, 0],
    };
    pub const LOCALHOST: Self = Self {
        octets: [127, 0, 0, 1],
    };
    pub const BROADCAST: Self = Self {
        octets: [255, 255, 255, 255],
    };

    pub const fn new(a: u8, b: u8, c: u8, d: u8) -> Self {
        Self {
            octets: [a, b, c, d],
        }
    }

    pub const fn from_octets(octets: [u8; 4]) -> Self {
        Self { octets }
    }

    pub const fn octets(&self) -> [u8; 4] {
        self.octets
    }

    pub const fn to_bits(self) -> u32 {
        u32::from_be_bytes(self.octets)
    }

    pub const fn from_bits(bits: u32) -> Self {
        Self {
            octets: bits.to_be_bytes(),
        }
    }
}

impl crate::convert::From<[u8; 4]> for Ipv4Addr {
    fn from(octets: [u8; 4]) -> Self {
        Self { octets }
    }
}

pub struct Ipv6Addr {
    octets: [u8; 16],
}

impl Copy for Ipv6Addr {}
impl Clone for Ipv6Addr {
    fn clone(&self) -> Self {
        *self
    }
}
impl PartialEq for Ipv6Addr {
    fn eq(&self, other: &Self) -> bool {
        self.octets == other.octets
    }
}
impl Eq for Ipv6Addr {}
impl Default for Ipv6Addr {
    fn default() -> Self {
        Self::UNSPECIFIED
    }
}
impl Hash for Ipv6Addr {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.octets.hash(state);
    }
}
impl fmt::Debug for Ipv6Addr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl fmt::Display for Ipv6Addr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let segs = self.segments();
        write!(
            f,
            "{:x}:{:x}:{:x}:{:x}:{:x}:{:x}:{:x}:{:x}",
            segs[0], segs[1], segs[2], segs[3], segs[4], segs[5], segs[6], segs[7]
        )
    }
}

impl Ipv6Addr {
    pub const UNSPECIFIED: Self = Self { octets: [0; 16] };
    pub const LOCALHOST: Self = Self {
        octets: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
    };

    pub const fn new(
        a: u16,
        b: u16,
        c: u16,
        d: u16,
        e: u16,
        g: u16,
        h: u16,
        i: u16,
    ) -> Self {
        let ab = a.to_be_bytes();
        let bb = b.to_be_bytes();
        let cb = c.to_be_bytes();
        let db = d.to_be_bytes();
        let eb = e.to_be_bytes();
        let gb = g.to_be_bytes();
        let hb = h.to_be_bytes();
        let ib = i.to_be_bytes();
        Self {
            octets: [
                ab[0], ab[1], bb[0], bb[1], cb[0], cb[1], db[0], db[1], eb[0], eb[1], gb[0],
                gb[1], hb[0], hb[1], ib[0], ib[1],
            ],
        }
    }

    pub const fn from_octets(octets: [u8; 16]) -> Self {
        Self { octets }
    }

    pub const fn octets(&self) -> [u8; 16] {
        self.octets
    }

    pub const fn segments(&self) -> [u16; 8] {
        [
            u16::from_be_bytes([self.octets[0], self.octets[1]]),
            u16::from_be_bytes([self.octets[2], self.octets[3]]),
            u16::from_be_bytes([self.octets[4], self.octets[5]]),
            u16::from_be_bytes([self.octets[6], self.octets[7]]),
            u16::from_be_bytes([self.octets[8], self.octets[9]]),
            u16::from_be_bytes([self.octets[10], self.octets[11]]),
            u16::from_be_bytes([self.octets[12], self.octets[13]]),
            u16::from_be_bytes([self.octets[14], self.octets[15]]),
        ]
    }
}

impl crate::convert::From<[u8; 16]> for Ipv6Addr {
    fn from(octets: [u8; 16]) -> Self {
        Self { octets }
    }
}

pub enum IpAddr {
    V4(Ipv4Addr),
    V6(Ipv6Addr),
}

impl Copy for IpAddr {}
impl Clone for IpAddr {
    fn clone(&self) -> Self {
        *self
    }
}
impl PartialEq for IpAddr {
    fn eq(&self, other: &Self) -> bool {
        match (*self, *other) {
            (IpAddr::V4(a), IpAddr::V4(b)) => a == b,
            (IpAddr::V6(a), IpAddr::V6(b)) => a == b,
            _ => false,
        }
    }
}
impl Eq for IpAddr {}
impl Hash for IpAddr {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            IpAddr::V4(a) => {
                0u8.hash(state);
                a.hash(state);
            }
            IpAddr::V6(a) => {
                1u8.hash(state);
                a.hash(state);
            }
        }
    }
}
impl fmt::Debug for IpAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl fmt::Display for IpAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IpAddr::V4(a) => fmt::Display::fmt(a, f),
            IpAddr::V6(a) => fmt::Display::fmt(a, f),
        }
    }
}

pub struct SocketAddrV4 {
    ip: Ipv4Addr,
    port: u16,
}

impl Copy for SocketAddrV4 {}
impl Clone for SocketAddrV4 {
    fn clone(&self) -> Self {
        *self
    }
}
impl PartialEq for SocketAddrV4 {
    fn eq(&self, other: &Self) -> bool {
        self.ip == other.ip && self.port == other.port
    }
}
impl Eq for SocketAddrV4 {}
impl Hash for SocketAddrV4 {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.ip.hash(state);
        self.port.hash(state);
    }
}
impl fmt::Debug for SocketAddrV4 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl fmt::Display for SocketAddrV4 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.ip, self.port)
    }
}

impl SocketAddrV4 {
    pub const fn new(ip: Ipv4Addr, port: u16) -> Self {
        Self { ip, port }
    }
    pub const fn ip(&self) -> &Ipv4Addr {
        &self.ip
    }
    pub fn set_ip(&mut self, new_ip: Ipv4Addr) {
        self.ip = new_ip;
    }
    pub const fn port(&self) -> u16 {
        self.port
    }
    pub fn set_port(&mut self, new_port: u16) {
        self.port = new_port;
    }
}

pub struct SocketAddrV6 {
    ip: Ipv6Addr,
    port: u16,
    flowinfo: u32,
    scope_id: u32,
}

impl Copy for SocketAddrV6 {}
impl Clone for SocketAddrV6 {
    fn clone(&self) -> Self {
        *self
    }
}
impl PartialEq for SocketAddrV6 {
    fn eq(&self, other: &Self) -> bool {
        self.ip == other.ip
            && self.port == other.port
            && self.flowinfo == other.flowinfo
            && self.scope_id == other.scope_id
    }
}
impl Eq for SocketAddrV6 {}
impl Hash for SocketAddrV6 {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.ip.hash(state);
        self.port.hash(state);
        self.flowinfo.hash(state);
        self.scope_id.hash(state);
    }
}
impl fmt::Debug for SocketAddrV6 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl fmt::Display for SocketAddrV6 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}]:{}", self.ip, self.port)
    }
}

impl SocketAddrV6 {
    pub const fn new(ip: Ipv6Addr, port: u16, flowinfo: u32, scope_id: u32) -> Self {
        Self {
            ip,
            port,
            flowinfo,
            scope_id,
        }
    }
    pub const fn ip(&self) -> &Ipv6Addr {
        &self.ip
    }
    pub fn set_ip(&mut self, new_ip: Ipv6Addr) {
        self.ip = new_ip;
    }
    pub const fn port(&self) -> u16 {
        self.port
    }
    pub fn set_port(&mut self, new_port: u16) {
        self.port = new_port;
    }
    pub const fn flowinfo(&self) -> u32 {
        self.flowinfo
    }
    pub fn set_flowinfo(&mut self, new_flowinfo: u32) {
        self.flowinfo = new_flowinfo;
    }
    pub const fn scope_id(&self) -> u32 {
        self.scope_id
    }
    pub fn set_scope_id(&mut self, new_scope_id: u32) {
        self.scope_id = new_scope_id;
    }
}

pub enum SocketAddr {
    V4(SocketAddrV4),
    V6(SocketAddrV6),
}

impl Copy for SocketAddr {}
impl Clone for SocketAddr {
    fn clone(&self) -> Self {
        *self
    }
}
impl PartialEq for SocketAddr {
    fn eq(&self, other: &Self) -> bool {
        match (*self, *other) {
            (SocketAddr::V4(a), SocketAddr::V4(b)) => a == b,
            (SocketAddr::V6(a), SocketAddr::V6(b)) => a == b,
            _ => false,
        }
    }
}
impl Eq for SocketAddr {}
impl Hash for SocketAddr {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            SocketAddr::V4(a) => {
                0u8.hash(state);
                a.hash(state);
            }
            SocketAddr::V6(a) => {
                1u8.hash(state);
                a.hash(state);
            }
        }
    }
}
impl fmt::Debug for SocketAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl fmt::Display for SocketAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SocketAddr::V4(a) => fmt::Display::fmt(a, f),
            SocketAddr::V6(a) => fmt::Display::fmt(a, f),
        }
    }
}

impl SocketAddr {
    pub const fn new(ip: IpAddr, port: u16) -> Self {
        match ip {
            IpAddr::V4(a) => SocketAddr::V4(SocketAddrV4::new(a, port)),
            IpAddr::V6(a) => SocketAddr::V6(SocketAddrV6::new(a, port, 0, 0)),
        }
    }

    pub const fn ip(&self) -> IpAddr {
        match *self {
            SocketAddr::V4(ref a) => IpAddr::V4(*a.ip()),
            SocketAddr::V6(ref a) => IpAddr::V6(*a.ip()),
        }
    }

    pub const fn port(&self) -> u16 {
        match *self {
            SocketAddr::V4(ref a) => a.port(),
            SocketAddr::V6(ref a) => a.port(),
        }
    }

    pub const fn is_ipv4(&self) -> bool {
        matches!(*self, SocketAddr::V4(_))
    }

    pub const fn is_ipv6(&self) -> bool {
        matches!(*self, SocketAddr::V6(_))
    }
}
