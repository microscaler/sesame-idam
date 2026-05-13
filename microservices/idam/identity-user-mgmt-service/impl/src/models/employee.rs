use lifeguard_derive::{LifeModel, LifeRecord};
use serde::{Deserialize, Serialize};

/// Employee metadata linked to a user.
#[derive(Clone, Debug, Serialize, Deserialize, LifeModel, LifeRecord)]
#[table_name = "employees"]
#[schema_name = "sesame_idam"]
pub struct Employee {
    #[primary_key]
    #[column_type = "UUID"]
    pub id: uuid::Uuid,

    #[column_type = "UUID"]
    #[foreign_key = "sesame_idam.users(id) ON DELETE CASCADE"]
    pub user_id: uuid::Uuid,

    #[column_type = "VARCHAR(64)"]
    pub employee_id: String,

    #[column_type = "VARCHAR(255)"]
    #[nullable]
    pub department: Option<String>,

    #[column_type = "VARCHAR(255)"]
    #[nullable]
    pub title: Option<String>,

    #[column_type = "UUID"]
    #[nullable]
    #[foreign_key = "sesame_idam.users(id) ON DELETE SET NULL"]
    pub manager_id: Option<uuid::Uuid>,

    #[column_type = "TIMESTAMP WITH TIME ZONE"]
    pub created_at: chrono::DateTime<chrono::Utc>,

    #[column_type = "TIMESTAMP WITH TIME ZONE"]
    pub updated_at: chrono::DateTime<chrono::Utc>,
}
