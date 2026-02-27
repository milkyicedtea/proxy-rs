use std::{net::SocketAddr, sync::Arc};
use tokio::{
    io::{self, AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

use crate::structs::Config;
use crate::time::now_eu;

fn read_varint_from_slice(buf: &mut &[u8]) -> io::Result<i32> {
    let mut num = 0i32;
    let mut shift = 0;

    loop {
        if buf.is_empty() {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "Varint EOF"));
        }

        let byte = buf[0];
        *buf = &buf[1..];

        num |= ((byte & 0x7F) as i32) << shift;

        if byte & 0x80 == 0 {
            break;
        }

        shift += 7;
        if shift > 35 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "VarInt too big"));
        }
    }

    Ok(num)
}

pub async fn handle_connection(
    client: &mut TcpStream,
    client_addr: SocketAddr,
    config: Arc<Config>,
) -> io::Result<()> {
    println!("[{}] [+] Connection started: {}", now_eu(), client_addr);

    // === Read full handshake packet ===

    // read first byte to start varint lenght
    let mut lenght_buf = Vec::new();
    loop {
        let mut byte = [0u8; 1];
        client.read_exact(&mut byte).await?;
        lenght_buf.push(byte[0]);

        if byte[0] & 0x80 == 0 {
            break;
        }
    }

    let mut lenght_slice = lenght_buf.as_slice();
    let packet_lenght = read_varint_from_slice(&mut lenght_slice)?;

    let mut packet_data = vec![0u8; packet_lenght as usize];
    client.read_exact(&mut packet_data).await?;

    // keep full handshake
    let mut full_packet = lenght_buf.clone();
    full_packet.extend_from_slice(&packet_data);

    // === Parse hostname from packet ===
    let mut slice = packet_data.as_slice();

    let packet_id = read_varint_from_slice(&mut slice)?;
    if packet_id != 0 {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "Not handshake"));
    }

    let _protocol_version = read_varint_from_slice(&mut slice)?;

    let str_len = read_varint_from_slice(&mut slice)? as usize;

    if slice.len() < str_len {
        return Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "Invalid hostname length",
        ));
    }

    let hostname = String::from_utf8_lossy(&slice[..str_len]).to_string();

    println!("[{}] Hostname requested: {}", now_eu(), hostname);

    // === Lookup backend ===
    let backend = config
        .backends
        .iter()
        .find(|(key, backend)| {
            // allow if:
            // 1) handshake hostname matches domain key
            // 2) handshake hostname matches backend host (ip)
            hostname == **key || hostname == backend.host
        })
        .map(|(_, backend)| backend.clone());

    if let Some(backend) = backend {
        println!(
            "[{}] Routing {} → {}:{}",
            now_eu(),
            hostname,
            backend.host,
            backend.port
        );
    
        // === Connect to backend ===
        let mut remote = TcpStream::connect((backend.host.as_str(), backend.port)).await?;
        remote.set_nodelay(true)?;
    
        // Forward handshake we already consumed
        remote.write_all(&full_packet).await?;
    
        // === Bidirectional forwarding ===
        let (mut client_read, mut client_write) = client.split();
        let (mut remote_read, mut remote_write) = remote.split();
    
        tokio::select! {
            r1 = io::copy(&mut client_read, &mut remote_write) => r1?,
            r2 = io::copy(&mut remote_read, &mut client_write) => r2?,
        };
    
        println!("[{}] [-] Connection ended: {}", now_eu(), client_addr);
    } else {
        eprintln!("[{}] Forbidden hostname/IP: {}", now_eu(), hostname);
        return Ok(());
    }
    
    Ok(())
}
