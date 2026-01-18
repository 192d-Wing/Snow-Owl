use snow_owl_core::*;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use std::path::Path;
use std::str::FromStr;
use uuid::Uuid;

pub struct Database {
    pool: SqlitePool,
}

impl Database {
    pub async fn new(database_path: &Path) -> Result<Self> {
        let options = SqliteConnectOptions::from_str(&format!("sqlite://{}", database_path.display()))?
            .create_if_missing(true);

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await?;

        let db = Self { pool };
        db.run_migrations().await?;

        Ok(db)
    }

    async fn run_migrations(&self) -> Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS machines (
                id TEXT PRIMARY KEY,
                mac_address TEXT NOT NULL UNIQUE,
                hostname TEXT,
                ip_address TEXT,
                last_seen TEXT NOT NULL,
                created_at TEXT NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS images (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                description TEXT,
                image_type TEXT NOT NULL,
                file_path TEXT NOT NULL,
                size_bytes INTEGER NOT NULL,
                created_at TEXT NOT NULL,
                checksum TEXT
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS deployments (
                id TEXT PRIMARY KEY,
                machine_id TEXT NOT NULL,
                image_id TEXT NOT NULL,
                status TEXT NOT NULL,
                started_at TEXT NOT NULL,
                completed_at TEXT,
                error_message TEXT,
                FOREIGN KEY (machine_id) REFERENCES machines(id),
                FOREIGN KEY (image_id) REFERENCES images(id)
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    // Machine operations
    pub async fn create_or_update_machine(&self, machine: &Machine) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO machines (id, mac_address, hostname, ip_address, last_seen, created_at)
            VALUES (?, ?, ?, ?, ?, ?)
            ON CONFLICT(mac_address) DO UPDATE SET
                hostname = excluded.hostname,
                ip_address = excluded.ip_address,
                last_seen = excluded.last_seen
            "#,
        )
        .bind(machine.id.to_string())
        .bind(machine.mac_address.to_string())
        .bind(&machine.hostname)
        .bind(machine.ip_address.map(|ip| ip.to_string()))
        .bind(machine.last_seen.to_rfc3339())
        .bind(machine.created_at.to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_machine_by_mac(&self, mac: &MacAddress) -> Result<Option<Machine>> {
        let row = sqlx::query_as::<_, MachineRow>(
            "SELECT * FROM machines WHERE mac_address = ?"
        )
        .bind(mac.to_string())
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| r.into()))
    }

    pub async fn get_machine_by_id(&self, id: Uuid) -> Result<Option<Machine>> {
        let row = sqlx::query_as::<_, MachineRow>(
            "SELECT * FROM machines WHERE id = ?"
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| r.into()))
    }

    pub async fn list_machines(&self) -> Result<Vec<Machine>> {
        let rows = sqlx::query_as::<_, MachineRow>(
            "SELECT * FROM machines ORDER BY last_seen DESC"
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    // Image operations
    pub async fn create_image(&self, image: &WindowsImage) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO images (id, name, description, image_type, file_path, size_bytes, created_at, checksum)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(image.id.to_string())
        .bind(&image.name)
        .bind(&image.description)
        .bind(serde_json::to_string(&image.image_type).unwrap())
        .bind(image.file_path.to_string_lossy().to_string())
        .bind(image.size_bytes as i64)
        .bind(image.created_at.to_rfc3339())
        .bind(&image.checksum)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_image_by_id(&self, id: Uuid) -> Result<Option<WindowsImage>> {
        let row = sqlx::query_as::<_, ImageRow>(
            "SELECT * FROM images WHERE id = ?"
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| r.try_into().ok()).flatten())
    }

    pub async fn get_image_by_name(&self, name: &str) -> Result<Option<WindowsImage>> {
        let row = sqlx::query_as::<_, ImageRow>(
            "SELECT * FROM images WHERE name = ?"
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| r.try_into().ok()).flatten())
    }

    pub async fn list_images(&self) -> Result<Vec<WindowsImage>> {
        let rows = sqlx::query_as::<_, ImageRow>(
            "SELECT * FROM images ORDER BY created_at DESC"
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().filter_map(|r| r.try_into().ok()).collect())
    }

    pub async fn delete_image(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM images WHERE id = ?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    // Deployment operations
    pub async fn create_deployment(&self, deployment: &Deployment) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO deployments (id, machine_id, image_id, status, started_at, completed_at, error_message)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(deployment.id.to_string())
        .bind(deployment.machine_id.to_string())
        .bind(deployment.image_id.to_string())
        .bind(serde_json::to_string(&deployment.status).unwrap())
        .bind(deployment.started_at.to_rfc3339())
        .bind(deployment.completed_at.map(|dt| dt.to_rfc3339()))
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
        let completed_at = if matches!(status, DeploymentStatus::Completed | DeploymentStatus::Failed) {
            Some(chrono::Utc::now().to_rfc3339())
        } else {
            None
        };

        sqlx::query(
            r#"
            UPDATE deployments
            SET status = ?, completed_at = ?, error_message = ?
            WHERE id = ?
            "#,
        )
        .bind(serde_json::to_string(&status).unwrap())
        .bind(completed_at)
        .bind(error_message)
        .bind(id.to_string())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_deployment_by_id(&self, id: Uuid) -> Result<Option<Deployment>> {
        let row = sqlx::query_as::<_, DeploymentRow>(
            "SELECT * FROM deployments WHERE id = ?"
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| r.try_into().ok()).flatten())
    }

    pub async fn get_active_deployment_for_machine(&self, machine_id: Uuid) -> Result<Option<Deployment>> {
        let row = sqlx::query_as::<_, DeploymentRow>(
            r#"
            SELECT * FROM deployments
            WHERE machine_id = ? AND status NOT IN ('"completed"', '"failed"')
            ORDER BY started_at DESC
            LIMIT 1
            "#
        )
        .bind(machine_id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| r.try_into().ok()).flatten())
    }

    pub async fn list_deployments(&self) -> Result<Vec<Deployment>> {
        let rows = sqlx::query_as::<_, DeploymentRow>(
            "SELECT * FROM deployments ORDER BY started_at DESC"
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().filter_map(|r| r.try_into().ok()).collect())
    }
}

// Row structures for SQLite
#[derive(sqlx::FromRow)]
struct MachineRow {
    id: String,
    mac_address: String,
    hostname: Option<String>,
    ip_address: Option<String>,
    last_seen: String,
    created_at: String,
}

impl From<MachineRow> for Machine {
    fn from(row: MachineRow) -> Self {
        Machine {
            id: Uuid::parse_str(&row.id).unwrap(),
            mac_address: row.mac_address.parse().unwrap(),
            hostname: row.hostname,
            ip_address: row.ip_address.and_then(|ip| ip.parse().ok()),
            last_seen: chrono::DateTime::parse_from_rfc3339(&row.last_seen)
                .unwrap()
                .with_timezone(&chrono::Utc),
            created_at: chrono::DateTime::parse_from_rfc3339(&row.created_at)
                .unwrap()
                .with_timezone(&chrono::Utc),
        }
    }
}

#[derive(sqlx::FromRow)]
struct ImageRow {
    id: String,
    name: String,
    description: Option<String>,
    image_type: String,
    file_path: String,
    size_bytes: i64,
    created_at: String,
    checksum: Option<String>,
}

impl TryFrom<ImageRow> for WindowsImage {
    type Error = anyhow::Error;

    fn try_from(row: ImageRow) -> std::result::Result<Self, Self::Error> {
        Ok(WindowsImage {
            id: Uuid::parse_str(&row.id)?,
            name: row.name,
            description: row.description,
            image_type: serde_json::from_str(&row.image_type)?,
            file_path: row.file_path.into(),
            size_bytes: row.size_bytes as u64,
            created_at: chrono::DateTime::parse_from_rfc3339(&row.created_at)?
                .with_timezone(&chrono::Utc),
            checksum: row.checksum,
        })
    }
}

#[derive(sqlx::FromRow)]
struct DeploymentRow {
    id: String,
    machine_id: String,
    image_id: String,
    status: String,
    started_at: String,
    completed_at: Option<String>,
    error_message: Option<String>,
}

impl TryFrom<DeploymentRow> for Deployment {
    type Error = anyhow::Error;

    fn try_from(row: DeploymentRow) -> std::result::Result<Self, Self::Error> {
        Ok(Deployment {
            id: Uuid::parse_str(&row.id)?,
            machine_id: Uuid::parse_str(&row.machine_id)?,
            image_id: Uuid::parse_str(&row.image_id)?,
            status: serde_json::from_str(&row.status)?,
            started_at: chrono::DateTime::parse_from_rfc3339(&row.started_at)?
                .with_timezone(&chrono::Utc),
            completed_at: row
                .completed_at
                .map(|dt| chrono::DateTime::parse_from_rfc3339(&dt))
                .transpose()?
                .map(|dt| dt.with_timezone(&chrono::Utc)),
            error_message: row.error_message,
        })
    }
}
