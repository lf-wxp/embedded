//! 极简静态文件服务器（仅依赖 std）
//!
//! 用途：为浏览器端的 Web Bluetooth 演示页面 `web/index.html` 提供 localhost 服务。
//! Web Bluetooth API 必须在 HTTPS 或 `localhost` 下才能使用，因此不能直接 `file://` 打开。
//!
//! 使用：
//! ```bash
//! cargo run --release        # 默认 http://127.0.0.1:8080
//! PORT=9000 cargo run        # 自定义端口
//! WEB_ROOT=../web cargo run  # 自定义静态目录
//! ```

use std::env;
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Component, Path, PathBuf};
use std::thread;

const DEFAULT_PORT: u16 = 8080;

fn main() {
  let port: u16 = env::var("PORT")
    .ok()
    .and_then(|s| s.parse().ok())
    .unwrap_or(DEFAULT_PORT);

  let web_root = match env::var("WEB_ROOT") {
    Ok(s) => {
      // WEB_ROOT 可以是相对或绝对路径
      match fs::canonicalize(&s) {
        Ok(p) => p,
        Err(e) => {
          eprintln!("❌ 无法解析 WEB_ROOT={:?}: {}", s, e);
          eprintln!("   请确认该目录存在");
          std::process::exit(1);
        }
      }
    }
    Err(_) => match find_web_root() {
      Some(p) => p,
      None => {
        eprintln!("❌ 无法自动定位 web/ 目录");
        eprintln!("   请在仓库根目录运行，或设置环境变量 WEB_ROOT=/path/to/web");
        std::process::exit(1);
      }
    },
  };

  let addr = format!("127.0.0.1:{}", port);
  let listener = TcpListener::bind(&addr).unwrap_or_else(|e| {
    eprintln!("❌ 无法绑定 {}: {}", addr, e);
    std::process::exit(1);
  });

  println!("🚀 Web Bluetooth 演示服务器已启动");
  println!("   静态目录: {}", web_root.display());
  println!("   访问地址: http://{}/", addr);
  println!("   按 Ctrl+C 退出");

  for stream in listener.incoming() {
    match stream {
      Ok(stream) => {
        let root = web_root.clone();
        thread::spawn(move || {
          if let Err(e) = handle_connection(stream, &root) {
            eprintln!("⚠️  请求处理失败: {}", e);
          }
        });
      }
      Err(e) => eprintln!("⚠️  接受连接失败: {}", e),
    }
  }
}

/// 智能查找 web/ 目录：
/// 1. 从可执行文件所在目录向上遍历，找到包含 Cargo.toml 的仓库根目录
/// 2. 检查该根目录下是否存在 web/ 子目录
/// 3. 存在则返回其绝对路径
fn find_web_root() -> Option<PathBuf> {
  let exe = env::current_exe().ok()?;
  let exe_dir = exe.parent()?;
  let mut cur = exe_dir;

  loop {
    // 找到仓库根目录（包含 Cargo.toml）
    if cur.join("Cargo.toml").exists() {
      let web = cur.join("web");
      if web.is_dir() {
        return fs::canonicalize(&web).ok();
      }
    }
    match cur.parent() {
      Some(p) => cur = p,
      None => break,
    }
  }
  None
}

fn handle_connection(mut stream: TcpStream, web_root: &Path) -> std::io::Result<()> {
  let peer = stream.peer_addr().ok();
  let mut reader = BufReader::new(stream.try_clone()?);

  // 读取请求行
  let mut request_line = String::new();
  reader.read_line(&mut request_line)?;
  let request_line = request_line.trim_end();

  // 解析 "METHOD PATH HTTP/1.1"
  let mut parts = request_line.split_whitespace();
  let method = parts.next().unwrap_or("");
  let raw_path = parts.next().unwrap_or("/");

  // 消费完剩余 headers（不解析 body，本服务器只接受 GET）
  loop {
    let mut line = String::new();
    let n = reader.read_line(&mut line)?;
    if n == 0 || line == "\r\n" || line == "\n" {
      break;
    }
  }

  if method != "GET" && method != "HEAD" {
    write_response(&mut stream, 405, "Method Not Allowed", "text/plain", b"405")?;
    return Ok(());
  }

  // 去除 query 字符串
  let path_only = raw_path.split('?').next().unwrap_or("/");
  let decoded = url_decode(path_only);

  // 安全：解析为相对路径并阻止穿越
  let rel = match safe_join(web_root, &decoded) {
    Some(p) => p,
    None => {
      write_response(&mut stream, 403, "Forbidden", "text/plain", b"403")?;
      return Ok(());
    }
  };

  // 默认文件
  let target = if rel.is_dir() {
    rel.join("index.html")
  } else {
    rel
  };

  match fs::read(&target) {
    Ok(body) => {
      let ctype = content_type(&target);
      println!(
        "[{}] GET {} -> 200 ({} bytes, {})",
        peer
          .map(|a| a.to_string())
          .unwrap_or_else(|| "?".to_string()),
        raw_path,
        body.len(),
        ctype
      );
      write_response(&mut stream, 200, "OK", ctype, &body)?;
    }
    Err(_) => {
      println!(
        "[{}] GET {} -> 404",
        peer
          .map(|a| a.to_string())
          .unwrap_or_else(|| "?".to_string()),
        raw_path
      );
      write_response(&mut stream, 404, "Not Found", "text/plain", b"404 Not Found")?;
    }
  }
  Ok(())
}

/// 安全地把 URL 路径拼接到 web_root，禁止 `..` 穿越
fn safe_join(web_root: &Path, url_path: &str) -> Option<PathBuf> {
  let trimmed = url_path.trim_start_matches('/');
  let mut p = web_root.to_path_buf();
  for comp in Path::new(trimmed).components() {
    match comp {
      Component::Normal(s) => p.push(s),
      Component::CurDir => {}
      // 拒绝任何向上、根、前缀
      _ => return None,
    }
  }
  Some(p)
}

fn write_response(
  stream: &mut TcpStream,
  status: u16,
  reason: &str,
  content_type: &str,
  body: &[u8],
) -> std::io::Result<()> {
  let header = format!(
    "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n",
    status,
    reason,
    content_type,
    body.len()
  );
  stream.write_all(header.as_bytes())?;
  stream.write_all(body)?;
  stream.flush()?;
  // 读完客户端剩余字节，避免 RST
  let _ = stream.read(&mut [0u8; 64]);
  Ok(())
}

fn content_type(path: &Path) -> &'static str {
  match path.extension().and_then(|s| s.to_str()) {
    Some("html") | Some("htm") => "text/html; charset=utf-8",
    Some("js") | Some("mjs") => "application/javascript; charset=utf-8",
    Some("css") => "text/css; charset=utf-8",
    Some("json") => "application/json; charset=utf-8",
    Some("svg") => "image/svg+xml",
    Some("png") => "image/png",
    Some("jpg") | Some("jpeg") => "image/jpeg",
    Some("ico") => "image/x-icon",
    _ => "application/octet-stream",
  }
}

/// URL 解码（仅处理 %XX，足以应对静态资源路径）
fn url_decode(s: &str) -> String {
  let bytes = s.as_bytes();
  let mut out = Vec::with_capacity(bytes.len());
  let mut i = 0;
  while i < bytes.len() {
    if bytes[i] == b'%' && i + 2 < bytes.len() {
      if let (Some(h), Some(l)) = (hex(bytes[i + 1]), hex(bytes[i + 2])) {
        out.push((h << 4) | l);
        i += 3;
        continue;
      }
    }
    out.push(bytes[i]);
    i += 1;
  }
  String::from_utf8_lossy(&out).into_owned()
}

fn hex(b: u8) -> Option<u8> {
  match b {
    b'0'..=b'9' => Some(b - b'0'),
    b'a'..=b'f' => Some(b - b'a' + 10),
    b'A'..=b'F' => Some(b - b'A' + 10),
    _ => None,
  }
}
