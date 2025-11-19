use crate::error::Result;
use log::trace;
use std::{
    net::{Shutdown, SocketAddr, TcpStream, UdpSocket},
    str::FromStr,
    time::Duration,
};

pub fn connect_timeout(ip: &str, port: u16, timeout: u16) -> bool {
    trace!("net:try_connect(ip: &str, port: u16, timeout: u16) -> bool");
    if let Ok(addr) = SocketAddr::from_str(&format!("{ip}:{port}")) {
        if let Ok(socket) = TcpStream::connect_timeout(&addr, Duration::from_millis(timeout as u64))
        {
            if let Ok(_) = socket.shutdown(Shutdown::Write) {
                return true;
            }
        }
    }
    return false;
}

pub fn send_wol(mac_address: &str) -> Result<()> {
    trace!("net::send_wol(mac_address: &str) -> Result<()>");

    let mac_address = mac_address.replace(['-', ':'], "");
    let mut mac = [0u8; 6];
    for i in 0..6 {
        let byte_str = &mac_address[i * 2..i * 2 + 2];
        mac[i] = u8::from_str_radix(byte_str, 16)?;
    }

    let mut packet = Vec::new();
    for _ in 0..6 {
        packet.push(0xFF); // 6 bytes of 0xFF
    }
    for _ in 0..16 {
        packet.extend_from_slice(&mac); // 16 repetitions of MAC address
    }

    let socket = UdpSocket::bind("0.0.0.0:0")?;
    socket.set_broadcast(true)?;
    socket.send_to(&packet, "255.255.255.255:9")?;

    Ok(())
}
