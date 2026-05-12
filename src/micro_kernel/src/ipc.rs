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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ipc_message_serialization_roundtrip() {
        let msg = IpcMessage {
            from: "user-space".into(),
            to: "kernel".into(),
            payload: vec![0xDE, 0xAD, 0xBE, 0xEF],
        };
        let json = serde_json::to_string(&msg).unwrap();
        let decoded: IpcMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.from, "user-space");
        assert_eq!(decoded.to, "kernel");
        assert_eq!(decoded.payload, vec![0xDE, 0xAD, 0xBE, 0xEF]);
    }

    #[test]
    fn ipc_message_empty_payload() {
        let msg = IpcMessage {
            from: "a".into(),
            to: "b".into(),
            payload: vec![],
        };
        let json = serde_json::to_string(&msg).unwrap();
        let decoded: IpcMessage = serde_json::from_str(&json).unwrap();
        assert!(decoded.payload.is_empty());
    }

    #[test]
    fn ipc_error_display_disconnected() {
        let err = IpcError::Disconnected;
        assert_eq!(format!("{err}"), "IPC connection lost");
    }

    #[test]
    fn ipc_error_display_permission_denied() {
        let err = IpcError::PermissionDenied;
        assert_eq!(format!("{err}"), "IPC permission denied");
    }

    #[test]
    fn ipc_error_display_serialize() {
        let err = IpcError::Serialize("bad utf8".into());
        let msg = format!("{err}");
        assert!(msg.contains("IPC serialization error"));
        assert!(msg.contains("bad utf8"));
    }

    #[test]
    fn ipc_error_implements_std_error() {
        let err: Box<dyn std::error::Error> = Box::new(IpcError::Disconnected);
        assert_eq!(err.to_string(), "IPC connection lost");
    }
}
