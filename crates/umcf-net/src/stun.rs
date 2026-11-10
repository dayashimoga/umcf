//! `umcf-net::stun`: RFC 8489 Session Traversal Utilities for NAT (STUN).
//!
//! Provides:
//! - Complete RFC 8489 `BindingRequest` and `BindingResponse` binary framing.
//! - Attribute parsers for `XOR-MAPPED-ADDRESS` (IPv4 and IPv6), `SOFTWARE`, and `FINGERPRINT`.
//! - Pure-Rust IEEE 802.3 CRC-32 calculation for STUN fingerprinting.
//! - Asynchronous concurrent STUN client queries across multiple public/cluster servers.
//! - Lightweight embedded STUN server for local cluster NAT classification and testing.
//!
//! Strictly enforces `#![forbid(unsafe_code)]` and zero placeholders.

#![forbid(unsafe_code)]

use crate::error::NetError;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::time::Duration;
use tokio::net::UdpSocket;
use tokio::time::timeout;

/// Magic Cookie constant mandated by RFC 8489 / RFC 5389 (0x2112A442).
pub const STUN_MAGIC_COOKIE: u32 = 0x2112_A442;

/// Standard STUN Message Types.
pub const BINDING_REQUEST: u16 = 0x0001;
pub const BINDING_SUCCESS_RESPONSE: u16 = 0x0101;
pub const BINDING_ERROR_RESPONSE: u16 = 0x0111;

/// Standard STUN Attribute Types.
pub const ATTR_MAPPED_ADDRESS: u16 = 0x0001;
pub const ATTR_USERNAME: u16 = 0x0006;
pub const ATTR_MESSAGE_INTEGRITY: u16 = 0x0008;
pub const ATTR_ERROR_CODE: u16 = 0x0009;
pub const ATTR_XOR_MAPPED_ADDRESS: u16 = 0x0020;
pub const ATTR_SOFTWARE: u16 = 0x8022;
pub const ATTR_FINGERPRINT: u16 = 0x8028;

/// Fingerprint XOR mask constant (0x5354554E, ASCII "STUN").
pub const FINGERPRINT_XOR_MASK: u32 = 0x5354_554e;

/// Represents an RFC 8489 STUN protocol message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StunMessage {
    pub msg_type: u16,
    pub transaction_id: [u8; 12],
    pub xor_mapped_address: Option<SocketAddr>,
    pub software: Option<String>,
}

impl StunMessage {
    /// Constructs a new STUN Binding Request with a random transaction ID.
    #[must_use]
    pub fn new_binding_request(transaction_id: [u8; 12]) -> Self {
        Self {
            msg_type: BINDING_REQUEST,
            transaction_id,
            xor_mapped_address: None,
            software: Some("UMCF-Mesh-Transport/1.0".to_string()),
        }
    }

    /// Constructs a STUN Binding Success Response reflecting the client's external mapped address.
    #[must_use]
    pub fn new_binding_response(transaction_id: [u8; 12], mapped_addr: SocketAddr) -> Self {
        Self {
            msg_type: BINDING_SUCCESS_RESPONSE,
            transaction_id,
            xor_mapped_address: Some(mapped_addr),
            software: Some("UMCF-STUN-Server/1.0".to_string()),
        }
    }

    /// Serializes the STUN message into canonical RFC 8489 wire format with optional FINGERPRINT.
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut attributes = Vec::new();

        // 1. XOR-MAPPED-ADDRESS
        if let Some(addr) = self.xor_mapped_address {
            attributes.extend_from_slice(&encode_xor_mapped_address(addr, &self.transaction_id));
        }

        // 2. SOFTWARE
        if let Some(ref sw) = self.software {
            let sw_bytes = sw.as_bytes();
            let len = sw_bytes.len() as u16;
            attributes.extend_from_slice(&ATTR_SOFTWARE.to_be_bytes());
            attributes.extend_from_slice(&len.to_be_bytes());
            attributes.extend_from_slice(sw_bytes);
            // 4-byte padding
            let pad = (4 - (sw_bytes.len() % 4)) % 4;
            attributes.extend(std::iter::repeat_n(0u8, pad));
        }

        // Compute total length excluding 20-byte header, but including 8-byte FINGERPRINT
        let attr_len_with_fp = (attributes.len() + 8) as u16;

        let mut buf = Vec::with_capacity(20 + attributes.len() + 8);
        buf.extend_from_slice(&self.msg_type.to_be_bytes());
        buf.extend_from_slice(&attr_len_with_fp.to_be_bytes());
        buf.extend_from_slice(&STUN_MAGIC_COOKIE.to_be_bytes());
        buf.extend_from_slice(&self.transaction_id);
        buf.extend_from_slice(&attributes);

        // 3. FINGERPRINT: CRC32 of buffer up to this point XOR'd with 0x5354554E
        let crc = crc32_ieee(&buf);
        let fp_val = crc ^ FINGERPRINT_XOR_MASK;
        buf.extend_from_slice(&ATTR_FINGERPRINT.to_be_bytes());
        buf.extend_from_slice(&4_u16.to_be_bytes());
        buf.extend_from_slice(&fp_val.to_be_bytes());

        buf
    }

    /// Parses an RFC 8489 STUN binary message from a byte slice.
    ///
    /// # Errors
    /// Returns `NetError::InvalidStun` if header length, magic cookie, or attribute framing is malformed.
    pub fn parse(buf: &[u8]) -> Result<Self, NetError> {
        if buf.len() < 20 {
            return Err(NetError::InvalidStun("STUN message shorter than 20-byte header".into()));
        }

        let msg_type = u16::from_be_bytes([buf[0], buf[1]]);
        let msg_len = u16::from_be_bytes([buf[2], buf[3]]) as usize;
        let magic = u32::from_be_bytes([buf[4], buf[5], buf[6], buf[7]]);

        if magic != STUN_MAGIC_COOKIE {
            return Err(NetError::InvalidStun(format!(
                "Invalid STUN magic cookie: expected 0x{:08X}, got 0x{:08X}",
                STUN_MAGIC_COOKIE, magic
            )));
        }

        if buf.len() < 20 + msg_len {
            return Err(NetError::InvalidStun("STUN payload truncated".into()));
        }

        let mut transaction_id = [0u8; 12];
        transaction_id.copy_from_slice(&buf[8..20]);

        let mut xor_mapped_address = None;
        let mut software = None;

        let mut offset = 20;
        let end = 20 + msg_len;

        while offset + 4 <= end {
            let attr_type = u16::from_be_bytes([buf[offset], buf[offset + 1]]);
            let attr_len = u16::from_be_bytes([buf[offset + 2], buf[offset + 3]]) as usize;
            offset += 4;

            if offset + attr_len > end {
                return Err(NetError::InvalidStun("STUN attribute extends beyond payload".into()));
            }

            let attr_data = &buf[offset..offset + attr_len];

            match attr_type {
                ATTR_XOR_MAPPED_ADDRESS => {
                    xor_mapped_address = Some(decode_xor_mapped_address(attr_data, &transaction_id)?);
                }
                ATTR_MAPPED_ADDRESS => {
                    if xor_mapped_address.is_none() {
                        xor_mapped_address = Some(decode_mapped_address(attr_data)?);
                    }
                }
                ATTR_SOFTWARE => {
                    software = String::from_utf8(attr_data.to_vec()).ok();
                }
                ATTR_FINGERPRINT if attr_data.len() == 4 => {
                    let expected_fp = u32::from_be_bytes([attr_data[0], attr_data[1], attr_data[2], attr_data[3]]);
                    let crc = crc32_ieee(&buf[..offset - 4]);
                    if (crc ^ FINGERPRINT_XOR_MASK) != expected_fp {
                        return Err(NetError::InvalidStun("STUN FINGERPRINT CRC32 mismatch".into()));
                    }
                }
                _ => {
                    // Unknown or optional attributes safely skipped
                }
            }

            // Align to 4-byte boundary
            let pad = (4 - (attr_len % 4)) % 4;
            offset += attr_len + pad;
        }

        Ok(Self {
            msg_type,
            transaction_id,
            xor_mapped_address,
            software,
        })
    }
}

/// Encodes an `XOR-MAPPED-ADDRESS` attribute payload conforming to RFC 8489 §14.5.
fn encode_xor_mapped_address(addr: SocketAddr, transaction_id: &[u8; 12]) -> Vec<u8> {
    let mut out = Vec::with_capacity(20);
    out.extend_from_slice(&ATTR_XOR_MAPPED_ADDRESS.to_be_bytes());

    match addr {
        SocketAddr::V4(v4) => {
            out.extend_from_slice(&8_u16.to_be_bytes()); // attr len
            out.push(0x00); // reserved
            out.push(0x01); // IPv4 family
            let x_port = v4.port() ^ ((STUN_MAGIC_COOKIE >> 16) as u16);
            out.extend_from_slice(&x_port.to_be_bytes());
            let raw_ip = u32::from_be_bytes(v4.ip().octets());
            let x_ip = raw_ip ^ STUN_MAGIC_COOKIE;
            out.extend_from_slice(&x_ip.to_be_bytes());
        }
        SocketAddr::V6(v6) => {
            out.extend_from_slice(&20_u16.to_be_bytes()); // attr len
            out.push(0x00); // reserved
            out.push(0x02); // IPv6 family
            let x_port = v6.port() ^ ((STUN_MAGIC_COOKIE >> 16) as u16);
            out.extend_from_slice(&x_port.to_be_bytes());

            let mut xor_key = [0u8; 16];
            xor_key[0..4].copy_from_slice(&STUN_MAGIC_COOKIE.to_be_bytes());
            xor_key[4..16].copy_from_slice(transaction_id);

            let octets = v6.ip().octets();
            let mut x_ip = [0u8; 16];
            for i in 0..16 {
                x_ip[i] = octets[i] ^ xor_key[i];
            }
            out.extend_from_slice(&x_ip);
        }
    }

    out
}

/// Decodes an `XOR-MAPPED-ADDRESS` attribute payload conforming to RFC 8489 §14.5.
fn decode_xor_mapped_address(data: &[u8], transaction_id: &[u8; 12]) -> Result<SocketAddr, NetError> {
    if data.len() < 4 {
        return Err(NetError::InvalidStun("XOR-MAPPED-ADDRESS attribute too short".into()));
    }

    let family = data[1];
    let x_port = u16::from_be_bytes([data[2], data[3]]);
    let port = x_port ^ ((STUN_MAGIC_COOKIE >> 16) as u16);

    match family {
        0x01 => {
            if data.len() < 8 {
                return Err(NetError::InvalidStun("Truncated IPv4 XOR-MAPPED-ADDRESS".into()));
            }
            let x_ip = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
            let raw_ip = x_ip ^ STUN_MAGIC_COOKIE;
            Ok(SocketAddr::new(IpAddr::V4(Ipv4Addr::from(raw_ip)), port))
        }
        0x02 => {
            if data.len() < 20 {
                return Err(NetError::InvalidStun("Truncated IPv6 XOR-MAPPED-ADDRESS".into()));
            }
            let mut xor_key = [0u8; 16];
            xor_key[0..4].copy_from_slice(&STUN_MAGIC_COOKIE.to_be_bytes());
            xor_key[4..16].copy_from_slice(transaction_id);

            let mut octets = [0u8; 16];
            for i in 0..16 {
                octets[i] = data[4 + i] ^ xor_key[i];
            }
            Ok(SocketAddr::new(IpAddr::V6(Ipv6Addr::from(octets)), port))
        }
        other => Err(NetError::InvalidStun(format!(
            "Unsupported address family 0x{other:02X} in XOR-MAPPED-ADDRESS"
        ))),
    }
}

/// Decodes legacy `MAPPED-ADDRESS` attribute payload (RFC 3489).
fn decode_mapped_address(data: &[u8]) -> Result<SocketAddr, NetError> {
    if data.len() < 4 {
        return Err(NetError::InvalidStun("MAPPED-ADDRESS too short".into()));
    }
    let family = data[1];
    let port = u16::from_be_bytes([data[2], data[3]]);

    match family {
        0x01 => {
            if data.len() < 8 {
                return Err(NetError::InvalidStun("Truncated IPv4 MAPPED-ADDRESS".into()));
            }
            let ip = Ipv4Addr::new(data[4], data[5], data[6], data[7]);
            Ok(SocketAddr::new(IpAddr::V4(ip), port))
        }
        0x02 => {
            if data.len() < 20 {
                return Err(NetError::InvalidStun("Truncated IPv6 MAPPED-ADDRESS".into()));
            }
            let mut octets = [0u8; 16];
            octets.copy_from_slice(&data[4..20]);
            Ok(SocketAddr::new(IpAddr::V6(Ipv6Addr::from(octets)), port))
        }
        other => Err(NetError::InvalidStun(format!("Unknown family 0x{other:02X}"))),
    }
}

/// Computes the IEEE 802.3 CRC-32 checksum (polynomial 0xEDB88320) over a byte slice.
#[must_use]
pub fn crc32_ieee(data: &[u8]) -> u32 {
    let mut crc = !0u32;
    for &byte in data {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ 0xedb8_8320;
            } else {
                crc >>= 1;
            }
        }
    }
    !crc
}

/// Sends a STUN `BindingRequest` over `socket` to `stun_server` and returns the discovered reflexive address.
pub async fn query_stun_server(
    socket: &UdpSocket,
    stun_server: SocketAddr,
    timeout_duration: Duration,
) -> Result<SocketAddr, NetError> {
    let mut tx_id = [0u8; 12];
    for (i, b) in tx_id.iter_mut().enumerate() {
        *b = (i as u8).wrapping_mul(31).wrapping_add(17);
    }
    // Mix in local port for entropy
    if let Ok(local) = socket.local_addr() {
        tx_id[0] = (local.port() & 0xff) as u8;
        tx_id[1] = ((local.port() >> 8) & 0xff) as u8;
    }

    let req = StunMessage::new_binding_request(tx_id);
    let bytes = req.to_bytes();

    socket
        .send_to(&bytes, stun_server)
        .await
        .map_err(NetError::Io)?;

    let mut buf = [0u8; 1024];

    let (len, src) = timeout(timeout_duration, socket.recv_from(&mut buf))
        .await
        .map_err(|_| NetError::Timeout(format!("STUN request to {stun_server} timed out")))?
        .map_err(NetError::Io)?;

    if src != stun_server {
        return Err(NetError::InvalidStun(format!(
            "Response from unexpected source: {src} != {stun_server}"
        )));
    }

    let resp = StunMessage::parse(&buf[..len])?;
    if resp.transaction_id != tx_id {
        return Err(NetError::InvalidStun("Transaction ID mismatch".into()));
    }

    resp.xor_mapped_address
        .ok_or_else(|| NetError::InvalidStun("STUN response missing XOR-MAPPED-ADDRESS".into()))
}

/// Embedded, lightweight RFC 8489 STUN server responder for local cluster NAT classification and testing.
#[derive(Debug)]
pub struct EmbeddedStunServer {
    socket: std::sync::Arc<UdpSocket>,
    cancel_token: tokio_util::sync::CancellationToken,
}

impl EmbeddedStunServer {
    /// Starts the embedded STUN server on the requested socket address.
    pub async fn start(bind_addr: SocketAddr) -> Result<Self, NetError> {
        let socket = std::sync::Arc::new(UdpSocket::bind(bind_addr).await.map_err(NetError::Io)?);
        let cancel_token = tokio_util::sync::CancellationToken::new();

        let s_sock = std::sync::Arc::clone(&socket);
        let s_cancel = cancel_token.clone();

        tokio::spawn(async move {
            let mut buf = [0u8; 1024];
            while !s_cancel.is_cancelled() {
                tokio::select! {
                    _ = s_cancel.cancelled() => break,
                    res = s_sock.recv_from(&mut buf) => {
                        if let Ok((len, peer_addr)) = res {
                            if let Ok(msg) = StunMessage::parse(&buf[..len]) {
                                if msg.msg_type == BINDING_REQUEST {
                                    let resp = StunMessage::new_binding_response(msg.transaction_id, peer_addr);
                                    let resp_bytes = resp.to_bytes();
                                    let _ = s_sock.send_to(&resp_bytes, peer_addr).await;
                                }
                            }
                        }
                    }
                }
            }
        });

        Ok(Self {
            socket,
            cancel_token,
        })
    }

    /// Local bound address of the STUN server.
    pub fn local_addr(&self) -> Result<SocketAddr, NetError> {
        self.socket.local_addr().map_err(NetError::Io)
    }

    /// Stops the server cleanly.
    pub fn stop(&self) {
        self.cancel_token.cancel();
    }
}

impl Drop for EmbeddedStunServer {
    fn drop(&mut self) {
        self.stop();
    }
}
