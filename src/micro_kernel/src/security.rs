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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rights_read_contains_read() {
        assert!(Rights::READ.contains(Rights::READ));
    }

    #[test]
    fn rights_read_write_contains_read() {
        let rw = Rights(Rights::READ.0 | Rights::WRITE.0);
        assert!(rw.contains(Rights::READ));
        assert!(rw.contains(Rights::WRITE));
        assert!(!rw.contains(Rights::EXECUTE));
    }

    #[test]
    fn rights_full_contains_all() {
        let full = Rights(Rights::READ.0 | Rights::WRITE.0 | Rights::EXECUTE.0 | Rights::ADMIN.0);
        assert!(full.contains(Rights::READ));
        assert!(full.contains(Rights::WRITE));
        assert!(full.contains(Rights::EXECUTE));
        assert!(full.contains(Rights::ADMIN));
    }

    #[test]
    fn rights_none_contains_nothing() {
        let none = Rights(0);
        assert!(!none.contains(Rights::READ));
        assert!(!none.contains(Rights::WRITE));
    }

    #[test]
    fn capability_equality() {
        let cap1 = Capability {
            resource: "fs:/tmp".into(),
            rights: Rights::READ,
        };
        let cap2 = Capability {
            resource: "fs:/tmp".into(),
            rights: Rights::READ,
        };
        assert_eq!(cap1, cap2);
    }

    #[test]
    fn capability_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(Capability {
            resource: "net:*".into(),
            rights: Rights::WRITE,
        });
        set.insert(Capability {
            resource: "net:*".into(),
            rights: Rights::WRITE,
        });
        assert_eq!(set.len(), 1);
    }

    #[test]
    fn capability_inequality() {
        let cap1 = Capability {
            resource: "fs:/tmp".into(),
            rights: Rights::READ,
        };
        let cap2 = Capability {
            resource: "fs:/tmp".into(),
            rights: Rights::WRITE,
        };
        assert_ne!(cap1, cap2);
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn rights_contains_reflexive(bits in 0u8..=15u8) {
            let r = Rights(bits);
            prop_assert!(r.contains(r));
        }

        #[test]
        fn rights_contains_transitive(a in 0u8..=15u8, b in 0u8..=15u8) {
            let ra = Rights(a);
            let rb = Rights(b);
            if ra.contains(rb) && rb.contains(ra) {
                prop_assert_eq!(a, b);
            }
        }

        #[test]
        fn rights_union_contains_each(a in 0u8..=15u8, b in 0u8..=15u8) {
            let union = Rights(a | b);
            prop_assert!(union.contains(Rights(a)));
            prop_assert!(union.contains(Rights(b)));
        }

        #[test]
        fn capability_resource_equality(resource in "[a-z:/]{1,20}", rights_bits in 0u8..=15u8) {
            let cap1 = Capability { resource: resource.clone(), rights: Rights(rights_bits) };
            let cap2 = Capability { resource, rights: Rights(rights_bits) };
            prop_assert_eq!(cap1, cap2);
        }
    }
}
