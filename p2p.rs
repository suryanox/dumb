use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    io::{BufRead, BufReader, Write},
    net::{TcpListener, TcpStream},
    sync::{Arc, Mutex},
    thread,
};

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Message {
    from: String,
    text: String,
}

type Peers = Arc<Mutex<HashMap<String, TcpStream>>>;

fn broadcast(peers: &Peers, msg: &Message, skip: Option<&str>) {
    let json = serde_json::to_string(msg).unwrap() + "\n";
    let mut map = peers.lock().unwrap();
    map.retain(|addr, stream| {
        if skip.map_or(false, |s| s == addr) {
            return true;
        }
        stream.write_all(json.as_bytes()).is_ok()
    });
}

fn handle_connection(stream: TcpStream, peers: Peers, my_addr: String) {
    let peer_addr = stream.peer_addr().unwrap().to_string();
    println!("[+] connected: {peer_addr}");

    peers
        .lock()
        .unwrap()
        .insert(peer_addr.clone(), stream.try_clone().unwrap());

    let reader = BufReader::new(stream);
    for line in reader.lines() {
        match line {
            Ok(json) => match serde_json::from_str::<Message>(&json) {
                Ok(msg) => {
                    println!("{}: {}", msg.from, msg.text);
                    broadcast(&peers, &msg, Some(&peer_addr));
                }
                Err(_) => {}
            },
            Err(_) => break,
        }
    }

    println!("[-] disconnected: {peer_addr}");
    peers.lock().unwrap().remove(&peer_addr);
    let _ = my_addr;
}

fn connect_to(addr: &str, peers: Peers, my_addr: String) {
    match TcpStream::connect(addr) {
        Ok(stream) => {
            println!("[+] outbound → {addr}");
            let peers2 = peers.clone();
            let my = my_addr.clone();
            let s = stream.try_clone().unwrap();
            peers.lock().unwrap().insert(addr.to_string(), s);
            thread::spawn(move || handle_connection(stream, peers2, my));
        }
        Err(e) => eprintln!("[!] could not connect to {addr}: {e}"),
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <port> [peer_addr ...]", args[0]);
        std::process::exit(1);
    }

    let port = &args[1];
    let my_addr = format!("127.0.0.1:{port}");
    let peers: Peers = Arc::new(Mutex::new(HashMap::new()));

    let listener = TcpListener::bind(&my_addr).expect("bind failed");
    println!("Listening on {my_addr}");

    {
        let peers = peers.clone();
        let my = my_addr.clone();
        thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let peers = peers.clone();
                let my = my.clone();
                thread::spawn(move || handle_connection(stream, peers, my));
            }
        });
    }

    for peer_addr in &args[2..] {
        connect_to(peer_addr, peers.clone(), my_addr.clone());
    }

    println!("Type messages and press Enter to send.\n");
    let stdin = std::io::stdin();
    for line in stdin.lock().lines() {
        let text = match line {
            Ok(t) if !t.trim().is_empty() => t,
            _ => continue,
        };
        let msg = Message { from: my_addr.clone(), text };
        broadcast(&peers, &msg, None);
    }
}
