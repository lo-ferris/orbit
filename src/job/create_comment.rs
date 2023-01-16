use uuid::Uuid;

use crate::{
  db::{
    comment_repository::CommentPool, follow_repository::FollowPool, job_repository::JobPool,
    orbit_repository::OrbitPool, post_repository::PostPool, user_orbit_repository::UserOrbitPool,
    user_repository::UserPool,
  },
  federation::activitypub::{FederateExtAction, FederateExtActorRef},
  helpers::api::map_db_err,
  logic::LogicErr,
  model::{
    job::{JobStatus, NewJob},
    queue_job::{QueueJob, QueueJobType},
  },
  work_queue::queue::Queue,
};

pub async fn create_comment(
  job_id: Uuid,
  jobs: &JobPool,
  orbits: &OrbitPool,
  user_orbits: &UserOrbitPool,
  users: &UserPool,
  posts: &PostPool,
  comments: &CommentPool,
  follows: &FollowPool,
  queue: &Queue,
) -> Result<(), LogicErr> {
  let job = match jobs.fetch_optional_by_id(&job_id).await {
    Some(job) => job,
    None => return Err(LogicErr::InternalError("Job not found".to_string())),
  };

  let user_id = match job.created_by_id {
    Some(id) => id,
    None => return Err(LogicErr::InternalError("User not found".to_string())),
  };

  let comment_id = match job.record_id {
    Some(id) => id,
    None => return Err(LogicErr::InternalError("Post ID not found for job".to_string())),
  };

  let comment = match comments.fetch_comment_by_id(&comment_id).await {
    Some(comment) => comment,
    None => return Err(LogicErr::InternalError("Comment not found for job".to_string())),
  };

  let post = posts.fetch_by_id(&comment.post_id).await?;

  if !post.is_external {
    log::warn!(
      "Post {} is not external, this is a bug, skipping as no federation needed",
      post.post_id
    );
    return Ok(());
  }

  let post_author = users.fetch_by_id(&post.user_id).await?;

  if !post_author.is_external {
    log::warn!(
      "Post {} author {} is not external, this is a bug, skipping as no federation needed",
      post.post_id,
      post.user_id
    );
    return Ok(());
  }

  let job = QueueJob::builder()
    .job_id(job_id)
    .job_type(QueueJobType::FederateActivityPubExt)
    .activitypub_federate_ext_action(FederateExtAction::CreateComment(comment_id))
    .activitypub_federate_ext_dest_actor(FederateExtActorRef::Person(post_author.user_id))
    .build();

  queue.send_job(job).await?;

  if let Some(orbit_id) = post.orbit_id {
    let orbit = match orbits.fetch_orbit(&orbit_id).await? {
      Some(orbit) => orbit,
      None => return Err(LogicErr::InternalError("Orbit not found for post".to_string())),
    };

    if orbit.is_external {
      let job = QueueJob::builder()
        .job_id(job_id)
        .job_type(QueueJobType::FederateActivityPubExt)
        .activitypub_federate_ext_action(FederateExtAction::CreateComment(comment_id))
        .activitypub_federate_ext_dest_actor(FederateExtActorRef::Group(orbit.orbit_id))
        .build();

      queue.send_job(job).await?;
    } else {
      let users = user_orbits.fetch_orbit_external_user_ids(&orbit_id).await?;

      for user in users {
        let job_id = jobs
          .create(NewJob {
            created_by_id: Some(user_id),
            status: JobStatus::NotStarted,
            record_id: Some(comment_id),
            associated_record_id: Some(user),
          })
          .await
          .map_err(map_db_err)?;

        let job = QueueJob::builder()
          .job_id(job_id)
          .job_type(QueueJobType::FederateActivityPubExt)
          .activitypub_federate_ext_action(FederateExtAction::CreateComment(comment_id))
          .activitypub_federate_ext_dest_actor(FederateExtActorRef::Person(user))
          .build();

        queue.send_job(job).await?;
      }
    }
  } else {
    let followers = follows
      .fetch_user_external_follower_ids(&user_id)
      .await
      .unwrap_or_default();

    for user in followers {
      let job_id = jobs
        .create(NewJob {
          created_by_id: Some(user_id),
          status: JobStatus::NotStarted,
          record_id: Some(comment_id),
          associated_record_id: Some(user),
        })
        .await
        .map_err(map_db_err)?;

      let job = QueueJob::builder()
        .job_id(job_id)
        .job_type(QueueJobType::FederateActivityPubExt)
        .activitypub_federate_ext_action(FederateExtAction::CreateComment(comment_id))
        .activitypub_federate_ext_dest_actor(FederateExtActorRef::Person(user))
        .build();

      queue.send_job(job).await?;
    }
  }

  Ok(())
}
