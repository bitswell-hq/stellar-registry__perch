//! A JSON-RPC server on loopback that records every method it is called with
//! and answers from a handler. Enough HTTP/1.1 for ureq: one request per
//! read, `Content-Length` bodies, keep-alive.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};

use serde_json::{json, Value};

type Handler = dyn Fn(&str, &Value) -> Value + Send + Sync;

pub struct FakeRpc {
    pub url: String,
    calls: Arc<Mutex<Vec<(String, Value)>>>,
}

impl FakeRpc {
    /// `handler(method, params)` returns the JSON-RPC `result`, or a value
    /// whose top-level `"error"` is an object to answer with that error
    /// instead.
    pub fn start(handler: impl Fn(&str, &Value) -> Value + Send + Sync + 'static) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback");
        let url = format!("http://{}", listener.local_addr().unwrap());
        let calls = Arc::new(Mutex::new(Vec::new()));
        let handler: Arc<Handler> = Arc::new(handler);
        let recorded = calls.clone();
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(stream) = stream else { return };
                let handler = handler.clone();
                let recorded = recorded.clone();
                std::thread::spawn(move || serve(stream, &*handler, &recorded));
            }
        });
        Self { url, calls }
    }

    /// Every method called so far, in order.
    pub fn methods(&self) -> Vec<String> {
        self.calls
            .lock()
            .unwrap()
            .iter()
            .map(|(m, _)| m.clone())
            .collect()
    }

    /// The params of every call to `method`, in order.
    pub fn params(&self, method: &str) -> Vec<Value> {
        self.calls
            .lock()
            .unwrap()
            .iter()
            .filter(|(m, _)| m == method)
            .map(|(_, p)| p.clone())
            .collect()
    }
}

fn serve(stream: TcpStream, handler: &Handler, recorded: &Mutex<Vec<(String, Value)>>) {
    let mut writer = stream.try_clone().expect("clone stream");
    let mut reader = BufReader::new(stream);
    loop {
        let mut length = 0usize;
        let mut line = String::new();
        // Request line, then headers up to the blank line.
        if reader.read_line(&mut line).unwrap_or(0) == 0 {
            return;
        }
        loop {
            line.clear();
            if reader.read_line(&mut line).unwrap_or(0) == 0 {
                return;
            }
            let header = line.trim_end();
            if header.is_empty() {
                break;
            }
            if let Some((name, value)) = header.split_once(':') {
                if name.eq_ignore_ascii_case("content-length") {
                    length = value.trim().parse().unwrap_or(0);
                }
            }
        }
        let mut body = vec![0u8; length];
        if reader.read_exact(&mut body).is_err() {
            return;
        }
        let request: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
        let method = request["method"].as_str().unwrap_or_default().to_string();
        let params = request["params"].clone();
        recorded
            .lock()
            .unwrap()
            .push((method.clone(), params.clone()));
        let answer = handler(&method, &params);
        let response = match answer.get("error").filter(|e| e.is_object()) {
            Some(e) => json!({ "jsonrpc": "2.0", "id": 1, "error": e }),
            None => json!({ "jsonrpc": "2.0", "id": 1, "result": answer }),
        };
        let out = serde_json::to_vec(&response).unwrap();
        let head = format!(
            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n",
            out.len()
        );
        if writer.write_all(head.as_bytes()).is_err() || writer.write_all(&out).is_err() {
            return;
        }
    }
}
