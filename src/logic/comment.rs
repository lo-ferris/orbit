use uuid::Uuid;

use crate::{
  activitypub::{
    activity_convertible::ActivityConvertible, document::ActivityPubDocument,
    helpers::create_activitypub_ordered_collection_page, object::ObjectType,
  },
  db::{
    comment_repository::CommentPool, follow_repository::FollowPool, job_repository::JobPool, post_repository::PostPool,
    tombstone_repository::TombstonePool,
  },
  helpers::math::div_up,
  model::{
    access_type::AccessType,
    comment_pub::CommentPub,
    job::{JobStatus, NewJob},
    queue_job::{QueueJob, QueueJobType},
    response::ListResponse,
  },
  settings::SETTINGS,
  work_queue::queue::Queue,
};

use super::LogicErr;

pub async fn create_comment(
  posts: &PostPool,
  follows: &FollowPool,
  comments: &CommentPool,
  jobs: &JobPool,
  queue: &Queue,
  post_id: &Uuid,
  user_id: &Uuid,
  content_md: &str,
) -> Result<CommentPub, LogicErr> {
  let visibility = match posts.fetch_visibility_by_id(post_id).await {
    Some(visibility) => visibility,
    None => return Err(LogicErr::MissingRecord),
  };

  let owner_id = match posts.fetch_owner_by_id(post_id).await {
    Some(id) => id,
    None => return Err(LogicErr::MissingRecord),
  };

  // If the commenting user doesn't own the post and the post isn't publicly available, don't let the user comment
  if (visibility == AccessType::Private || visibility == AccessType::Shadow) && &owner_id != user_id {
    return Err(LogicErr::UnauthorizedError);
  }

  // If the post is only available to the author's followers and the user isn't a follower of the author, don't let the
  // user comment
  if visibility == AccessType::FollowersOnly && !follows.user_follows_poster(post_id, user_id).await {
    return Err(LogicErr::MissingRecord);
  }

  let content_html = markdown::to_html(content_md);

  let comment_id = comments
    .create_comment(user_id, post_id, content_md, &content_html, false)
    .await?;

  let job_id = jobs
    .create(NewJob {
      created_by_id: Some(user_id.to_owned()),
      status: JobStatus::NotStarted,
      record_id: Some(comment_id.to_owned()),
      associated_record_id: None,
    })
    .await?;

  let job = QueueJob::builder()
    .job_id(job_id)
    .job_type(QueueJobType::CreateComment)
    .build();

  queue.send_job(job).await?;

  match comments
    .fetch_comment(post_id, &comment_id, &Some(user_id.to_owned()))
    .await
  {
    Some(comment) => Ok(comment),
    None => Err(LogicErr::MissingRecord),
  }
}

pub async fn create_comment_like(
  posts: &PostPool,
  follows: &FollowPool,
  comments: &CommentPool,
  post_id: &Uuid,
  comment_id: &Uuid,
  user_id: &Uuid,
) -> Result<(), LogicErr> {
  let visibility = match posts.fetch_visibility_by_id(post_id).await {
    Some(visibility) => visibility,
    None => return Err(LogicErr::MissingRecord),
  };

  let owner_id = match posts.fetch_owner_by_id(post_id).await {
    Some(id) => id,
    None => return Err(LogicErr::MissingRecord),
  };

  // If the commenting user doesn't own the post and the post isn't publicly available, don't let the user comment
  if (visibility == AccessType::Private || visibility == AccessType::Shadow) && &owner_id != user_id {
    return Err(LogicErr::UnauthorizedError);
  }

  // If the post is only available to the author's followers and the user isn't a follower of the author, don't let the
  // user comment
  if visibility == AccessType::FollowersOnly && !follows.user_follows_poster(post_id, user_id).await {
    return Err(LogicErr::MissingRecord);
  }

  comments.create_comment_like(user_id, comment_id, post_id).await
}

pub async fn delete_comment(
  comments: &CommentPool,
  jobs: &JobPool,
  tombstones: &TombstonePool,
  queue: &Queue,
  post_id: &Uuid,
  comment_id: &Uuid,
  user_id: &Uuid,
) -> Result<(), LogicErr> {
  let comment = comments
    .fetch_comment(post_id, comment_id, &Some(user_id.to_owned()))
    .await;

  let comment = match comment {
    Some(comment) => comment,
    None => return Err(LogicErr::MissingRecord),
  };

  if comment.user_id != *user_id {
    return Err(LogicErr::MissingRecord);
  }

  comments.delete_comment(user_id, post_id, comment_id).await?;

  let uri = format!("{}/feed/{}/comments/{}", SETTINGS.server.api_fqdn, post_id, comment_id);

  tombstones.create_tombstone(&uri, &ObjectType::Note.to_string()).await?;

  let job_id = jobs
    .create(NewJob {
      created_by_id: Some(user_id.to_owned()),
      status: JobStatus::NotStarted,
      record_id: Some(comment_id.to_owned()),
      associated_record_id: None,
    })
    .await?;

  let job = QueueJob::builder()
    .job_id(job_id)
    .job_type(QueueJobType::DeleteComment)
    .build();

  queue.send_job(job).await
}

pub async fn delete_comment_like(
  comments: &CommentPool,
  post_id: &Uuid,
  comment_id: &Uuid,
  user_id: &Uuid,
) -> Result<(), LogicErr> {
  comments.delete_comment_like(user_id, comment_id, post_id).await
}

pub async fn get_comments(
  comments: &CommentPool,
  post_id: &Uuid,
  own_user_id: &Option<Uuid>,
  page: &Option<i64>,
  page_size: &Option<i64>,
) -> Result<ListResponse<CommentPub>, LogicErr> {
  let page = page.unwrap_or(0);
  let page_size = page_size.unwrap_or(20);
  let comments_count = match comments.fetch_comments_count(post_id, own_user_id).await {
    Ok(count) => count,
    Err(err) => return Err(err),
  };

  if comments_count == 0 {
    return Err(LogicErr::MissingRecord);
  }

  match comments
    .fetch_comments(post_id, own_user_id, page_size, page * page_size)
    .await
  {
    Ok(comments) => Ok(ListResponse {
      data: comments,
      page,
      total_items: comments_count,
      total_pages: div_up(comments_count, page_size) + 1,
    }),
    Err(err) => Err(err),
  }
}

pub async fn activitypub_get_comments(
  posts: &PostPool,
  comments: &CommentPool,
  post_id: &Uuid,
  own_user_id: &Option<Uuid>,
  page: &Option<i64>,
  page_size: &Option<i64>,
) -> Result<ActivityPubDocument, LogicErr> {
  let author_id = match posts.fetch_owner_by_id(post_id).await {
    Some(h) => h,
    None => return Err(LogicErr::MissingRecord),
  };

  let page = page.unwrap_or(0);
  let page_size = page_size.unwrap_or(20);
  let comments_count = match comments.fetch_comments_count(post_id, own_user_id).await {
    Ok(count) => count,
    Err(err) => return Err(err),
  };

  match comments
    .fetch_comments(post_id, own_user_id, page_size, page * page_size)
    .await
  {
    Ok(comments) => Ok(create_activitypub_ordered_collection_page(
      &format!("{}/feed/{}/comments", SETTINGS.server.api_fqdn, post_id),
      page.try_into().unwrap_or_default(),
      page_size.try_into().unwrap_or_default(),
      comments_count.try_into().unwrap_or_default(),
      comments,
      Some(format!("{}/user/{}", SETTINGS.server.api_fqdn, author_id)),
    )),
    Err(err) => Err(err),
  }
}

pub async fn get_comment(
  comments: &CommentPool,
  post_id: &Uuid,
  comment_id: &Uuid,
  own_user_id: &Option<Uuid>,
) -> Result<CommentPub, LogicErr> {
  match comments.fetch_comment(post_id, comment_id, own_user_id).await {
    Some(comment) => Ok(comment),
    None => Err(LogicErr::MissingRecord),
  }
}

pub async fn activitypub_get_comment(
  comments: &CommentPool,
  posts: &PostPool,
  post_id: &Uuid,
  comment_id: &Uuid,
  own_user_id: &Option<Uuid>,
) -> Result<ActivityPubDocument, LogicErr> {
  let author_id = match posts.fetch_owner_by_id(post_id).await {
    Some(h) => h,
    None => return Err(LogicErr::MissingRecord),
  };

  let actor = format!("{}/user/{}", SETTINGS.server.api_fqdn, author_id);

  match comments.fetch_comment(post_id, comment_id, own_user_id).await {
    Some(comment) => match comment.to_object(&actor) {
      Some(comment) => {
        let comment = ActivityPubDocument::new(comment);
        Ok(comment)
      }
      None => Err(LogicErr::MissingRecord),
    },
    None => Err(LogicErr::MissingRecord),
  }
}
