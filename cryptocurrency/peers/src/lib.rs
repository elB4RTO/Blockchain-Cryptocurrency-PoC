
#[derive(PartialEq, Eq, Hash, Clone)]
pub struct Peer (std::net::SocketAddr);

impl Peer {
	pub const fn new(peer: std::net::SocketAddr) -> Self {
		Self(peer)
	}
	
	const fn size_of_self(&self) -> usize {
		match self.0.is_ipv4() {
			true => Self::size_of_tag() + Self::size_of_ipv4() + Self::size_of_port(),
			false => Self::size_of_tag() + Self::size_of_ipv6() + Self::size_of_port(),
		}
	}
	
	const fn size_of_tag() -> usize {
		std::mem::size_of::<u16>()
	}
	
	const fn offset_of_tag() -> usize {
		0
	}
	
	const fn is_ipv4_tag(tag: u16) -> bool {
		tag as usize == Self::size_of_ipv4()
	}
	
	const fn is_ipv6_tag(tag: u16) -> bool {
		tag as usize == Self::size_of_ipv6()
	}
	
	const fn size_of_ipv4() -> usize {
		std::mem::size_of::<u32>()
	}
	
	const fn size_of_ipv6() -> usize {
		std::mem::size_of::<u128>()
	}
	
	const fn offset_of_ip() -> usize {
		Self::offset_of_tag() + Self::size_of_tag()
	}
	
	const fn size_of_port() -> usize {
		std::mem::size_of::<u16>()
	}
	
	const fn offset_of_port(is_ipv4: bool) -> usize {
		match is_ipv4 {
			true => Self::offset_of_ip() + Self::size_of_ipv4(),
			false => Self::offset_of_ip() + Self::size_of_ipv6(),
		}
	}
}

impl From<std::net::SocketAddr> for Peer {
	fn from(sock: std::net::SocketAddr) -> Self {
		Self(sock)
	}
}

impl From<(std::net::IpAddr, u16)> for Peer {
	fn from((ip, port): (std::net::IpAddr, u16)) -> Self {
		Self(std::net::SocketAddr::new(ip, port))
	}
}

impl std::fmt::Display for Peer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl bytes_utils::BytesConversion for Peer {
	type Error = String;
	
    fn as_boxed_bytes(&self) -> Result<Box<[u8]>, Self::Error> {
        let mut bytes = bytes_utils::boxed_bytes(self.size_of_self());
        let ref mut bytes_index : usize = 0;
        match self.0.ip() {
			std::net::IpAddr::V4(ip) => {
				bytes_utils::append_bytes_raw(&(Self::size_of_ipv4() as u16).to_be_bytes(), &mut bytes, bytes_index)?;
				bytes_utils::append_bytes_raw(&ip.to_bits().to_be_bytes(), &mut bytes, bytes_index)?;
			},
			std::net::IpAddr::V6(ip) => {
				bytes_utils::append_bytes_raw(&(Self::size_of_ipv6() as u16).to_be_bytes(), &mut bytes, bytes_index)?;
				bytes_utils::append_bytes_raw(&ip.to_bits().to_be_bytes(), &mut bytes, bytes_index)?;
			},
		}
        bytes_utils::append_bytes_raw(&self.0.port().to_be_bytes(), &mut bytes, bytes_index)?;
        Ok(bytes)
    }
    
    fn from_raw_bytes(bytes: &[u8]) -> Result<Self, Self::Error> {
		if bytes.len() < Self::size_of_tag() {
			return Err(format!("Peer::from_raw_bytes(): not enough bytes for 'tag': expected {}, got {}", Self::size_of_tag(), bytes.len()));
		}
		let (mut beg, mut end);
		beg = Self::offset_of_tag();
		end = beg + Self::size_of_tag();
		let tag = u16::from_be_bytes(bytes[beg..end].try_into()
			.map_err(|_| String::from("Peer::from_raw_bytes(): failed to convert 'tag'"))?);
		let ip_addr: std::net::IpAddr;
		beg = Self::offset_of_ip();
		if Self::is_ipv4_tag(tag) {
			end = beg + Self::size_of_ipv4();
			if bytes.len() < end {
				return Err(format!("Peer::from_raw_bytes(): not enough bytes for 'IPv4': expected {}, got {}", end, bytes.len()));
			}
			let ip_bytes: [u8; 4] = bytes[beg..end].try_into()
				.map_err(|_| String::from("Peer::from_raw_bytes(): failed to convert 'IPv4' bytes"))?;
			ip_addr = std::net::IpAddr::from(ip_bytes);
		} else if Self::is_ipv6_tag(tag) {
			end = beg + Self::size_of_ipv6();
			if bytes.len() < end {
				return Err(format!("Peer::from_raw_bytes(): not enough bytes for 'IPv6': expected {}, got {}", end, bytes.len()));
			}
			let ip_bytes: [u8; 16] = bytes[beg..end].try_into()
				.map_err(|_| String::from("Peer::from_raw_bytes(): failed to convert 'IPv4' bytes"))?;
			ip_addr = std::net::IpAddr::from(ip_bytes);
		} else {
			return Err(format!("Peer::from_raw_bytes(): unmatched 'tag': {}", tag));
		}
		beg = Self::offset_of_port(ip_addr.is_ipv4());
		end = beg + Self::size_of_port();
		if bytes.len() < end {
			return Err(format!("Peer::from_raw_bytes(): not enough bytes for 'port': expected {}, got {}", end, bytes.len()));
		}
		let port = u16::from_be_bytes(bytes[beg..end].try_into()
			.map_err(|_| String::from("Peer::from_raw_bytes(): failed to convert 'port'"))?);
		Ok(Self::from((ip_addr, port)))
	}
}


pub struct Peers (std::collections::HashSet<Peer>);

impl Peers {
	pub fn with_capacity(capacity: usize) -> Self {
		Self(std::collections::HashSet::with_capacity(capacity))
	}
}

impl std::ops::Deref for Peers {
	type Target = std::collections::HashSet<Peer>;
	
	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl std::ops::DerefMut for Peers {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.0
	}
}

impl bytes_utils::BytesConversion for Peers {
	type Error = String;
	
    fn as_boxed_bytes(&self) -> Result<Box<[u8]>, Self::Error> {
		let n_bytes = std::mem::size_of::<usize>() + self.iter().fold(0, |tot, peer| tot + peer.size_of_self());
        let mut bytes = bytes_utils::boxed_bytes(n_bytes);
        let ref mut bytes_index : usize = 0;
        bytes_utils::append_bytes_raw(&self.len().to_be_bytes(), &mut bytes, bytes_index)?;
        for peer in self.iter() {
			bytes_utils::append_bytes_raw(&peer.as_boxed_bytes()?, &mut bytes, bytes_index)?;
		}
        Ok(bytes)
    }
    
    fn from_raw_bytes(bytes: &[u8]) -> Result<Self, Self::Error> {
		if bytes.len() < std::mem::size_of::<usize>() {
			return Err(format!("Peers::from_raw_bytes(): not enough bytes for 'len': expected {}, got {}", std::mem::size_of::<usize>(), bytes.len()));
		}
		let (mut beg, mut end);
		beg = 0;
		end = std::mem::size_of::<usize>();
		let len = usize::from_be_bytes(bytes[beg..end].try_into()
			.map_err(|_| String::from("Peers::from_raw_bytes(): failed to convert 'len'"))?);
		let mut peers = Self::with_capacity(len);
		for _ in 0..len {
			beg = end;
			if bytes.len() < beg {
				return Err(format!("Peers::from_raw_bytes(): not enough bytes for next peer: expected {}, got {}", beg, bytes.len()));
			}
			let peer = Peer::from_raw_bytes(&bytes[beg..])?;
			end += peer.size_of_self();
			let _ = peers.insert(peer);
		}
		Ok(peers)
	}
}


#[cfg(test)]
mod test {
    use super::*;
    
    #[test]
    fn peer_struct_layout() {
		{
			let ipv4_peer = Peer::new(std::net::SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED), 0));
			let expected_peer_size = (std::mem::size_of::<u16>() * 2) + std::mem::size_of::<u32>();
			assert_eq!(ipv4_peer.size_of_self(), expected_peer_size);
			let expected_peer_alignment: usize = std::mem::align_of::<std::net::SocketAddr>();
			assert_eq!(std::mem::align_of::<Peer>(), expected_peer_alignment);
			let expected_tag_offset: usize = 0;
			assert_eq!(Peer::offset_of_tag(), expected_tag_offset);
			let expected_ip_offset: usize = expected_tag_offset + std::mem::size_of::<u16>();
			assert_eq!(Peer::offset_of_ip(), expected_ip_offset);
			let expected_port_offset = expected_ip_offset + std::mem::size_of::<u32>();
			assert_eq!(Peer::offset_of_port(true), expected_port_offset);
		}
		{
			let ipv6_peer = Peer::new(std::net::SocketAddr::new(std::net::IpAddr::V6(std::net::Ipv6Addr::UNSPECIFIED), 0));
			let expected_peer_size = (std::mem::size_of::<u16>() * 2) + std::mem::size_of::<u128>();
			assert_eq!(ipv6_peer.size_of_self(), expected_peer_size);
			let expected_peer_alignment: usize = std::mem::align_of::<std::net::SocketAddr>();
			assert_eq!(std::mem::align_of::<Peer>(), expected_peer_alignment);
			let expected_tag_offset: usize = 0;
			assert_eq!(Peer::offset_of_tag(), expected_tag_offset);
			let expected_ip_offset: usize = expected_tag_offset + std::mem::size_of::<u16>();
			assert_eq!(Peer::offset_of_ip(), expected_ip_offset);
			let expected_port_offset = expected_ip_offset + std::mem::size_of::<u128>();
			assert_eq!(Peer::offset_of_port(false), expected_port_offset);
		}
	}
    
    #[test]
    fn peer_bytes_conversion() {
		use bytes_utils::BytesConversion;
		{
			let original_peer = Peer::new(std::net::SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST), 1337));
			let peer_bytes = original_peer.as_actix_bytes().unwrap();
			let converted_peer = Peer::from_actix_bytes(peer_bytes).unwrap();
			assert!(original_peer == converted_peer);
		}
		{
			let original_peer = Peer::new(std::net::SocketAddr::new(std::net::IpAddr::V6(std::net::Ipv6Addr::LOCALHOST), 1337));
			let peer_bytes = original_peer.as_actix_bytes().unwrap();
			let converted_peer = Peer::from_actix_bytes(peer_bytes).unwrap();
			assert!(original_peer == converted_peer);
		}
	}
}
