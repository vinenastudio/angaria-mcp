//! angaria-mcp: the stdio MCP server for clients that take only stdio
//! servers (Claude Desktop) and for configs written before Angaria served a
//! plain URL. It forwards each JSON-RPC line to the MCP server built into the
//! running Angaria app and writes the answer back. It reads no file and never
//! opens Angaria: with Angaria closed, nothing listens and every request gets
//! an error.

use std::collections::HashMap;
use std::env;
use std::io::{self, BufRead, Read, Write};
use std::net::{Shutdown, SocketAddr, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use base64::Engine;
use serde_json::{json, Value};

/// The one port the Angaria app serves MCP on.
const MCP_PORT: u16 = 41234;
/// Overrides the port, for tests only (they run a server on a free port).
const PORT_ENV: &str = "ANGARIA_MCP_PORT";
const META_VERSION: &str = "io.modelcontextprotocol/protocolVersion";
/// Longer than the server's own wait for a tool (120 seconds).
const READ_TIMEOUT: Duration = Duration::from_secs(150);
const INTERNAL_ERROR: i64 = -32603;

const NOT_REACHABLE: &str = "Angaria’ya bağlanılamadı. Angaria’yı aç.";
const CONNECTION_LOST: &str =
  "Angaria’yla bağlantı yanıt gelmeden koptu. İşlemin yapılıp yapılmadığını Angaria’da kontrol et.";

fn port() -> u16 {
  env::var(PORT_ENV)
    .ok()
    .and_then(|port| port.parse().ok())
    .unwrap_or(MCP_PORT)
}

/// Visible ASCII as is; anything else in the `=?base64?...?=` form MCP defines.
fn header_safe(value: &str) -> String {
  let plain = value.bytes().all(|byte| (0x20..0x7f).contains(&byte))
    && value.trim() == value
    && !(value.starts_with("=?base64?") && value.ends_with("?="));
  if plain {
    value.to_string()
  } else {
    format!(
      "=?base64?{}?=",
      base64::engine::general_purpose::STANDARD.encode(value)
    )
  }
}

/// The MCP headers of `message`: a modern request mirrors its version, method
/// and tool name; a legacy one sends the version `initialize` agreed on.
fn mcp_headers(message: &Value, negotiated: Option<&str>) -> Vec<(&'static str, String)> {
  let mut headers = Vec::new();
  let modern = message["params"]["_meta"][META_VERSION].as_str();
  if let Some(version) = modern.or(negotiated) {
    headers.push(("MCP-Protocol-Version", header_safe(version)));
  }
  if modern.is_some() {
    if let Some(method) = message["method"].as_str() {
      headers.push(("Mcp-Method", header_safe(method)));
    }
    let params = &message["params"];
    if let Some(name) = params["name"].as_str().or_else(|| params["uri"].as_str()) {
      headers.push(("Mcp-Name", header_safe(name)));
    }
  }
  headers
}

enum PostError {
  /// Nothing was sent: the server is not there.
  Connect,
  /// The request went out; it must not be sent again.
  Lost,
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
  haystack
    .windows(needle.len())
    .position(|window| window == needle)
}

fn dechunk(mut body: &[u8]) -> Option<Vec<u8>> {
  let mut out = Vec::new();
  loop {
    let line_end = find(body, b"\r\n")?;
    let size_text = std::str::from_utf8(&body[..line_end]).ok()?;
    let size = usize::from_str_radix(size_text.split(';').next()?.trim(), 16).ok()?;
    body = &body[line_end + 2..];
    if size == 0 {
      return Some(out);
    }
    out.extend_from_slice(body.get(..size)?);
    body = body.get(size + 2..)?;
  }
}

/// Status and body of a whole HTTP/1.1 response.
fn parse_response(raw: &[u8]) -> Option<(u16, Vec<u8>)> {
  let head_end = find(raw, b"\r\n\r\n")?;
  let head = std::str::from_utf8(&raw[..head_end]).ok()?;
  let status = head.split(' ').nth(1)?.parse().ok()?;
  let body = &raw[head_end + 4..];
  let chunked = head.lines().skip(1).any(|line| {
    line.split_once(':').is_some_and(|(name, value)| {
      name.trim().eq_ignore_ascii_case("transfer-encoding")
        && value.trim().eq_ignore_ascii_case("chunked")
    })
  });
  let body = if chunked {
    dechunk(body)?
  } else {
    body.to_vec()
  };
  Some((status, body))
}

/// Sends one request on its own connection. `opened` gets a handle to the
/// connection once it is up, so a cancellation can close it.
fn post(
  port: u16,
  headers: &[(&'static str, String)],
  body: &[u8],
  opened: impl FnOnce(&TcpStream),
) -> Result<(u16, Vec<u8>), PostError> {
  let address = SocketAddr::from(([127, 0, 0, 1], port));
  let mut stream = TcpStream::connect_timeout(&address, Duration::from_secs(2))
    .map_err(|_| PostError::Connect)?;
  opened(&stream);
  let mut head = format!(
    "POST /mcp HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n\
     Content-Type: application/json\r\nAccept: application/json, text/event-stream\r\n\
     Content-Length: {}\r\nConnection: close\r\n",
    body.len()
  );
  for (name, value) in headers {
    head.push_str(&format!("{name}: {value}\r\n"));
  }
  head.push_str("\r\n");
  let mut raw = Vec::new();
  stream
    .set_read_timeout(Some(READ_TIMEOUT))
    .and_then(|()| stream.write_all(head.as_bytes()))
    .and_then(|()| stream.write_all(body))
    .and_then(|()| stream.read_to_end(&mut raw))
    .map_err(|_| PostError::Lost)?;
  parse_response(&raw).ok_or(PostError::Lost)
}

fn error_reply(id: Value, message: &str) -> Value {
  json!({ "jsonrpc": "2.0", "id": id, "error": { "code": INTERNAL_ERROR, "message": message } })
}

struct Bridge {
  port: u16,
  /// The legacy protocol version `initialize` agreed on.
  negotiated: Mutex<Option<String>>,
  /// Requests in flight by JSON-RPC id.
  flights: Mutex<HashMap<String, Flight>>,
}

/// A request in flight: its connection once open, and whether the client
/// cancelled it (then it gets no answer).
#[derive(Default)]
struct Flight {
  cancelled: bool,
  stream: Option<TcpStream>,
}

impl Bridge {
  fn new(port: u16) -> Self {
    Self {
      port,
      negotiated: Mutex::new(None),
      flights: Mutex::new(HashMap::new()),
    }
  }

  fn is_cancelled(&self, key: &str) -> bool {
    self
      .flights
      .lock()
      .unwrap()
      .get(key)
      .is_some_and(|flight| flight.cancelled)
  }

  /// `notifications/cancelled` for a request in flight: closing its
  /// connection is how Streamable HTTP cancels, and the server stops the call.
  fn cancel(&self, key: &str) {
    if let Some(flight) = self.flights.lock().unwrap().get_mut(key) {
      flight.cancelled = true;
      if let Some(stream) = flight.stream.take() {
        let _ = stream.shutdown(Shutdown::Both);
      }
    }
  }

  /// Keeps a handle to the request's connection, or closes it when the
  /// request was cancelled meanwhile.
  fn opened(&self, key: &str, stream: &TcpStream) {
    let Ok(handle) = stream.try_clone() else {
      return;
    };
    if let Some(flight) = self.flights.lock().unwrap().get_mut(key) {
      if flight.cancelled {
        let _ = handle.shutdown(Shutdown::Both);
      } else {
        flight.stream = Some(handle);
      }
    }
  }

  /// Posts request `key` once. Nothing listening is an error: Angaria is
  /// closed, and the bridge never opens it.
  fn send(
    &self,
    key: &str,
    headers: &[(&'static str, String)],
    body: &[u8],
  ) -> Result<(u16, Vec<u8>), &'static str> {
    if self.is_cancelled(key) {
      return Err(NOT_REACHABLE);
    }
    let result = post(self.port, headers, body, |stream| self.opened(key, stream));
    if let Some(flight) = self.flights.lock().unwrap().get_mut(key) {
      flight.stream = None;
    }
    result.map_err(|error| match error {
      PostError::Connect => NOT_REACHABLE,
      PostError::Lost => CONNECTION_LOST,
    })
  }

  /// The answer to write back for `message`, none for a notification or a
  /// cancelled request.
  fn forward(&self, message: &Value) -> Option<Value> {
    let negotiated = self.negotiated.lock().unwrap().clone();
    let headers = mcp_headers(message, negotiated.as_deref());
    let Some(id) = message.get("id").cloned().filter(|_| message.get("method").is_some()) else {
      if message["method"] == "notifications/cancelled" {
        if let Some(request) = message["params"].get("requestId") {
          self.cancel(&request.to_string());
        }
        return None;
      }
      // A notification needs no answer: with Angaria closed it is dropped.
      let _ = post(self.port, &headers, message.to_string().as_bytes(), |_| {});
      return None;
    };
    let key = id.to_string();
    self.flights.lock().unwrap().insert(key.clone(), Flight::default());
    let outcome = self.send(&key, &headers, message.to_string().as_bytes());
    let flight = self.flights.lock().unwrap().remove(&key);
    if flight.is_some_and(|flight| flight.cancelled) {
      return None;
    }
    let (status, body) = match outcome {
      Ok(response) => response,
      Err(text) => return Some(error_reply(id, text)),
    };
    let reply = serde_json::from_slice::<Value>(&body)
      .ok()
      .filter(|reply| reply.get("result").is_some() || reply.get("error").is_some());
    let Some(mut reply) = reply else {
      return Some(error_reply(
        id,
        &format!("Angaria beklenmeyen bir yanıt verdi (HTTP {status})."),
      ));
    };
    if message["method"] == "initialize" {
      if let Some(version) = reply["result"]["protocolVersion"].as_str() {
        *self.negotiated.lock().unwrap() = Some(version.to_string());
      }
    }
    // A refusal before the server read the body names no id.
    if reply.get("id").map_or(true, Value::is_null) {
      reply["id"] = id;
    }
    Some(reply)
  }
}

fn write_line(out: &Mutex<io::Stdout>, value: &Value) {
  let mut out = out.lock().unwrap();
  let _ = writeln!(out, "{value}");
  let _ = out.flush();
}

fn main() {
  let bridge = Arc::new(Bridge::new(port()));
  let out = Arc::new(Mutex::new(io::stdout()));
  let mut running = Vec::new();
  for line in io::stdin().lock().lines() {
    let Ok(line) = line else {
      break;
    };
    if line.trim().is_empty() {
      continue;
    }
    let message = match serde_json::from_str::<Value>(&line) {
      Ok(message) if message.is_object() => message,
      _ => {
        let reply = json!({ "jsonrpc": "2.0", "id": null, "error": { "code": -32700, "message": "Parse error" } });
        write_line(&out, &reply);
        continue;
      }
    };
    // initialize settles the version later messages send, and notifications
    // keep their order; other requests run side by side, so a slow GİB call
    // does not hold up a ping.
    let inline = message["method"] == "initialize" || message.get("id").is_none();
    if inline {
      if let Some(reply) = bridge.forward(&message) {
        write_line(&out, &reply);
      }
    } else {
      let bridge = bridge.clone();
      let out = out.clone();
      running.retain(|handle: &thread::JoinHandle<()>| !handle.is_finished());
      running.push(thread::spawn(move || {
        if let Some(reply) = bridge.forward(&message) {
          write_line(&out, &reply);
        }
      }));
    }
  }
  // stdin closed: answer what is still in flight before exiting.
  for handle in running {
    let _ = handle.join();
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::time::Instant;

  #[test]
  fn modern_requests_mirror_their_metadata() {
    let message = json!({
      "jsonrpc": "2.0", "id": 1, "method": "tools/call",
      "params": { "name": "list_invoices", "_meta": { META_VERSION: "2026-07-28" } },
    });
    assert_eq!(
      mcp_headers(&message, Some("2025-06-18")),
      vec![
        ("MCP-Protocol-Version", "2026-07-28".to_string()),
        ("Mcp-Method", "tools/call".to_string()),
        ("Mcp-Name", "list_invoices".to_string()),
      ]
    );
  }

  #[test]
  fn legacy_requests_send_the_negotiated_version() {
    let message = json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/list" });
    assert_eq!(mcp_headers(&message, None), vec![]);
    assert_eq!(
      mcp_headers(&message, Some("2025-06-18")),
      vec![("MCP-Protocol-Version", "2025-06-18".to_string())]
    );
  }

  #[test]
  fn header_values_outside_ascii_are_base64() {
    assert_eq!(header_safe("list_invoices"), "list_invoices");
    assert_eq!(header_safe("Hello, 世界"), "=?base64?SGVsbG8sIOS4lueVjA==?=");
    assert_eq!(header_safe(" padded "), "=?base64?IHBhZGRlZCA=?=");
    assert_eq!(
      header_safe("=?base64?literal?="),
      "=?base64?PT9iYXNlNjQ/bGl0ZXJhbD89?="
    );
  }

  #[test]
  fn reads_responses_with_and_without_chunks() {
    let plain = b"HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: 2\r\n\r\n{}";
    assert_eq!(parse_response(plain), Some((200, b"{}".to_vec())));
    let chunked = b"HTTP/1.1 400 Bad Request\r\nTransfer-Encoding: chunked\r\n\r\n3\r\n{\"a\r\n4\r\n\":1}\r\n0\r\n\r\n";
    assert_eq!(parse_response(chunked), Some((400, b"{\"a\":1}".to_vec())));
    assert_eq!(
      parse_response(b"HTTP/1.1 202 Accepted\r\n\r\n"),
      Some((202, vec![]))
    );
    assert_eq!(parse_response(b"garbage"), None);
  }

  /// A port nothing listens on: Angaria closed.
  fn closed_port() -> u16 {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.local_addr().unwrap().port()
  }

  #[test]
  fn a_cancelled_request_closes_its_connection_and_gets_no_answer() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    // A server that reads the request, then waits for the client to hang up.
    let server = thread::spawn(move || {
      let (mut stream, _) = listener.accept().unwrap();
      let mut seen = Vec::new();
      let mut buffer = [0u8; 1024];
      while let Ok(read) = stream.read(&mut buffer) {
        if read == 0 {
          break;
        }
        seen.extend_from_slice(&buffer[..read]);
      }
      String::from_utf8(seen).unwrap()
    });
    let bridge = Arc::new(Bridge::new(port));
    let request = json!({ "jsonrpc": "2.0", "id": 5, "method": "tools/call", "params": { "name": "x" } });
    let worker = {
      let bridge = bridge.clone();
      thread::spawn(move || bridge.forward(&request))
    };
    while !bridge
      .flights
      .lock()
      .unwrap()
      .get("5")
      .is_some_and(|flight| flight.stream.is_some())
    {
      thread::sleep(Duration::from_millis(10));
    }
    let note = json!({ "jsonrpc": "2.0", "method": "notifications/cancelled", "params": { "requestId": 5 } });
    assert!(bridge.forward(&note).is_none());
    assert!(worker.join().unwrap().is_none());
    assert!(server.join().unwrap().contains("tools/call"));
    assert!(bridge.flights.lock().unwrap().is_empty());
    // A late cancellation of a finished request marks nothing.
    bridge.cancel("5");
    assert!(bridge.flights.lock().unwrap().is_empty());
  }

  #[test]
  fn a_closed_app_is_a_turkish_error_for_requests_only() {
    let bridge = Bridge::new(closed_port());
    let started = Instant::now();
    let request = json!({ "jsonrpc": "2.0", "id": 7, "method": "tools/list" });
    let reply = bridge.forward(&request).unwrap();
    assert_eq!(reply["id"], 7);
    assert_eq!(reply["error"]["message"], NOT_REACHABLE);
    assert_eq!(reply["error"]["message"], "Angaria’ya bağlanılamadı. Angaria’yı aç.");
    let note = json!({ "jsonrpc": "2.0", "method": "notifications/initialized" });
    assert!(bridge.forward(&note).is_none());
    // No wait for an Angaria to start: it answers at once.
    assert!(started.elapsed() < Duration::from_secs(1));
  }

  /// A server that answers one request with `reply` and returns what it read.
  fn answer_once(reply: Value) -> (u16, thread::JoinHandle<String>) {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = thread::spawn(move || {
      let (mut stream, _) = listener.accept().unwrap();
      let mut seen = Vec::new();
      let mut buffer = [0u8; 4096];
      loop {
        let read = stream.read(&mut buffer).unwrap();
        seen.extend_from_slice(&buffer[..read]);
        let Some(head_end) = find(&seen, b"\r\n\r\n") else {
          continue;
        };
        let head = String::from_utf8_lossy(&seen[..head_end]).to_lowercase();
        let length: usize = head
          .lines()
          .find_map(|line| line.strip_prefix("content-length:"))
          .map_or(0, |value| value.trim().parse().unwrap());
        if read == 0 || seen.len() >= head_end + 4 + length {
          break;
        }
      }
      let body = reply.to_string();
      write!(
        stream,
        "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{body}",
        body.len()
      )
      .unwrap();
      String::from_utf8(seen).unwrap()
    });
    (port, server)
  }

  #[test]
  fn forwards_to_the_fixed_port_without_a_token() {
    let reply = json!({ "jsonrpc": "2.0", "id": 9, "result": { "tools": [] } });
    let (port, server) = answer_once(reply.clone());
    let bridge = Bridge::new(port);
    let request = json!({ "jsonrpc": "2.0", "id": 9, "method": "tools/list" });
    assert_eq!(bridge.forward(&request), Some(reply));
    let seen = server.join().unwrap();
    assert!(seen.starts_with(&format!("POST /mcp HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n")));
    assert!(!seen.to_lowercase().contains("authorization"));
  }

  #[test]
  fn the_port_is_the_apps_unless_a_test_moves_it() {
    if env::var_os(PORT_ENV).is_none() {
      assert_eq!(port(), MCP_PORT);
    }
  }
}
