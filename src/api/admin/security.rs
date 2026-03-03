//! Login logs (security) endpoints

use axum::{
    extract::{Query, State},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::api::common::{default_page_i64, default_per_page};
use crate::api::middleware::{ApiError, AppState, AuthenticatedUser};

/// Login log entry
#[derive(Debug, Serialize)]
pub struct LoginLogEntry {
    pub id: i64,
    pub username: String,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub success: bool,
    pub failure_reason: Option<String>,
    pub created_at: String,
}

/// Query parameters for login logs
#[derive(Debug, Deserialize)]
pub struct LoginLogsQuery {
    #[serde(default = "default_page_i64")]
    pub page: i64,
    #[serde(default = "default_per_page")]
    pub per_page: i64,
    pub username: Option<String>,
    pub ip_address: Option<String>,
    pub success: Option<bool>,
}

/// Response for login logs list
#[derive(Debug, Serialize)]
pub struct LoginLogsResponse {
    pub logs: Vec<LoginLogEntry>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
    pub success_count: i64,
    pub failed_count: i64,
}

/// GET /api/v1/admin/login-logs - List login logs
///
/// Query parameters:
/// - page: Page number (default: 1)
/// - per_page: Items per page (default: 20)
/// - username: Filter by username (optional)
/// - ip_address: Filter by IP address (optional)
/// - success: Filter by success status (optional)
///
/// Requires admin authentication.
pub async fn list_login_logs(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
    Query(query): Query<LoginLogsQuery>,
) -> Result<Json<LoginLogsResponse>, ApiError> {
    use crate::config::DatabaseDriver;
    
    let offset = (query.page - 1) * query.per_page;
    
    // Build WHERE clause
    let mut where_clauses = Vec::new();
    
    if query.username.is_some() {
        where_clauses.push("username LIKE ?".to_string());
    }
    if query.ip_address.is_some() {
        where_clauses.push("ip_address = ?".to_string());
    }
    if query.success.is_some() {
        where_clauses.push("success = ?".to_string());
    }
    
    let where_sql = if where_clauses.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", where_clauses.join(" AND "))
    };
    
    // Get total count
    let count_sql = format!("SELECT COUNT(*) FROM login_logs {}", where_sql);
    let total: i64 = match state.pool.driver() {
        DatabaseDriver::Sqlite => {
            let mut query_builder = sqlx::query_scalar(&count_sql);
            if let Some(ref username) = query.username {
                query_builder = query_builder.bind(format!("%{}%", username));
            }
            if let Some(ref ip) = query.ip_address {
                query_builder = query_builder.bind(ip);
            }
            if let Some(success) = query.success {
                query_builder = query_builder.bind(if success { 1 } else { 0 });
            }
            query_builder
                .fetch_one(state.pool.as_sqlite().expect("sqlite pool"))
                .await
                .map_err(|e| ApiError::internal_error(e.to_string()))?
        }
        DatabaseDriver::Mysql => {
            let mut query_builder = sqlx::query_scalar(&count_sql);
            if let Some(ref username) = query.username {
                query_builder = query_builder.bind(format!("%{}%", username));
            }
            if let Some(ref ip) = query.ip_address {
                query_builder = query_builder.bind(ip);
            }
            if let Some(success) = query.success {
                query_builder = query_builder.bind(if success { 1 } else { 0 });
            }
            query_builder
                .fetch_one(state.pool.as_mysql().expect("mysql pool"))
                .await
                .map_err(|e| ApiError::internal_error(e.to_string()))?
        }
    };
    
    // Get success/failed counts with same filters (excluding success filter)
    let mut base_where_clauses = Vec::new();
    
    if query.username.is_some() {
        base_where_clauses.push("username LIKE ?".to_string());
    }
    if query.ip_address.is_some() {
        base_where_clauses.push("ip_address = ?".to_string());
    }
    
    let base_where_sql = if base_where_clauses.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", base_where_clauses.join(" AND "))
    };
    
    // Build WHERE clause for success/failed counts
    let success_where = if base_where_sql.is_empty() {
        "WHERE success = 1".to_string()
    } else {
        format!("{} AND success = 1", base_where_sql)
    };
    let failed_where = if base_where_sql.is_empty() {
        "WHERE success = 0".to_string()
    } else {
        format!("{} AND success = 0", base_where_sql)
    };
    
    let success_count_sql = format!("SELECT COUNT(*) FROM login_logs {}", success_where);
    let failed_count_sql = format!("SELECT COUNT(*) FROM login_logs {}", failed_where);
    
    let (success_count, failed_count): (i64, i64) = match state.pool.driver() {
        DatabaseDriver::Sqlite => {
            let mut success_query = sqlx::query_scalar(&success_count_sql);
            let mut failed_query = sqlx::query_scalar(&failed_count_sql);
            
            if let Some(ref username) = query.username {
                success_query = success_query.bind(format!("%{}%", username));
            }
            if let Some(ref ip) = query.ip_address {
                success_query = success_query.bind(ip);
            }
            
            if let Some(ref username) = query.username {
                failed_query = failed_query.bind(format!("%{}%", username));
            }
            if let Some(ref ip) = query.ip_address {
                failed_query = failed_query.bind(ip);
            }
            
            let success: i64 = success_query
                .fetch_one(state.pool.as_sqlite().expect("sqlite pool"))
                .await
                .map_err(|e| ApiError::internal_error(e.to_string()))?;
            let failed: i64 = failed_query
                .fetch_one(state.pool.as_sqlite().expect("sqlite pool"))
                .await
                .map_err(|e| ApiError::internal_error(e.to_string()))?;
            (success, failed)
        }
        DatabaseDriver::Mysql => {
            let mut success_query = sqlx::query_scalar(&success_count_sql);
            let mut failed_query = sqlx::query_scalar(&failed_count_sql);
            
            if let Some(ref username) = query.username {
                success_query = success_query.bind(format!("%{}%", username));
            }
            if let Some(ref ip) = query.ip_address {
                success_query = success_query.bind(ip);
            }
            
            if let Some(ref username) = query.username {
                failed_query = failed_query.bind(format!("%{}%", username));
            }
            if let Some(ref ip) = query.ip_address {
                failed_query = failed_query.bind(ip);
            }
            
            let success: i64 = success_query
                .fetch_one(state.pool.as_mysql().expect("mysql pool"))
                .await
                .map_err(|e| ApiError::internal_error(e.to_string()))?;
            let failed: i64 = failed_query
                .fetch_one(state.pool.as_mysql().expect("mysql pool"))
                .await
                .map_err(|e| ApiError::internal_error(e.to_string()))?;
            (success, failed)
        }
    };
    
    // Get logs
    let logs_sql = format!(
        "SELECT id, username, ip_address, user_agent, success, failure_reason, {} FROM login_logs {} ORDER BY created_at DESC LIMIT ? OFFSET ?",
        match state.pool.driver() {
            DatabaseDriver::Sqlite => "created_at",
            DatabaseDriver::Mysql => "DATE_FORMAT(created_at, '%Y-%m-%dT%H:%i:%sZ') as created_at",
        },
        where_sql
    );
    
    let logs: Vec<LoginLogEntry> = match state.pool.driver() {
        DatabaseDriver::Sqlite => {
            let mut query_builder = sqlx::query_as::<_, (i64, String, Option<String>, Option<String>, i64, Option<String>, String)>(&logs_sql);
            if let Some(ref username) = query.username {
                query_builder = query_builder.bind(format!("%{}%", username));
            }
            if let Some(ref ip) = query.ip_address {
                query_builder = query_builder.bind(ip);
            }
            if let Some(success) = query.success {
                query_builder = query_builder.bind(if success { 1 } else { 0 });
            }
            query_builder
                .bind(query.per_page)
                .bind(offset)
                .fetch_all(state.pool.as_sqlite().expect("sqlite pool"))
                .await
                .map_err(|e| ApiError::internal_error(e.to_string()))?
                .into_iter()
                .map(|(id, username, ip_address, user_agent, success, failure_reason, created_at)| {
                    LoginLogEntry {
                        id,
                        username,
                        ip_address,
                        user_agent,
                        success: success != 0,
                        failure_reason,
                        created_at,
                    }
                })
                .collect()
        }
        DatabaseDriver::Mysql => {
            let mut query_builder = sqlx::query_as::<_, (i64, String, Option<String>, Option<String>, i8, Option<String>, String)>(&logs_sql);
            if let Some(ref username) = query.username {
                query_builder = query_builder.bind(format!("%{}%", username));
            }
            if let Some(ref ip) = query.ip_address {
                query_builder = query_builder.bind(ip);
            }
            if let Some(success) = query.success {
                query_builder = query_builder.bind(if success { 1 } else { 0 });
            }
            query_builder
                .bind(query.per_page)
                .bind(offset)
                .fetch_all(state.pool.as_mysql().expect("mysql pool"))
                .await
                .map_err(|e| ApiError::internal_error(e.to_string()))?
                .into_iter()
                .map(|(id, username, ip_address, user_agent, success, failure_reason, created_at)| {
                    LoginLogEntry {
                        id,
                        username,
                        ip_address,
                        user_agent,
                        success: success != 0,
                        failure_reason,
                        created_at,
                    }
                })
                .collect()
        }
    };
    
    Ok(Json(LoginLogsResponse {
        logs,
        total,
        page: query.page,
        per_page: query.per_page,
        success_count,
        failed_count,
    }))
}
