use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio_postgres::Row;
use uuid::Uuid;

use crate::db::FromRow;

#[derive(Deserialize, Serialize)]
/// Represents a user's comment on a post
pub struct Comment {
  pub comment_id: Uuid,
  pub user_id: Uuid,
  pub post_id: Uuid,
  pub content_md: String,
  pub content_html: String,
  pub created_at: DateTime<Utc>,
  pub updated_at: DateTime<Utc>,
  pub is_external: bool,
  pub uri: String,
  pub replies_uri: String,
}

impl FromRow for Comment {
  fn from_row(row: Row) -> Option<Self> {
    Some(Comment {
      comment_id: row.get("comment_id"),
      user_id: row.get("user_id"),
      post_id: row.get("post_id"),
      content_md: row.get("content_md"),
      content_html: row.get("content_html"),
      created_at: row.get("created_at"),
      updated_at: row.get("updated_at"),
      is_external: row.get("is_external"),
      uri: row.get("uri"),
      replies_uri: row.get("replies_uri"),
    })
  }
}
