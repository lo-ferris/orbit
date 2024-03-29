use crate::{
  cdn::cdn_store::Cdn,
  db::repositories::Repositories,
  logic::LogicErr,
  model::queue_job::{QueueJob, QueueJobType},
  work_queue::queue::Queue,
};

mod clean_jobs;
mod convert_new_post_images;
mod create_boost_event;
mod create_boost_events;
mod create_post_event;
mod create_post_events;
mod delete_boost_events;
mod delete_post;
mod federate_activitypub;
mod federate_activitypub_ext;
mod fetch_external_orbit_posts;
mod refresh_external_orbit;
mod refresh_external_orbits;
mod refresh_external_profile;
mod refresh_external_profiles;
mod update_post;

pub async fn delegate_job(
  queue_job: &QueueJob,
  repositories: &Repositories,
  cdn: &Cdn,
  queue: &Queue,
) -> Result<(), LogicErr> {
  match queue_job.job_type {
    QueueJobType::ConvertNewPostImages => {
      convert_new_post_images::convert_new_post_images(
        queue_job.job_id,
        &repositories.jobs,
        &repositories.post_attachments,
        cdn,
      )
      .await
    }
    QueueJobType::CreatePostEvents => {
      create_post_events::create_post_events(
        &repositories.jobs,
        &repositories.posts,
        &repositories.events,
        &repositories.follows,
        &repositories.user_orbits,
        &repositories.orbits,
        &repositories.users,
        queue_job.job_id,
        queue,
      )
      .await
    }
    QueueJobType::CreatePostEvent => {
      create_post_event::create_post_event(
        &repositories.jobs,
        &repositories.posts,
        &repositories.events,
        &repositories.users,
        &repositories.orbits,
        queue_job.job_id,
      )
      .await
    }
    QueueJobType::CreateBoostEvents => {
      create_boost_events::create_boost_events(&repositories.jobs, &repositories.follows, queue_job.job_id, queue).await
    }
    QueueJobType::CreateBoostEvent => {
      create_boost_event::create_boost_event(
        queue_job.job_id,
        &repositories.jobs,
        &repositories.posts,
        &repositories.events,
      )
      .await
    }
    QueueJobType::DeleteBoostEvents => {
      delete_boost_events::delete_boost_events(queue_job.job_id, &repositories.jobs, &repositories.events).await
    }
    QueueJobType::DeletePost => {
      delete_post::delete_post(
        queue_job.job_id,
        &repositories.jobs,
        &repositories.orbits,
        &repositories.user_orbits,
        &repositories.users,
        &repositories.posts,
        &repositories.follows,
        queue,
      )
      .await
    }
    QueueJobType::UpdatePost => {
      update_post::update_post(
        queue_job.job_id,
        &repositories.jobs,
        &repositories.orbits,
        &repositories.user_orbits,
        &repositories.users,
        &repositories.posts,
        &repositories.follows,
        queue,
      )
      .await
    }
    QueueJobType::FederateActivityPub => {
      federate_activitypub::federate_activitypub(&queue_job.data, &queue_job.origin_data, repositories, queue).await
    }
    QueueJobType::FederateActivityPubExt => {
      federate_activitypub_ext::federate_activitypub(
        &queue_job.context,
        &queue_job.activitypub_federate_ext_action,
        &queue_job.activitypub_federate_ext_dest_actor,
        repositories,
      )
      .await
    }
    QueueJobType::FetchExternalOrbitPosts => {
      fetch_external_orbit_posts::fetch_external_orbit_posts(
        &repositories.orbits,
        &repositories.users,
        &repositories.follows,
        &repositories.posts,
        &repositories.likes,
        &repositories.jobs,
        &repositories.post_attachments,
        &repositories.user_orbits,
        &queue_job.job_id,
        queue,
      )
      .await
    }
    QueueJobType::CleanJobs => clean_jobs::clean_jobs(&repositories.jobs).await,
    QueueJobType::RefreshExternalOrbits => {
      refresh_external_orbits::refresh_external_orbits(&repositories.orbits, &repositories.jobs, queue).await
    }
    QueueJobType::RefreshExternalProfiles => {
      refresh_external_profiles::refresh_external_profiles(&repositories.users, &repositories.jobs, queue).await
    }
    QueueJobType::RefreshExternalOrbit => {
      refresh_external_orbit::refresh_external_orbit(&repositories.orbits, &repositories.jobs, queue_job.job_id).await
    }
    QueueJobType::RefreshExternalProfile => {
      refresh_external_profile::refresh_external_profile(&repositories.users, &repositories.jobs, queue_job.job_id)
        .await
    }
    QueueJobType::Unknown => Err(LogicErr::Unimplemented),
  }
}
