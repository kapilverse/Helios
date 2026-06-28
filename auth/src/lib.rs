use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Role {
    Viewer,
    Editor,
    Owner,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Permission {
    pub user_id: Uuid,
    pub org_id: String,
    pub role: Role,
}

#[derive(Error, Debug)]
pub enum AuthError {
    #[error("unauthorized: {0}")]
    Unauthorized(String),

    #[error("insufficient permissions: need {needed:?}, have {have:?}")]
    InsufficientPermissions { needed: Role, have: Role },
}

pub struct AuthChecker;

impl AuthChecker {
    pub fn new() -> Self {
        Self
    }

    pub fn can_write(&self, permission: &Permission) -> Result<(), AuthError> {
        match permission.role {
            Role::Editor | Role::Owner => Ok(()),
            Role::Viewer => Err(AuthError::InsufficientPermissions {
                needed: Role::Editor,
                have: Role::Viewer,
            }),
        }
    }

    pub fn can_admin(&self, permission: &Permission) -> Result<(), AuthError> {
        match permission.role {
            Role::Owner => Ok(()),
            _ => Err(AuthError::InsufficientPermissions {
                needed: Role::Owner,
                have: permission.role,
            }),
        }
    }
}

impl Default for AuthChecker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_editor_can_write() {
        let checker = AuthChecker::new();
        let perm = Permission {
            user_id: Uuid::new_v4(),
            org_id: "org1".to_string(),
            role: Role::Editor,
        };
        assert!(checker.can_write(&perm).is_ok());
    }

    #[test]
    fn test_viewer_cannot_write() {
        let checker = AuthChecker::new();
        let perm = Permission {
            user_id: Uuid::new_v4(),
            org_id: "org1".to_string(),
            role: Role::Viewer,
        };
        assert!(checker.can_write(&perm).is_err());
    }

    #[test]
    fn test_only_owner_can_admin() {
        let checker = AuthChecker::new();
        let owner = Permission {
            user_id: Uuid::new_v4(),
            org_id: "org1".to_string(),
            role: Role::Owner,
        };
        let editor = Permission {
            user_id: Uuid::new_v4(),
            org_id: "org1".to_string(),
            role: Role::Editor,
        };
        assert!(checker.can_admin(&owner).is_ok());
        assert!(checker.can_admin(&editor).is_err());
    }
}
