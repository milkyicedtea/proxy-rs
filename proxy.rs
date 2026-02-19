use std::env;
use std::net::SocketAddr;
use tokio::io;
use tokio::net::{TcpListener, TcpStream};

use chrono::Utc;
use chrono_tz::Europe::Rome;
use tokio::sync::mpsc;

fn now_eu() -> String {
    let t = Utc::now().with_timezone(&Rome);
    t.format("%Y-%m-%d %H:%M:%S").to_string()
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let local_port: u16 = env::var("LOCAL_PORT")
        .unwrap_or_else(|_| "25565".to_string())
        .parse()
        .expect("Invalid LOCAL_PORT");
    let remote_host = env::var("REMOTE_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let remote_port: u16 = env::var("REMOTE_PORT")
        .unwrap_or_else(|_| "25565".to_string())
        .parse()
        .expect("Invalid REMOTE_PORT");

    let listener = TcpListener::bind(("0.0.0.0", local_port)).await?;
    println!("[{}] TCP proxy listening on port {}", now_eu(), local_port);

    // Channel for requesting warm connections (handlers request from pool)
    let (request_tx, request_rx) = mpsc::channel::<mpsc::Sender<TcpStream>>(65536);

    // Spawn the warm connection pool manager
    tokio::spawn(warm_connection_manager(
        request_rx,
        remote_host.clone(),
        remote_port,
    ));

    loop {
        let (mut client_socket, client_addr) = listener.accept().await?;

        client_socket.set_nodelay(true)?;
        client_socket.set_quickack(true)?;

        let remote_host = remote_host.clone();
        let request_tx = request_tx.clone();

        tokio::spawn(async move {
            if let Err(e) = handle_connection(
                &mut client_socket,
                client_addr,
                &remote_host,
                remote_port,
                request_tx,
            )
            .await
            {
                eprintln!(
                    "[{}] [!] Connection error ({}): {}",
                    now_eu(),
                    client_addr,
                    e
                );
            }
        });
    }
}

async fn warm_connection_manager(
    mut request_rx: mpsc::Receiver<mpsc::Sender<TcpStream>>,
    host: String,
    port: u16,
) {
    // Always maintain exactly 1 warm connection ready to go
    let mut warm_conn: Option<TcpStream> = None;

    loop {
        // If we don't have a warm connection, create one
        if warm_conn.is_none() {
            match TcpStream::connect((host.as_str(), port)).await {
                Ok(conn) => {
                    conn.set_nodelay(true).ok();
                    conn.set_quickack(true).ok();
                    println!("[{}] [+] Warm connection created and ready", now_eu());
                    warm_conn = Some(conn);
                }
                Err(e) => {
                    eprintln!("[{}] [!] Failed to create warm connection: {}", now_eu(), e);
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    continue;
                }
            }
        }

        // Wait for a request for a connection
        if let Some(response_tx) = request_rx.recv().await {
            if let Some(conn) = warm_conn.take() {
                println!("[{}] [+] Serving warm connection to client", now_eu());
                let _ = response_tx.send(conn).await;
                // warm_conn is now None, so the loop will create a new one immediately
            }
        } else {
            // Channel closed, exit
            break;
        }
    }
}

async fn handle_connection(
    client: &mut TcpStream,
    client_addr: SocketAddr,
    remote_host: &str,
    remote_port: u16,
    request_tx: mpsc::Sender<mpsc::Sender<TcpStream>>,
) -> io::Result<()> {
    println!("[{}] [+] Connection started: {}", now_eu(), client_addr);

    // Request a warm connection from the pool
    let (response_tx, mut response_rx) = mpsc::channel::<TcpStream>(1);

    if request_tx.send(response_tx).await.is_err() {
        eprintln!("[{}] [!] Pool manager died", now_eu());
        return Ok(());
    }

    let mut remote = match response_rx.recv().await {
        Some(conn) => conn,
        None => {
            eprintln!("[{}] [!] Failed to get connection from pool", now_eu());
            // Fallback: create connection directly
            match TcpStream::connect((remote_host, remote_port)).await {
                Ok(conn) => {
                    conn.set_nodelay(true)?;
                    conn.set_quickack(true)?;
                    println!("[{}] [+] Fallback direct connection created", now_eu());
                    conn
                }
                Err(err) => {
                    eprintln!(
                        "[{}] [!] Remote connection failed ({}): {}",
                        now_eu(),
                        client_addr,
                        err
                    );
                    return Ok(());
                }
            }
        }
    };

    let (mut client_read, mut client_write) = client.split();
    let (mut remote_read, mut remote_write) = remote.split();

    // Start bidirectional forwarding
    let c_to_r = io::copy(&mut client_read, &mut remote_write);
    let r_to_c = io::copy(&mut remote_read, &mut client_write);

    tokio::select! {
        r1 = c_to_r => r1?,
        r2 = r_to_c => r2?,
    };

    println!("[{}] [-] Connection ended: {}", now_eu(), client_addr);
    // Connection drops here, which is exactly what we want

    Ok(())
}
