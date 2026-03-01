use crate::rpc::{JsonRpcRequest, JsonRpcResponse};
use serde::Deserialize;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::time::Duration;
use thiserror::Error;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::time::timeout;

const CONNECT_TIMEOUT: Duration = Duration::from_millis(1200);
const RESPONSE_TIMEOUT: Duration = Duration::from_secs(20);
const IPC_FILENAME: &str = "daemon-ipc.json";

#[derive(Debug, Error)]
pub enum DaemonClientError {
    #[error("daemon IPC is not available")]
    NotAvailable,
    #[error("daemon IPC protocol error: {0}")]
    Protocol(String),
    #[error("daemon returned RPC error: {0}")]
    Rpc(String),
}

#[derive(Debug, Clone, Deserialize)]
struct DaemonIpcInfo {
    config_path: String,
    address: String,
}

pub async fn request_via_daemon(
    config_path: &Path,
    method: &str,
    params: Option<Value>,
) -> Result<Value, DaemonClientError> {
    let resolved_config = resolve_config_path(config_path);
    let info_path = daemon_info_path(&resolved_config);
    if !info_path.exists() {
        return Err(DaemonClientError::NotAvailable);
    }

    let info = read_daemon_info(&info_path)?;
    if info.config_path != config_identity(&resolved_config) {
        return Err(DaemonClientError::NotAvailable);
    }

    let stream = match timeout(CONNECT_TIMEOUT, TcpStream::connect(&info.address)).await {
        Ok(Ok(stream)) => stream,
        _ => {
            let _ = std::fs::remove_file(&info_path);
            return Err(DaemonClientError::NotAvailable);
        }
    };

    let request = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: 1,
        method: method.to_string(),
        params,
    };
    let payload = serde_json::to_string(&request)
        .map_err(|e| DaemonClientError::Protocol(format!("failed to encode request: {}", e)))?;

    let (read_half, mut write_half) = stream.into_split();
    write_half
        .write_all(payload.as_bytes())
        .await
        .map_err(|e| DaemonClientError::Protocol(format!("failed to write request: {}", e)))?;
    write_half.write_all(b"\n").await.map_err(|e| {
        DaemonClientError::Protocol(format!("failed to write request newline: {}", e))
    })?;
    write_half
        .flush()
        .await
        .map_err(|e| DaemonClientError::Protocol(format!("failed to flush request: {}", e)))?;

    let mut reader = BufReader::new(read_half);
    let mut line = String::new();
    let bytes_read = timeout(RESPONSE_TIMEOUT, reader.read_line(&mut line))
        .await
        .map_err(|_| {
            DaemonClientError::Protocol("timed out waiting for daemon response".to_string())
        })?
        .map_err(|e| DaemonClientError::Protocol(format!("failed to read response: {}", e)))?;

    if bytes_read == 0 {
        return Err(DaemonClientError::Protocol(
            "daemon closed connection without a response".to_string(),
        ));
    }

    let response: JsonRpcResponse = serde_json::from_str(line.trim()).map_err(|e| {
        DaemonClientError::Protocol(format!("failed to decode daemon response: {}", e))
    })?;

    if let Some(error) = response.error {
        return Err(DaemonClientError::Rpc(error.message));
    }

    Ok(response.result.unwrap_or(Value::Null))
}

fn read_daemon_info(info_path: &Path) -> Result<DaemonIpcInfo, DaemonClientError> {
    let contents =
        std::fs::read_to_string(info_path).map_err(|_| DaemonClientError::NotAvailable)?;
    serde_json::from_str::<DaemonIpcInfo>(&contents)
        .map_err(|e| DaemonClientError::Protocol(format!("invalid daemon IPC file: {}", e)))
}

fn daemon_info_path(config_path: &Path) -> PathBuf {
    let config_dir = config_path.parent().unwrap_or_else(|| Path::new("."));
    config_dir.join(".dmn").join(IPC_FILENAME)
}

fn resolve_config_path(config_path: &Path) -> PathBuf {
    if config_path.is_absolute() {
        config_path.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(config_path)
    }
}

fn config_identity(config_path: &Path) -> String {
    std::fs::canonicalize(config_path)
        .unwrap_or_else(|_| config_path.to_path_buf())
        .to_string_lossy()
        .to_string()
}
