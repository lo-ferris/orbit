use uuid::Uuid;

use crate::{
  db::{
    follow_repository::FollowPool, job_repository::JobPool, like_repository::LikePool, orbit_repository::OrbitPool,
    post_attachment_repository::PostAttachmentPool, post_repository::PostPool, user_orbit_repository::UserOrbitPool,
    user_repository::UserPool,
  },
  federation::activitypub::federate_posts_collection,
  logic::LogicErr,
  work_queue::queue::Queue,
};

pub async fn fetch_external_orbit_posts(
  orbits: &OrbitPool,
  users: &UserPool,
  follows: &FollowPool,
  posts: &PostPool,
  likes: &LikePool,
  jobs: &JobPool,
  post_attachments: &PostAttachmentPool,
  user_orbits: &UserOrbitPool,
  job_id: &Uuid,
  queue: &Queue,
) -> Result<(), LogicErr> {
  let job = match jobs.fetch_by_id(job_id).await? {
    Some(job) => job,
    None => return Err(LogicErr::MissingRecord),
  };

  let orbit_id = match job.record_id {
    Some(id) => id,
    None => return Err(LogicErr::MissingRecord),
  };

  let orbit = match orbits.fetch_orbit(&orbit_id).await? {
    Some(orbit) => orbit,
    None => return Err(LogicErr::MissingRecord),
  };

  let collection_uri = match orbit.ext_apub_outbox_uri {
    Some(uri) => uri,
    None => return Err(LogicErr::InvalidData),
  };

  federate_posts_collection(
    &collection_uri,
    users,
    follows,
    posts,
    likes,
    jobs,
    post_attachments,
    orbits,
    user_orbits,
    queue,
  )
  .await
}
