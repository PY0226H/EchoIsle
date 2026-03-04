use crate::{AppError, AppState};
use chat_core::User;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;

const ROLE_OPS_ADMIN: &str = "ops_admin";
const ROLE_OPS_REVIEWER: &str = "ops_reviewer";
const ROLE_OPS_VIEWER: &str = "ops_viewer";

#[derive(Debug, Clone, Copy)]
pub(crate) enum OpsPermission {
    DebateManage,
    JudgeReview,
    JudgeRejudge,
}

#[derive(Debug, Clone, ToSchema, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpsertOpsRoleInput {
    pub role: String,
}

#[derive(Debug, Clone, ToSchema, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpsRoleAssignment {
    pub user_id: u64,
    pub user_email: String,
    pub user_fullname: String,
    pub role: String,
    pub granted_by: u64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, ToSchema, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListOpsRoleAssignmentsOutput {
    pub items: Vec<OpsRoleAssignment>,
}

#[derive(Debug, Clone, ToSchema, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RevokeOpsRoleOutput {
    pub user_id: u64,
    pub removed: bool,
}

#[derive(Debug, Clone, FromRow)]
struct OpsRoleAssignmentRow {
    user_id: i64,
    user_email: String,
    user_fullname: String,
    role: String,
    granted_by: i64,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

fn normalize_ops_role(role: &str) -> Result<String, AppError> {
    let normalized = role.trim().to_lowercase();
    let is_valid = matches!(
        normalized.as_str(),
        ROLE_OPS_ADMIN | ROLE_OPS_REVIEWER | ROLE_OPS_VIEWER
    );
    if !is_valid {
        return Err(AppError::DebateError(
            "invalid role, expect one of: ops_admin, ops_reviewer, ops_viewer".to_string(),
        ));
    }
    Ok(normalized)
}

fn role_grants_permission(role: &str, permission: OpsPermission) -> bool {
    match permission {
        OpsPermission::DebateManage => role == ROLE_OPS_ADMIN,
        OpsPermission::JudgeReview => {
            matches!(role, ROLE_OPS_ADMIN | ROLE_OPS_REVIEWER | ROLE_OPS_VIEWER)
        }
        OpsPermission::JudgeRejudge => matches!(role, ROLE_OPS_ADMIN | ROLE_OPS_REVIEWER),
    }
}

fn map_assignment_row(row: OpsRoleAssignmentRow) -> OpsRoleAssignment {
    OpsRoleAssignment {
        user_id: row.user_id as u64,
        user_email: row.user_email,
        user_fullname: row.user_fullname,
        role: row.role,
        granted_by: row.granted_by as u64,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }
}

impl AppState {
    pub(crate) async fn ensure_ops_permission(
        &self,
        user: &User,
        permission: OpsPermission,
    ) -> Result<(), AppError> {
        let owner_row: Option<(i64,)> =
            sqlx::query_as("SELECT owner_id FROM workspaces WHERE id = $1")
                .bind(user.ws_id)
                .fetch_optional(&self.pool)
                .await?;
        let Some((owner_id,)) = owner_row else {
            return Err(AppError::NotFound(format!("workspace id {}", user.ws_id)));
        };
        if owner_id == user.id {
            return Ok(());
        }

        let role_row: Option<(String,)> = sqlx::query_as(
            r#"
            SELECT role
            FROM workspace_user_roles
            WHERE ws_id = $1 AND user_id = $2
            "#,
        )
        .bind(user.ws_id)
        .bind(user.id)
        .fetch_optional(&self.pool)
        .await?;
        let Some((role,)) = role_row else {
            return Err(AppError::DebateConflict(
                "missing ops role assignment".to_string(),
            ));
        };
        if !role_grants_permission(&role, permission) {
            return Err(AppError::DebateConflict(format!(
                "ops role {} cannot access this operation",
                role
            )));
        }
        Ok(())
    }

    async fn ensure_workspace_owner_for_ops_rbac(&self, user: &User) -> Result<(), AppError> {
        let owner_row: Option<(i64,)> =
            sqlx::query_as("SELECT owner_id FROM workspaces WHERE id = $1")
                .bind(user.ws_id)
                .fetch_optional(&self.pool)
                .await?;
        let Some((owner_id,)) = owner_row else {
            return Err(AppError::NotFound(format!("workspace id {}", user.ws_id)));
        };
        if owner_id != user.id {
            return Err(AppError::DebateConflict(
                "only workspace owner can manage ops roles".to_string(),
            ));
        }
        Ok(())
    }

    pub async fn list_ops_role_assignments_by_owner(
        &self,
        user: &User,
    ) -> Result<ListOpsRoleAssignmentsOutput, AppError> {
        self.ensure_workspace_owner_for_ops_rbac(user).await?;
        let rows: Vec<OpsRoleAssignmentRow> = sqlx::query_as(
            r#"
            SELECT
                r.user_id,
                u.email AS user_email,
                u.fullname AS user_fullname,
                r.role,
                r.granted_by,
                r.created_at,
                r.updated_at
            FROM workspace_user_roles r
            JOIN users u ON u.id = r.user_id
            WHERE r.ws_id = $1
            ORDER BY r.updated_at DESC, r.user_id DESC
            "#,
        )
        .bind(user.ws_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(ListOpsRoleAssignmentsOutput {
            items: rows.into_iter().map(map_assignment_row).collect(),
        })
    }

    pub async fn upsert_ops_role_assignment_by_owner(
        &self,
        user: &User,
        target_user_id: u64,
        input: UpsertOpsRoleInput,
    ) -> Result<OpsRoleAssignment, AppError> {
        self.ensure_workspace_owner_for_ops_rbac(user).await?;
        let role = normalize_ops_role(&input.role)?;

        let target_exists: Option<(i64,)> = sqlx::query_as(
            r#"
            SELECT id
            FROM users
            WHERE id = $1 AND ws_id = $2
            "#,
        )
        .bind(target_user_id as i64)
        .bind(user.ws_id)
        .fetch_optional(&self.pool)
        .await?;
        if target_exists.is_none() {
            return Err(AppError::NotFound(format!("user id {}", target_user_id)));
        }

        let row: OpsRoleAssignmentRow = sqlx::query_as(
            r#"
            INSERT INTO workspace_user_roles(
                ws_id, user_id, role, granted_by, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, NOW(), NOW())
            ON CONFLICT (ws_id, user_id)
            DO UPDATE
            SET role = EXCLUDED.role,
                granted_by = EXCLUDED.granted_by,
                updated_at = NOW()
            RETURNING
                user_id,
                (SELECT email FROM users WHERE id = workspace_user_roles.user_id) AS user_email,
                (SELECT fullname FROM users WHERE id = workspace_user_roles.user_id) AS user_fullname,
                role,
                granted_by,
                created_at,
                updated_at
            "#,
        )
        .bind(user.ws_id)
        .bind(target_user_id as i64)
        .bind(role)
        .bind(user.id)
        .fetch_one(&self.pool)
        .await?;
        Ok(map_assignment_row(row))
    }

    pub async fn revoke_ops_role_assignment_by_owner(
        &self,
        user: &User,
        target_user_id: u64,
    ) -> Result<RevokeOpsRoleOutput, AppError> {
        self.ensure_workspace_owner_for_ops_rbac(user).await?;
        let removed = sqlx::query_scalar::<_, i64>(
            r#"
            DELETE FROM workspace_user_roles
            WHERE ws_id = $1 AND user_id = $2
            RETURNING user_id
            "#,
        )
        .bind(user.ws_id)
        .bind(target_user_id as i64)
        .fetch_optional(&self.pool)
        .await?
        .is_some();

        Ok(RevokeOpsRoleOutput {
            user_id: target_user_id,
            removed,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;

    #[tokio::test]
    async fn ensure_ops_permission_should_allow_owner_and_assigned_role() -> Result<()> {
        let (_tdb, state) = AppState::new_for_test().await?;
        state.update_workspace_owner(1, 1).await?;
        let owner = state.find_user_by_id(1).await?.expect("owner should exist");
        let user = state.find_user_by_id(2).await?.expect("user should exist");

        state
            .ensure_ops_permission(&owner, OpsPermission::DebateManage)
            .await?;

        state
            .upsert_ops_role_assignment_by_owner(
                &owner,
                user.id as u64,
                UpsertOpsRoleInput {
                    role: "ops_admin".to_string(),
                },
            )
            .await?;
        state
            .ensure_ops_permission(&user, OpsPermission::DebateManage)
            .await?;
        state
            .ensure_ops_permission(&user, OpsPermission::JudgeRejudge)
            .await?;
        Ok(())
    }

    #[tokio::test]
    async fn ensure_ops_permission_should_respect_role_matrix() -> Result<()> {
        let (_tdb, state) = AppState::new_for_test().await?;
        state.update_workspace_owner(1, 1).await?;
        let owner = state.find_user_by_id(1).await?.expect("owner should exist");
        let user = state.find_user_by_id(2).await?.expect("user should exist");

        state
            .upsert_ops_role_assignment_by_owner(
                &owner,
                user.id as u64,
                UpsertOpsRoleInput {
                    role: "ops_viewer".to_string(),
                },
            )
            .await?;
        state
            .ensure_ops_permission(&user, OpsPermission::JudgeReview)
            .await?;
        let manage_err = state
            .ensure_ops_permission(&user, OpsPermission::DebateManage)
            .await
            .expect_err("viewer should not manage debate");
        assert!(matches!(manage_err, AppError::DebateConflict(_)));
        let rejudge_err = state
            .ensure_ops_permission(&user, OpsPermission::JudgeRejudge)
            .await
            .expect_err("viewer should not rejudge");
        assert!(matches!(rejudge_err, AppError::DebateConflict(_)));
        Ok(())
    }

    #[tokio::test]
    async fn ops_role_assignment_crud_should_work() -> Result<()> {
        let (_tdb, state) = AppState::new_for_test().await?;
        state.update_workspace_owner(1, 1).await?;
        let owner = state.find_user_by_id(1).await?.expect("owner should exist");

        let created = state
            .upsert_ops_role_assignment_by_owner(
                &owner,
                2,
                UpsertOpsRoleInput {
                    role: "ops_reviewer".to_string(),
                },
            )
            .await?;
        assert_eq!(created.user_id, 2);
        assert_eq!(created.role, "ops_reviewer");

        let list = state.list_ops_role_assignments_by_owner(&owner).await?;
        assert_eq!(list.items.len(), 1);
        assert_eq!(list.items[0].user_id, 2);

        let revoked = state.revoke_ops_role_assignment_by_owner(&owner, 2).await?;
        assert!(revoked.removed);

        let list_after = state.list_ops_role_assignments_by_owner(&owner).await?;
        assert!(list_after.items.is_empty());
        Ok(())
    }
}
