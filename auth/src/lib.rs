use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::collections::HashMap;
use thiserror::Error;
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Role {
    Viewer,
    Editor,
    Owner,
}

impl Role {
    pub fn can_write(&self) -> bool {
        matches!(self, Role::Editor | Role::Owner)
    }

    pub fn can_admin(&self) -> bool {
        matches!(self, Role::Owner)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub org_id: String,
    pub role: Role,
    pub exp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtHeader {
    pub alg: String,
    pub typ: String,
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

    #[error("token expired")]
    TokenExpired,

    #[error("invalid token: {0}")]
    InvalidToken(String),

    #[error("insufficient permissions: need {needed:?}, have {have:?}")]
    InsufficientPermissions { needed: Role, have: Role },

    #[error("document access denied")]
    DocumentAccessDenied,
}

pub struct AuthChecker {
    secret: Vec<u8>,
    permissions: HashMap<(Uuid, String), Permission>,
}

impl AuthChecker {
    pub fn new(secret: &[u8]) -> Self {
        Self {
            secret: secret.to_vec(),
            permissions: HashMap::new(),
        }
    }

    pub fn create_token(&self, user_id: &Uuid, org_id: &str, role: Role, exp: u64) -> Result<String, AuthError> {
        let claims = Claims {
            sub: user_id.to_string(),
            org_id: org_id.to_string(),
            role,
            exp,
        };

        let header = JwtHeader {
            alg: "HS256".to_string(),
            typ: "JWT".to_string(),
        };

        let header_b64 = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&header).unwrap());
        let payload_b64 = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&claims).unwrap());
        let signing_input = format!("{}.{}", header_b64, payload_b64);

        let mut mac = HmacSha256::new_from_slice(&self.secret)
            .map_err(|e| AuthError::InvalidToken(e.to_string()))?;
        mac.update(signing_input.as_bytes());
        let signature = URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes());

        Ok(format!("{}.{}", signing_input, signature))
    }

    pub fn verify_token(&self, token: &str) -> Result<Claims, AuthError> {
        let parts: Vec<&str> = token.split('.').collect();
        if parts.len() != 3 {
            return Err(AuthError::InvalidToken("invalid format".into()));
        }

        let signing_input = format!("{}.{}", parts[0], parts[1]);
        let signature_bytes = URL_SAFE_NO_PAD
            .decode(parts[2])
            .map_err(|e| AuthError::InvalidToken(e.to_string()))?;

        let mut mac = HmacSha256::new_from_slice(&self.secret)
            .map_err(|e| AuthError::InvalidToken(e.to_string()))?;
        mac.update(signing_input.as_bytes());
        mac.verify_slice(&signature_bytes)
            .map_err(|_| AuthError::InvalidToken("invalid signature".into()))?;

        let payload_bytes = URL_SAFE_NO_PAD
            .decode(parts[1])
            .map_err(|e| AuthError::InvalidToken(e.to_string()))?;
        let claims: Claims = serde_json::from_slice(&payload_bytes)
            .map_err(|e| AuthError::InvalidToken(e.to_string()))?;

        Ok(claims)
    }

    pub fn set_permission(&mut self, permission: Permission) {
        self.permissions
            .insert((permission.user_id, permission.org_id.clone()), permission);
    }

    pub fn get_permission(&self, user_id: &Uuid, org_id: &str) -> Option<&Permission> {
        self.permissions.get(&(*user_id, org_id.to_string()))
    }

    pub fn can_write(&self, user_id: &Uuid, org_id: &str) -> Result<(), AuthError> {
        self.get_permission(user_id, org_id)
            .ok_or(AuthError::DocumentAccessDenied)
            .and_then(|p| {
                if p.role.can_write() {
                    Ok(())
                } else {
                    Err(AuthError::InsufficientPermissions {
                        needed: Role::Editor,
                        have: p.role,
                    })
                }
            })
    }

    pub fn can_admin(&self, user_id: &Uuid, org_id: &str) -> Result<(), AuthError> {
        self.get_permission(user_id, org_id)
            .ok_or(AuthError::DocumentAccessDenied)
            .and_then(|p| {
                if p.role.can_admin() {
                    Ok(())
                } else {
                    Err(AuthError::InsufficientPermissions {
                        needed: Role::Owner,
                        have: p.role,
                    })
                }
            })
    }

    pub fn check_token_permission(&self, token: &str, org_id: &str, require_write: bool) -> Result<Claims, AuthError> {
        let claims = self.verify_token(token)?;

        if claims.org_id != org_id {
            return Err(AuthError::DocumentAccessDenied);
        }

        if require_write && !claims.role.can_write() {
            return Err(AuthError::InsufficientPermissions {
                needed: Role::Editor,
                have: claims.role,
            });
        }

        Ok(claims)
    }

    pub fn revoke_permission(&mut self, user_id: &Uuid, org_id: &str) -> Option<Permission> {
        self.permissions.remove(&(*user_id, org_id.to_string()))
    }
}

impl Default for AuthChecker {
    fn default() -> Self {
        Self::new(b"default-secret-change-in-production")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn checker() -> AuthChecker {
        AuthChecker::new(b"test-secret-key-for-hmac")
    }

    #[test]
    fn test_role_can_write() {
        assert!(Role::Editor.can_write());
        assert!(Role::Owner.can_write());
        assert!(!Role::Viewer.can_write());
    }

    #[test]
    fn test_role_can_admin() {
        assert!(Role::Owner.can_admin());
        assert!(!Role::Editor.can_admin());
        assert!(!Role::Viewer.can_admin());
    }

    #[test]
    fn test_jwt_roundtrip() {
        let c = checker();
        let user_id = Uuid::new_v4();
        let token = c.create_token(&user_id, "org1", Role::Editor, u64::MAX).unwrap();
        let claims = c.verify_token(&token).unwrap();

        assert_eq!(claims.sub, user_id.to_string());
        assert_eq!(claims.org_id, "org1");
        assert_eq!(claims.role, Role::Editor);
    }

    #[test]
    fn test_jwt_wrong_secret() {
        let c1 = AuthChecker::new(b"secret1");
        let c2 = AuthChecker::new(b"secret2");
        let user_id = Uuid::new_v4();

        let token = c1.create_token(&user_id, "org1", Role::Editor, u64::MAX).unwrap();
        assert!(c2.verify_token(&token).is_err());
    }

    #[test]
    fn test_jwt_invalid_format() {
        let c = checker();
        assert!(c.verify_token("not.a.jwt").is_err());
        assert!(c.verify_token("onlytwo").is_err());
    }

    #[test]
    fn test_permission_store() {
        let mut c = checker();
        let user_id = Uuid::new_v4();

        c.set_permission(Permission {
            user_id,
            org_id: "org1".into(),
            role: Role::Editor,
        });

        assert!(c.can_write(&user_id, "org1").is_ok());
        assert!(c.can_write(&user_id, "org2").is_err());
    }

    #[test]
    fn test_revoke_permission() {
        let mut c = checker();
        let user_id = Uuid::new_v4();

        c.set_permission(Permission {
            user_id,
            org_id: "org1".into(),
            role: Role::Editor,
        });

        let revoked = c.revoke_permission(&user_id, "org1");
        assert!(revoked.is_some());
        assert!(c.can_write(&user_id, "org1").is_err());
    }

    #[test]
    fn test_check_token_permission() {
        let c = checker();
        let user_id = Uuid::new_v4();

        let viewer_token = c.create_token(&user_id, "org1", Role::Viewer, u64::MAX).unwrap();
        let editor_token = c.create_token(&user_id, "org1", Role::Editor, u64::MAX).unwrap();

        assert!(c.check_token_permission(&viewer_token, "org1", false).is_ok());
        assert!(c.check_token_permission(&viewer_token, "org1", true).is_err());
        assert!(c.check_token_permission(&editor_token, "org1", true).is_ok());

        assert!(c.check_token_permission(&viewer_token, "org2", false).is_err());
    }

    #[test]
    fn test_viewer_cannot_write() {
        let mut c = checker();
        let user_id = Uuid::new_v4();

        c.set_permission(Permission {
            user_id,
            org_id: "org1".into(),
            role: Role::Viewer,
        });

        assert!(c.can_write(&user_id, "org1").is_err());
    }

    #[test]
    fn test_owner_can_admin() {
        let mut c = checker();
        let user_id = Uuid::new_v4();

        c.set_permission(Permission {
            user_id,
            org_id: "org1".into(),
            role: Role::Owner,
        });

        assert!(c.can_admin(&user_id, "org1").is_ok());
    }

    #[test]
    fn test_editor_cannot_admin() {
        let mut c = checker();
        let user_id = Uuid::new_v4();

        c.set_permission(Permission {
            user_id,
            org_id: "org1".into(),
            role: Role::Editor,
        });

        assert!(c.can_admin(&user_id, "org1").is_err());
    }
}
