//! Capability-based security primitives.

/// A capability token grants a specific right.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Capability {
    /// Resource identifier.
    pub resource: String,
    /// Granted rights (read, write, execute, admin).
    pub rights: Rights,
}

/// Rights bitmask.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rights(pub u8);

impl Rights {
    /// Read permission.
    pub const READ: Self = Self(0b0001);
    /// Write permission.
    pub const WRITE: Self = Self(0b0010);
    /// Execute permission.
    pub const EXECUTE: Self = Self(0b0100);
    /// Admin permission.
    pub const ADMIN: Self = Self(0b1000);

    /// Check if this rights set contains `other`.
    pub fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }
}
