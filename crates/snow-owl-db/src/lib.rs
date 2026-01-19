use snow_owl_core::*;
use sqlx::postgres::{PgPool, PgPoolOptions};
use uuid::Uuid;

/// Database abstraction layer with security controls
///
/// NIST Controls:
/// - SI-10: Information Input Validation (parameterized queries prevent SQL injection)
/// - SC-28: Protection of Information at Rest (database encryption via PostgreSQL)
/// - AU-9: Protection of Audit Information (database integrity)
pub struct Database {
    pool: PgPool,
}

impl Database {
    pub async fn new(database_url: &str) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await?;

        let db = Self { pool };
        db.run_migrations().await?;

        Ok(db)
    }

    async fn run_migrations(&self) -> Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS machines (
                id UUID PRIMARY KEY,
                mac_address VARCHAR(17) NOT NULL UNIQUE,
                hostname TEXT,
                ip_address INET,
                last_seen TIMESTAMPTZ NOT NULL,
                created_at TIMESTAMPTZ NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS images (
                id UUID PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                description TEXT,
                image_type TEXT NOT NULL,
                file_path TEXT NOT NULL,
                size_bytes BIGINT NOT NULL,
                created_at TIMESTAMPTZ NOT NULL,
                checksum TEXT
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS deployments (
                id UUID PRIMARY KEY,
                machine_id UUID NOT NULL REFERENCES machines(id),
                image_id UUID NOT NULL REFERENCES images(id),
                status TEXT NOT NULL,
                started_at TIMESTAMPTZ NOT NULL,
                completed_at TIMESTAMPTZ,
                error_message TEXT
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // NIST AC-2: Account Management - users table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS users (
                id UUID PRIMARY KEY,
                username TEXT NOT NULL UNIQUE,
                role TEXT NOT NULL,
                created_at TIMESTAMPTZ NOT NULL,
                last_login TIMESTAMPTZ
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // NIST IA-5: Authenticator Management - API keys table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS api_keys (
                id UUID PRIMARY KEY,
                user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                name TEXT NOT NULL,
                key_hash TEXT NOT NULL UNIQUE,
                created_at TIMESTAMPTZ NOT NULL,
                expires_at TIMESTAMPTZ,
                last_used TIMESTAMPTZ
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // NIST AU-2: Audit Events - audit log table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS audit_log (
                id UUID PRIMARY KEY,
                user_id UUID REFERENCES users(id),
                action TEXT NOT NULL,
                resource_type TEXT,
                resource_id UUID,
                ip_address INET,
                user_agent TEXT,
                success BOOLEAN NOT NULL,
                error_message TEXT,
                created_at TIMESTAMPTZ NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    // Machine operations

    /// Create or update machine record with SQL injection protection
    ///
    /// NIST Controls:
    /// - SI-10: Information Input Validation (parameterized queries)
    /// - AU-3: Content of Audit Records (track machine registration)
    /// - CM-8: Information System Component Inventory (machine tracking)
    pub async fn create_or_update_machine(&self, machine: &Machine) -> Result<()> {
        // NIST SI-10: Use parameterized queries to prevent SQL injection
        // PostgreSQL placeholder syntax ($1, $2, ...) ensures safe parameter binding
        sqlx::query(
            r#"
            INSERT INTO machines (id, mac_address, hostname, ip_address, last_seen, created_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT(mac_address) DO UPDATE SET
                hostname = EXCLUDED.hostname,
                ip_address = EXCLUDED.ip_address,
                last_seen = EXCLUDED.last_seen
            "#,
        )
        .bind(machine.id)
        .bind(machine.mac_address.to_string())
        .bind(&machine.hostname)
        .bind(machine.ip_address.map(|ip| ip.to_string()))
        .bind(machine.last_seen)
        .bind(machine.created_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_machine_by_mac(&self, mac: &MacAddress) -> Result<Option<Machine>> {
        let row = sqlx::query_as::<_, MachineRow>("SELECT * FROM machines WHERE mac_address = $1")
            .bind(mac.to_string())
            .fetch_optional(&self.pool)
            .await?;

        Ok(row.and_then(|r| r.try_into().ok()))
    }

    pub async fn get_machine_by_id(&self, id: Uuid) -> Result<Option<Machine>> {
        let row = sqlx::query_as::<_, MachineRow>("SELECT * FROM machines WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;

        Ok(row.and_then(|r| r.try_into().ok()))
    }

    pub async fn list_machines(&self) -> Result<Vec<Machine>> {
        let rows =
            sqlx::query_as::<_, MachineRow>("SELECT * FROM machines ORDER BY last_seen DESC")
                .fetch_all(&self.pool)
                .await?;

        Ok(rows.into_iter().filter_map(|r| r.try_into().ok()).collect())
    }

    // Image operations
    pub async fn create_image(&self, image: &WindowsImage) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO images (id, name, description, image_type, file_path, size_bytes, created_at, checksum)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
        )
        .bind(image.id)
        .bind(&image.name)
        .bind(&image.description)
        .bind(serde_json::to_string(&image.image_type).unwrap())
        .bind(image.file_path.to_string_lossy().to_string())
        .bind(image.size_bytes as i64)
        .bind(image.created_at)
        .bind(&image.checksum)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_image_by_id(&self, id: Uuid) -> Result<Option<WindowsImage>> {
        let row = sqlx::query_as::<_, ImageRow>("SELECT * FROM images WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;

        Ok(row.and_then(|r| r.try_into().ok()))
    }

    pub async fn get_image_by_name(&self, name: &str) -> Result<Option<WindowsImage>> {
        let row = sqlx::query_as::<_, ImageRow>("SELECT * FROM images WHERE name = $1")
            .bind(name)
            .fetch_optional(&self.pool)
            .await?;

        Ok(row.and_then(|r| r.try_into().ok()))
    }

    pub async fn list_images(&self) -> Result<Vec<WindowsImage>> {
        let rows = sqlx::query_as::<_, ImageRow>("SELECT * FROM images ORDER BY created_at DESC")
            .fetch_all(&self.pool)
            .await?;

        Ok(rows.into_iter().filter_map(|r| r.try_into().ok()).collect())
    }

    pub async fn delete_image(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM images WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    // Deployment operations
    pub async fn create_deployment(&self, deployment: &Deployment) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO deployments (id, machine_id, image_id, status, started_at, completed_at, error_message)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(deployment.id)
        .bind(deployment.machine_id)
        .bind(deployment.image_id)
        .bind(serde_json::to_string(&deployment.status).unwrap())
        .bind(deployment.started_at)
        .bind(deployment.completed_at)
        .bind(&deployment.error_message)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn update_deployment_status(
        &self,
        id: Uuid,
        status: DeploymentStatus,
        error_message: Option<String>,
    ) -> Result<()> {
        let completed_at = if matches!(
            status,
            DeploymentStatus::Completed | DeploymentStatus::Failed
        ) {
            Some(chrono::Utc::now())
        } else {
            None
        };

        sqlx::query(
            r#"
            UPDATE deployments
            SET status = $1, completed_at = $2, error_message = $3
            WHERE id = $4
            "#,
        )
        .bind(serde_json::to_string(&status).unwrap())
        .bind(completed_at)
        .bind(error_message)
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_deployment_by_id(&self, id: Uuid) -> Result<Option<Deployment>> {
        let row = sqlx::query_as::<_, DeploymentRow>("SELECT * FROM deployments WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;

        Ok(row.and_then(|r| r.try_into().ok()))
    }

    pub async fn get_active_deployment_for_machine(
        &self,
        machine_id: Uuid,
    ) -> Result<Option<Deployment>> {
        let row = sqlx::query_as::<_, DeploymentRow>(
            r#"
            SELECT * FROM deployments
            WHERE machine_id = $1 AND status NOT IN ('"completed"', '"failed"')
            ORDER BY started_at DESC
            LIMIT 1
            "#,
        )
        .bind(machine_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.and_then(|r| r.try_into().ok()))
    }

    pub async fn list_deployments(&self) -> Result<Vec<Deployment>> {
        let rows = sqlx::query_as::<_, DeploymentRow>(
            "SELECT * FROM deployments ORDER BY started_at DESC",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().filter_map(|r| r.try_into().ok()).collect())
    }

    // User operations

    /// Create a new user account
    ///
    /// NIST Controls:
    /// - AC-2: Account Management
    /// - IA-2: Identification and Authentication
    /// - AU-3: Content of Audit Records
    pub async fn create_user(&self, user: &User) -> Result<()> {
        // NIST SI-10: Parameterized query prevents SQL injection
        sqlx::query(
            r#"
            INSERT INTO users (id, username, role, created_at, last_login)
            VALUES ($1, $2, $3, $4, $5)
            "#,
        )
        .bind(user.id)
        .bind(&user.username)
        .bind(user.role.to_string())
        .bind(user.created_at)
        .bind(user.last_login)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get user by username
    ///
    /// NIST Controls:
    /// - IA-2: Identification and Authentication
    pub async fn get_user_by_username(&self, username: &str) -> Result<Option<User>> {
        let row = sqlx::query_as::<_, UserRow>("SELECT * FROM users WHERE username = $1")
            .bind(username)
            .fetch_optional(&self.pool)
            .await?;

        Ok(row.and_then(|r| r.try_into().ok()))
    }

    /// Get user by ID
    pub async fn get_user_by_id(&self, id: Uuid) -> Result<Option<User>> {
        let row = sqlx::query_as::<_, UserRow>("SELECT * FROM users WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;

        Ok(row.and_then(|r| r.try_into().ok()))
    }

    /// Update user last login timestamp
    ///
    /// NIST Controls:
    /// - AU-3: Content of Audit Records
    pub async fn update_user_last_login(&self, user_id: Uuid) -> Result<()> {
        sqlx::query("UPDATE users SET last_login = $1 WHERE id = $2")
            .bind(chrono::Utc::now())
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// List all users
    pub async fn list_users(&self) -> Result<Vec<User>> {
        let rows = sqlx::query_as::<_, UserRow>("SELECT * FROM users ORDER BY created_at DESC")
            .fetch_all(&self.pool)
            .await?;

        Ok(rows.into_iter().filter_map(|r| r.try_into().ok()).collect())
    }

    // API Key operations

    /// Create a new API key
    ///
    /// NIST Controls:
    /// - IA-5: Authenticator Management
    /// - SC-12: Cryptographic Key Establishment
    /// - SC-13: Cryptographic Protection (key hashing)
    pub async fn create_api_key(&self, api_key: &ApiKey) -> Result<()> {
        // NIST SC-13: Store hashed API key, never plaintext
        sqlx::query(
            r#"
            INSERT INTO api_keys (id, user_id, name, key_hash, created_at, expires_at, last_used)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(api_key.id)
        .bind(api_key.user_id)
        .bind(&api_key.name)
        .bind(&api_key.key_hash)
        .bind(api_key.created_at)
        .bind(api_key.expires_at)
        .bind(api_key.last_used)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Validate API key and return associated user
    ///
    /// NIST Controls:
    /// - IA-2: Identification and Authentication (API key validation)
    /// - AC-3: Access Enforcement (retrieve user permissions)
    pub async fn validate_api_key(&self, key_hash: &str) -> Result<Option<(User, ApiKey)>> {
        // NIST IA-2: Validate key hash and check expiration
        let key_row = sqlx::query_as::<_, ApiKeyRow>(
            r#"
            SELECT * FROM api_keys
            WHERE key_hash = $1
            AND (expires_at IS NULL OR expires_at > NOW())
            "#,
        )
        .bind(key_hash)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(key_row) = key_row {
            let user_row = sqlx::query_as::<_, UserRow>("SELECT * FROM users WHERE id = $1")
                .bind(key_row.user_id)
                .fetch_optional(&self.pool)
                .await?;

            if let Some(user_row) = user_row {
                let user: User = user_row.try_into()?;
                let api_key: ApiKey = key_row.try_into()?;
                return Ok(Some((user, api_key)));
            }
        }

        Ok(None)
    }

    /// Update API key last used timestamp
    ///
    /// NIST Controls:
    /// - AU-3: Content of Audit Records
    pub async fn update_api_key_last_used(&self, key_id: Uuid) -> Result<()> {
        sqlx::query("UPDATE api_keys SET last_used = $1 WHERE id = $2")
            .bind(chrono::Utc::now())
            .bind(key_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// List API keys for a user
    pub async fn list_user_api_keys(&self, user_id: Uuid) -> Result<Vec<ApiKey>> {
        let rows = sqlx::query_as::<_, ApiKeyRow>(
            "SELECT * FROM api_keys WHERE user_id = $1 ORDER BY created_at DESC",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().filter_map(|r| r.try_into().ok()).collect())
    }

    /// Revoke (delete) an API key
    ///
    /// NIST Controls:
    /// - AC-2(4): Automated Audit Actions
    pub async fn revoke_api_key(&self, key_id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM api_keys WHERE id = $1")
            .bind(key_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}

// Row structures for PostgreSQL
#[derive(sqlx::FromRow)]
struct MachineRow {
    id: Uuid,
    mac_address: String,
    hostname: Option<String>,
    ip_address: Option<String>,
    last_seen: chrono::DateTime<chrono::Utc>,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl TryFrom<MachineRow> for Machine {
    type Error = anyhow::Error;

    fn try_from(row: MachineRow) -> std::result::Result<Self, Self::Error> {
        Ok(Machine {
            id: row.id,
            mac_address: row.mac_address.parse()?,
            hostname: row.hostname,
            ip_address: row.ip_address.and_then(|ip| ip.parse().ok()),
            last_seen: row.last_seen,
            created_at: row.created_at,
        })
    }
}

#[derive(sqlx::FromRow)]
struct ImageRow {
    id: Uuid,
    name: String,
    description: Option<String>,
    image_type: String,
    file_path: String,
    size_bytes: i64,
    created_at: chrono::DateTime<chrono::Utc>,
    checksum: Option<String>,
}

impl TryFrom<ImageRow> for WindowsImage {
    type Error = anyhow::Error;

    fn try_from(row: ImageRow) -> std::result::Result<Self, Self::Error> {
        Ok(WindowsImage {
            id: row.id,
            name: row.name,
            description: row.description,
            image_type: serde_json::from_str(&row.image_type)?,
            file_path: row.file_path.into(),
            size_bytes: row.size_bytes as u64,
            created_at: row.created_at,
            checksum: row.checksum,
        })
    }
}

#[derive(sqlx::FromRow)]
struct DeploymentRow {
    id: Uuid,
    machine_id: Uuid,
    image_id: Uuid,
    status: String,
    started_at: chrono::DateTime<chrono::Utc>,
    completed_at: Option<chrono::DateTime<chrono::Utc>>,
    error_message: Option<String>,
}

impl TryFrom<DeploymentRow> for Deployment {
    type Error = anyhow::Error;

    fn try_from(row: DeploymentRow) -> std::result::Result<Self, Self::Error> {
        Ok(Deployment {
            id: row.id,
            machine_id: row.machine_id,
            image_id: row.image_id,
            status: serde_json::from_str(&row.status)?,
            started_at: row.started_at,
            completed_at: row.completed_at,
            error_message: row.error_message,
        })
    }
}

#[derive(sqlx::FromRow)]
struct UserRow {
    id: Uuid,
    username: String,
    role: String,
    created_at: chrono::DateTime<chrono::Utc>,
    last_login: Option<chrono::DateTime<chrono::Utc>>,
}

impl TryFrom<UserRow> for User {
    type Error = anyhow::Error;

    fn try_from(row: UserRow) -> std::result::Result<Self, Self::Error> {
        let role = match row.role.as_str() {
            "admin" => UserRole::Admin,
            "operator" => UserRole::Operator,
            "readonly" => UserRole::ReadOnly,
            _ => return Err(anyhow::anyhow!("Invalid user role: {}", row.role)),
        };

        Ok(User {
            id: row.id,
            username: row.username,
            role,
            created_at: row.created_at,
            last_login: row.last_login,
        })
    }
}

#[derive(sqlx::FromRow)]
struct ApiKeyRow {
    id: Uuid,
    user_id: Uuid,
    name: String,
    key_hash: String,
    created_at: chrono::DateTime<chrono::Utc>,
    expires_at: Option<chrono::DateTime<chrono::Utc>>,
    last_used: Option<chrono::DateTime<chrono::Utc>>,
}

impl TryFrom<ApiKeyRow> for ApiKey {
    type Error = anyhow::Error;

    fn try_from(row: ApiKeyRow) -> std::result::Result<Self, Self::Error> {
        Ok(ApiKey {
            id: row.id,
            user_id: row.user_id,
            name: row.name,
            key_hash: row.key_hash,
            created_at: row.created_at,
            expires_at: row.expires_at,
            last_used: row.last_used,
        })
    }
}
