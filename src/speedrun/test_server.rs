//! A local HTTP server for tests of the speedrun.com workers, so no test makes
//! a real network request.

use std::io::{BufRead, BufReader, Write};
use std::net::{SocketAddr, TcpListener};
use std::sync::{Arc, mpsc};
use std::time::Duration;

/// What a test server received for one request.
pub struct Received {
    /// The first line of the request, like `GET /games HTTP/1.1`.
    pub request_line: String,
    /// The header lines, without their line endings.
    pub headers: Vec<String>,
}

/// Serves HTTP on 127.0.0.1, one thread per connection. `respond` gets the
/// request and the number of the connection (from 0) and returns the whole
/// response, or `None` to leave the connection unanswered. Every request is
/// also sent on the returned channel.
pub fn serve<F>(respond: F) -> (SocketAddr, mpsc::Receiver<Received>)
where
    F: Fn(&Received, usize) -> Option<String> + Send + Sync + 'static,
{
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let (seen_tx, seen_rx) = mpsc::channel();
    let respond = Arc::new(respond);
    std::thread::spawn(move || {
        for (number, stream) in listener.incoming().enumerate() {
            let Ok(mut stream) = stream else { continue };
            let seen_tx = seen_tx.clone();
            let respond = respond.clone();
            std::thread::spawn(move || {
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut request_line = String::new();
                if reader.read_line(&mut request_line).unwrap_or(0) == 0 {
                    return;
                }
                let mut headers = Vec::new();
                loop {
                    let mut line = String::new();
                    if reader.read_line(&mut line).unwrap_or(0) == 0 {
                        return;
                    }
                    let line = line.trim_end().to_string();
                    if line.is_empty() {
                        break;
                    }
                    headers.push(line);
                }
                let received = Received {
                    request_line: request_line.trim_end().to_string(),
                    headers,
                };
                let response = {
                    // Report the request before answering, so a test can wait for it.
                    let _ = seen_tx.send(Received {
                        request_line: received.request_line.clone(),
                        headers: received.headers.clone(),
                    });
                    respond(&received, number)
                };
                if let Some(response) = response {
                    let _ = stream.write_all(response.as_bytes());
                    let _ = stream.flush();
                } else {
                    // Keep the connection open without answering.
                    std::thread::sleep(Duration::from_secs(10));
                }
            });
        }
    });
    (addr, seen_rx)
}

/// A 200 response carrying `body`.
pub fn ok(body: &str) -> String {
    format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
}
