//! Database abstraction macros
//!
//! Macros to eliminate duplicate SQLite/MySQL implementation code in repositories.
//! These macros generate both SQLite and MySQL variants from a single definition
//! and simplify the `match self.pool.driver()` dispatch boilerplate.

/// Dispatch a repository method call to the appropriate SQLite/MySQL implementation.
///
/// This macro replaces the repetitive `match self.pool.driver()` pattern used
/// in every trait impl method. It handles extracting the typed pool reference
/// and calling the corresponding backend-specific function.
///
/// # Examples
///
/// ```ignore
/// // Before:
/// async fn get_by_id(&self, id: i64) -> Result<Option<Entity>> {
///     match self.pool.driver() {
///         DatabaseDriver::Sqlite => {
///             get_entity_by_id_sqlite(self.pool.as_sqlite_or_err()?, id).await
///         }
///         DatabaseDriver::Mysql => {
///             get_entity_by_id_mysql(self.pool.as_mysql_or_err()?, id).await
///         }
///     }
/// }
///
/// // After:
/// async fn get_by_id(&self, id: i64) -> Result<Option<Entity>> {
///     dispatch!(self, get_entity_by_id, id)
/// }
/// ```
macro_rules! dispatch {
    ($self:expr, $fn_base:ident $(,)?) => {
        match $self.pool.driver() {
            crate::config::DatabaseDriver::Sqlite => {
                paste::paste! { [<$fn_base _sqlite>]($self.pool.as_sqlite_or_err()?).await }
            }
            crate::config::DatabaseDriver::Mysql => {
                paste::paste! { [<$fn_base _mysql>]($self.pool.as_mysql_or_err()?).await }
            }
        }
    };
    ($self:expr, $fn_base:ident, $($arg:expr),+ $(,)?) => {
        match $self.pool.driver() {
            crate::config::DatabaseDriver::Sqlite => {
                paste::paste! { [<$fn_base _sqlite>]($self.pool.as_sqlite_or_err()?, $($arg),+).await }
            }
            crate::config::DatabaseDriver::Mysql => {
                paste::paste! { [<$fn_base _mysql>]($self.pool.as_mysql_or_err()?, $($arg),+).await }
            }
        }
    };
}

/// Generate both SQLite and MySQL variants of a database function.
///
/// This macro takes a single function definition with a generic pool placeholder
/// and generates two concrete functions: one for `SqlitePool` and one for `MySqlPool`.
///
/// The function body is shared between both variants. For row mapping, use
/// `impl_row_mapper!` to generate a shared row mapper.
///
/// # Syntax
///
/// ```ignore
/// impl_dual_fn! {
///     /// Optional doc comment
///     pub(super) async fn get_entity_by_id(pool, id: i64) -> Result<Option<Entity>> {
///         // function body — `pool` is the pool variable name
///         let row = sqlx::query("SELECT * FROM entities WHERE id = ?")
///             .bind(id)
///             .fetch_optional(pool)
///             .await
///             .context("Failed to get entity")?;
///         match row {
///             Some(row) => Ok(Some(row_to_entity(&row)?)),
///             None => Ok(None),
///         }
///     }
/// }
/// ```
///
/// Generates:
/// - `get_entity_by_id_sqlite(pool: &SqlitePool, id: i64) -> Result<Option<Entity>>`
/// - `get_entity_by_id_mysql(pool: &MySqlPool, id: i64) -> Result<Option<Entity>>`
macro_rules! impl_dual_fn {
    (
        $(#[$meta:meta])*
        $vis:vis async fn $fn_name:ident($pool:ident $(, $param:ident : $ptype:ty)*) -> $ret:ty
        $body:block
    ) => {
        paste::paste! {
            $(#[$meta])*
            $vis async fn [<$fn_name _sqlite>]($pool: &::sqlx::SqlitePool $(, $param: $ptype)*) -> $ret
            $body

            $(#[$meta])*
            $vis async fn [<$fn_name _mysql>]($pool: &::sqlx::MySqlPool $(, $param: $ptype)*) -> $ret
            $body
        }
    };
}

/// Generate both SQLite and MySQL row mapper functions.
///
/// Since `SqliteRow` and `MySqlRow` are different types, we need separate
/// `row_to_xxx` functions for each. This macro generates both from a single
/// field mapping definition.
///
/// # Syntax
///
/// ```ignore
/// impl_row_mapper! {
///     fn row_to_session(row) -> Result<Session> {
///         Session {
///             id: row.get("id"),
///             user_id: row.get("user_id"),
///             expires_at: row.get("expires_at"),
///             created_at: row.get("created_at"),
///         }
///     }
/// }
/// ```
///
/// Generates `row_to_session_sqlite` and `row_to_session_mysql` functions.
macro_rules! impl_row_mapper {
    (
        $(#[$meta:meta])*
        $vis:vis fn $fn_name:ident($row:ident) -> Result<$entity:ty>
        $body:block
    ) => {
        paste::paste! {
            $(#[$meta])*
            $vis fn [<$fn_name _sqlite>]($row: &::sqlx::sqlite::SqliteRow) -> ::anyhow::Result<$entity>
            $body

            $(#[$meta])*
            $vis fn [<$fn_name _mysql>]($row: &::sqlx::mysql::MySqlRow) -> ::anyhow::Result<$entity>
            $body
        }
    };
}
