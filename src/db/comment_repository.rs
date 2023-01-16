use crate::{
  helpers::api::map_db_err,
  logic::LogicErr,
  model::{comment::Comment, comment_pub::CommentPub},
};

use super::FromRow;
use async_trait::async_trait;
use deadpool_postgres::Pool;
use std::sync::Arc;
use uuid::Uuid;

#[cfg(test)]
use mockall::automock;

#[cfg_attr(test, automock)]
#[async_trait]
pub trait CommentRepo {
  async fn fetch_comments(
    &self,
    post_id: &Uuid,
    own_user_id: &Option<Uuid>,
    limit: i64,
    skip: i64,
  ) -> Result<Vec<CommentPub>, LogicErr>;
  async fn create_comment(
    &self,
    user_id: &Uuid,
    post_id: &Uuid,
    content_md: &str,
    content_html: &str,
    is_external: bool,
  ) -> Result<Uuid, LogicErr>;
  async fn delete_comment(&self, user_id: &Uuid, post_id: &Uuid, comment_id: &Uuid) -> Result<(), LogicErr>;
  async fn fetch_comments_count(&self, post_id: &Uuid, own_user_id: &Option<Uuid>) -> Result<i64, LogicErr>;
  async fn create_comment_like(&self, user_id: &Uuid, comment_id: &Uuid, post_id: &Uuid) -> Result<(), LogicErr>;
  async fn delete_comment_like(&self, user_id: &Uuid, comment_id: &Uuid, post_id: &Uuid) -> Result<(), LogicErr>;
  async fn fetch_comment_count(&self) -> i64;
  async fn fetch_comment(&self, post_id: &Uuid, comment_id: &Uuid, own_user_id: &Option<Uuid>) -> Option<CommentPub>;
  async fn fetch_comment_pub_by_id(&self, comment_id: &Uuid, own_user_id: &Option<Uuid>) -> Option<CommentPub>;
  async fn fetch_comment_by_id(&self, comment_id: &Uuid) -> Option<Comment>;
}

pub type CommentPool = Arc<dyn CommentRepo + Send + Sync>;

pub struct DbCommentRepo {
  pub db: Pool,
}

#[async_trait]
impl CommentRepo for DbCommentRepo {
  async fn fetch_comments(
    &self,
    post_id: &Uuid,
    own_user_id: &Option<Uuid>,
    limit: i64,
    skip: i64,
  ) -> Result<Vec<CommentPub>, LogicErr> {
    let db = self.db.get().await.map_err(map_db_err)?;
    let rows = db
      .query(
        include_str!("./sql/fetch_post_comments.sql"),
        &[&own_user_id, &post_id, &limit, &skip],
      )
      .await
      .map_err(map_db_err)?;

    Ok(rows.into_iter().flat_map(CommentPub::from_row).collect())
  }

  async fn create_comment(
    &self,
    user_id: &Uuid,
    post_id: &Uuid,
    content_md: &str,
    content_html: &str,
    is_external: bool,
  ) -> Result<Uuid, LogicErr> {
    let comment_id = Uuid::new_v4();
    let uri = format!("/api/feed/{}/comments/{}", post_id, comment_id);
    let replies_uri = format!("/api/feed/{}/comments", post_id);

    let db = self.db.get().await.map_err(map_db_err)?;
    let row = db.query_one("INSERT INTO comments (comment_id, user_id, post_id, content_md, content_html, uri, replies_uri, is_external) VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING comment_id",
      &[
        &comment_id,
        &user_id,
        &post_id,
        &content_md,
        &content_html,
        &uri,
        &replies_uri,
        &is_external,
      ],
    )
    .await
    .map_err(map_db_err)?;

    Ok(row.get(0))
  }

  async fn delete_comment(&self, user_id: &Uuid, post_id: &Uuid, comment_id: &Uuid) -> Result<(), LogicErr> {
    let db = self.db.get().await.map_err(map_db_err)?;
    db.execute(
      "DELETE FROM comments WHERE post_id = $1 AND user_id = $2 AND comment_id = $3",
      &[&post_id, &user_id, &comment_id],
    )
    .await
    .map_err(map_db_err)?;

    Ok(())
  }

  async fn fetch_comments_count(&self, post_id: &Uuid, own_user_id: &Option<Uuid>) -> Result<i64, LogicErr> {
    let db = self.db.get().await.map_err(map_db_err)?;
    let row = db
      .query_one(
        include_str!("./sql/fetch_post_comments_count.sql"),
        &[&own_user_id, &post_id],
      )
      .await
      .map_err(map_db_err)?;

    Ok(row.get(0))
  }

  async fn create_comment_like(&self, user_id: &Uuid, comment_id: &Uuid, post_id: &Uuid) -> Result<(), LogicErr> {
    let comment_like_id = Uuid::new_v4();

    let db = self.db.get().await.map_err(map_db_err)?;
    db.execute(
      "INSERT INTO comment_likes (comment_like_id, user_id, comment_id, post_id) VALUES($1, $2, $3, $4)",
      &[&comment_like_id, &user_id, &comment_id, &post_id],
    )
    .await
    .map_err(map_db_err)?;

    Ok(())
  }

  async fn delete_comment_like(&self, user_id: &Uuid, comment_id: &Uuid, post_id: &Uuid) -> Result<(), LogicErr> {
    let db = self.db.get().await.map_err(map_db_err)?;
    db.execute(
      "DELETE FROM comment_likes WHERE user_id = $1 AND comment_id = $2 AND post_id = $3",
      &[&user_id, &comment_id, &post_id],
    )
    .await
    .map_err(map_db_err)?;

    Ok(())
  }

  async fn fetch_comment_count(&self) -> i64 {
    let db = match self.db.get().await.map_err(map_db_err) {
      Ok(db) => db,
      Err(_) => return 0,
    };

    let row = match db
      .query_one("SELECT COUNT(*) FROM comments", &[])
      .await
      .map_err(map_db_err)
    {
      Ok(row) => row,
      Err(_) => return 0,
    };

    row.get(0)
  }

  async fn fetch_comment(&self, post_id: &Uuid, comment_id: &Uuid, own_user_id: &Option<Uuid>) -> Option<CommentPub> {
    let db = match self.db.get().await.map_err(map_db_err) {
      Ok(db) => db,
      Err(_) => return None,
    };

    let row = match db
      .query_opt(
        include_str!("./sql/fetch_post_comment.sql"),
        &[&own_user_id, &post_id, &comment_id],
      )
      .await
      .map_err(map_db_err)
    {
      Ok(row) => row,
      Err(_) => return None,
    };

    row.and_then(CommentPub::from_row)
  }

  async fn fetch_comment_pub_by_id(&self, comment_id: &Uuid, own_user_id: &Option<Uuid>) -> Option<CommentPub> {
    let db = match self.db.get().await.map_err(map_db_err) {
      Ok(db) => db,
      Err(_) => return None,
    };

    let row = match db
      .query_opt(
        include_str!("./sql/fetch_post_comment_pub.sql"),
        &[&own_user_id, &comment_id],
      )
      .await
      .map_err(map_db_err)
    {
      Ok(row) => row,
      Err(_) => return None,
    };

    row.and_then(CommentPub::from_row)
  }

  async fn fetch_comment_by_id(&self, comment_id: &Uuid) -> Option<Comment> {
    let db = match self.db.get().await.map_err(map_db_err) {
      Ok(db) => db,
      Err(_) => return None,
    };

    let row = match db
      .query_opt("SELECT * FROM comments WHERE comment_id = $1", &[&comment_id])
      .await
      .map_err(map_db_err)
    {
      Ok(row) => row,
      Err(_) => return None,
    };

    row.and_then(Comment::from_row)
  }
}
