use std::str::FromStr;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio_postgres::Row;
use uuid::Uuid;

use crate::{
  activitypub::{
    activity_convertible::ActivityConvertible,
    collection::CollectionProps,
    object::{Object, ObjectSource, ObjectType},
    rdf_string::RdfString,
    reference::Reference,
  },
  db::FromRow,
  helpers::api::relative_to_absolute_uri,
  settings::SETTINGS,
};

use super::access_type::AccessType;

#[derive(Deserialize, Serialize, Eq, PartialEq, Debug, Clone)]
/// Represents a user's comment on a post
pub struct CommentPub {
  pub comment_id: Uuid,
  pub user_id: Uuid,
  pub post_id: Uuid,
  // TODO: Add post_uri to
  pub content_md: String,
  pub content_html: String,
  pub created_at: DateTime<Utc>,
  pub updated_at: DateTime<Utc>,
  pub user_handle: String,
  pub user_fediverse_id: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub user_avatar_url: Option<String>,
  pub likes: i64,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub liked: Option<bool>,
  #[serde(skip)]
  pub visibility: AccessType,
  pub is_external: bool,
  pub uri: String,
  pub replies_uri: String,
}

impl FromRow for CommentPub {
  fn from_row(row: Row) -> Option<Self> {
    Some(CommentPub {
      comment_id: row.get("comment_id"),
      user_id: row.get("user_id"),
      post_id: row.get("post_id"),
      content_md: row.get("content_md"),
      content_html: row.get("content_html"),
      created_at: row.get("created_at"),
      updated_at: row.get("updated_at"),
      user_handle: row.get("user_handle"),
      user_fediverse_id: row.get("user_fediverse_id"),
      user_avatar_url: row.get("user_avatar_url"),
      likes: row.get("likes"),
      liked: row.get("liked"),
      visibility: AccessType::from_str(row.get("visibility")).unwrap_or_default(),
      is_external: row.get("is_external"),
      uri: row.get("uri"),
      replies_uri: row.get("replies_uri"),
    })
  }
}

impl ActivityConvertible for CommentPub {
  fn to_object(&self, actor: &str) -> Option<Object> {
    let attributed_to_uri = format!("{}/user/{}", SETTINGS.server.api_fqdn, self.user_id);
    let cc_uri = format!("{}/followers", actor);
    let in_reply_to_uri = format!("{}/feed/{}", SETTINGS.server.api_fqdn, self.post_id);

    let to = match self.visibility {
      AccessType::Shadow => None,
      AccessType::Unlisted => None,
      AccessType::Private => None,
      AccessType::FollowersOnly => Some(Reference::Remote::<Object>(cc_uri.clone())),
      AccessType::PublicLocal => Some(Reference::Mixed::<Object>(vec![
        Reference::Remote::<Object>("https://www.w3.org/ns/activitystreams#Local".to_string()),
        Reference::Remote::<Object>(cc_uri.clone()),
      ])),
      AccessType::PublicFederated => Some(Reference::Mixed::<Object>(vec![
        Reference::Remote::<Object>("https://www.w3.org/ns/activitystreams#Public".to_string()),
        Reference::Remote::<Object>(cc_uri.clone()),
      ])),
      _ => None,
    };

    let replies_collection_items = match self.is_external {
      true => self.replies_uri.clone(),
      false => format!("{}?page=0&page_size=20", relative_to_absolute_uri(&self.replies_uri)),
    };

    let replies_collection = Object::builder()
      .kind(Some("OrderedCollection".to_string()))
      .id(Some(self.replies_uri.clone()))
      .collection(Some(
        CollectionProps::builder()
          .items(Some(Reference::Remote(replies_collection_items)))
          .build(),
      ))
      .build();

    Some(
      Object::builder()
        .id(Some(self.uri.clone()))
        .kind(Some(ObjectType::Note.to_string()))
        .url(Some(Reference::Remote(self.uri.clone())))
        .attributed_to(Some(Reference::Remote(attributed_to_uri)))
        .cc(Some(Reference::Mixed(vec![Reference::Remote(cc_uri)])))
        .to(to)
        .content(Some(RdfString::Raw(self.content_html.clone())))
        .replies(Some(Box::new(replies_collection)))
        .source(Some(
          ObjectSource::builder()
            .content(self.content_md.clone())
            .media_type("text/markdown".to_string())
            .build(),
        ))
        .in_reply_to(Some(Reference::Remote(in_reply_to_uri)))
        .published(Some(self.created_at))
        .build(),
    )
  }
}
