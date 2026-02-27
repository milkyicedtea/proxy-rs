use std::{env, fs, sync::Arc};
use tokio::{io, net::TcpListener};

mod connection_handler;
mod structs;
mod time;

use connection_handler::handle_connection;
use structs::Config;
use time::now_eu;

fn load_config(path: &str) -> Config {
    let content = fs::read_to_string(path).expect("Failed to read config file");

    toml::from_str(&content).expect("Invalid config format")
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let local_port: u16 = env::var("LOCAL_PORT")
        .unwrap_or_else(|_| "25565".to_string())
        .parse()
        .expect("Invalid LOCAL_PORT");

    let config = Arc::new(load_config("config.toml"));

    let listener = TcpListener::bind(("0.0.0.0", local_port)).await?;
    println!("[{}] TCP proxy listening on port {}", now_eu(), local_port);

    loop {
        let (mut client_socket, client_addr) = listener.accept().await?;

        client_socket.set_nodelay(true)?;
        client_socket.set_quickack(true)?;

        let config = config.clone();

        tokio::spawn(async move {
            if let Err(e) = handle_connection(&mut client_socket, client_addr, config).await {
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

// async fn warm_connection_manager(
//     mut request_rx: mpsc::Receiver<mpsc::Sender<TcpStream>>,
//     host: String,
//     port: u16,
// ) {
//     // Always maintain exactly 1 warm connection ready to go
//     let mut warm_conn: Option<TcpStream> = None;

//     loop {
//         // If we don't have a warm connection, create one
//         if warm_conn.is_none() {
//             match TcpStream::connect((host.as_str(), port)).await {
//                 Ok(conn) => {
//                     conn.set_nodelay(true).ok();
//                     conn.set_quickack(true).ok();
//                     println!("[{}] [+] Warm connection created and ready", now_eu());
//                     warm_conn = Some(conn);
//                 }
//                 Err(e) => {
//                     eprintln!("[{}] [!] Failed to create warm connection: {}", now_eu(), e);
//                     tokio::time::sleep(std::time::Duration::from_secs(1)).await;
//                     continue;
//                 }
//             }
//         }

//         // Wait for a request for a connection
//         if let Some(response_tx) = request_rx.recv().await {
//             if let Some(conn) = warm_conn.take() {
//                 println!("[{}] [+] Serving warm connection to client", now_eu());
//                 let _ = response_tx.send(conn).await;
//                 // warm_conn is now None, so the loop will create a new one immediately
//             }
//         } else {
//             // Channel closed, exit
//             break;
//         }
//     }
// }
