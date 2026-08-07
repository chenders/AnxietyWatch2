use std::path::Path;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;
use tokio::time::timeout;
use vitals_proto::{Request, Response, check_frame_len, decode_payload, encode_frame};

#[derive(Debug)]
pub enum ClientError {
    ConnectFailed(String),
    Timeout,
    Io(String),
    Protocol(String),
    UnexpectedResponse,
}

impl std::fmt::Display for ClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ConnectFailed(e) => write!(f, "connect failed: {e}"),
            Self::Timeout => write!(f, "timeout waiting for daemon"),
            Self::Io(e) => write!(f, "io error: {e}"),
            Self::Protocol(e) => write!(f, "protocol error: {e}"),
            Self::UnexpectedResponse => write!(f, "unexpected response type"),
        }
    }
}

pub async fn get_health(socket_path: &Path) -> Result<Box<vitals_proto::Health>, ClientError> {
    let req = Request::Health;
    let req_bytes = encode_frame(&req).map_err(|e| ClientError::Protocol(e.to_string()))?;

    let future = async {
        let mut stream = UnixStream::connect(socket_path)
            .await
            .map_err(|e| ClientError::ConnectFailed(e.to_string()))?;

        stream
            .write_all(&req_bytes)
            .await
            .map_err(|e| ClientError::Io(e.to_string()))?;

        let mut len_buf = [0u8; 4];
        stream
            .read_exact(&mut len_buf)
            .await
            .map_err(|e| ClientError::Io(e.to_string()))?;

        let len = check_frame_len(u32::from_le_bytes(len_buf))
            .map_err(|e| ClientError::Protocol(e.to_string()))?;

        let mut payload = vec![0u8; len];
        stream
            .read_exact(&mut payload)
            .await
            .map_err(|e| ClientError::Io(e.to_string()))?;

        let resp: Response =
            decode_payload(&payload).map_err(|e| ClientError::Protocol(e.to_string()))?;

        match resp {
            Response::Health { health } => Ok(health),
            _ => Err(ClientError::UnexpectedResponse),
        }
    };

    timeout(Duration::from_secs(10), future)
        .await
        .map_err(|_| ClientError::Timeout)?
}
