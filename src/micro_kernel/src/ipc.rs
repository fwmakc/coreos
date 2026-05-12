//! Inter-process communication bridge.

/// IPC message envelope.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IpcMessage {
    /// Sender capability token.
    pub from: String,
    /// Recipient capability token.
    pub to: String,
    /// Payload bytes.
    pub payload: Vec<u8>,
}

/// IPC bridge interface.
pub trait IpcBridge {
    /// Send a message.
    fn send(&self, msg: IpcMessage) -> Result<(), IpcError>;
    /// Receive the next message.
    fn recv(&self) -> Result<IpcMessage, IpcError>;
}

/// IPC errors.
#[derive(Debug)]
pub enum IpcError {
    /// Connection lost.
    Disconnected,
    /// Permission denied.
    PermissionDenied,
    /// Serialization failure.
    Serialize(String),
}

impl std::fmt::Display for IpcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IpcError::Disconnected => write!(f, "IPC connection lost"),
            IpcError::PermissionDenied => write!(f, "IPC permission denied"),
            IpcError::Serialize(e) => write!(f, "IPC serialization error: {e}"),
        }
    }
}

impl std::error::Error for IpcError {}
