use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{
  actor::{federate_orbit_group, federate_update_orbit_group, federate_update_user_actor, federate_user_actor},
  article::{
    federate_create_article, federate_ext_create_article, federate_ext_create_article_comment,
    federate_ext_delete_article, federate_ext_delete_article_comment, federate_ext_update_article,
    federate_ext_update_article_comment, federate_update_article,
  },
  group::{federate_create_member, federate_remove_member},
  note::{
    federate_create_note, federate_ext_create_comment_note, federate_ext_create_note, federate_ext_delete_comment_note,
    federate_ext_delete_note, federate_ext_update_comment_note, federate_ext_update_note, federate_like_note,
    federate_unlike_note, federate_update_note,
  },
  object::federate_delete_remote_object,
  person::{
    federate_create_follow, federate_ext_create_follow, federate_ext_join_group, federate_ext_leave_group,
    federate_ext_remove_follow, federate_remove_follow,
  },
  util::{
    activitypub_ref_to_uri_opt, deref_activitypub_ref, deref_activitypub_ref_list, determine_activity_target,
    determine_activity_visibility, send_activitypub_object, ActivityTarget, FederateResult,
  },
};
use crate::{
  activitypub::{
    activity::ActivityProps,
    activity_type::ActivityType,
    document::ActivityPubDocument,
    object::{Object, ObjectType},
    reference::Reference,
  },
  db::{
    comment_repository::CommentPool, follow_repository::FollowPool, job_repository::JobPool, like_repository::LikePool,
    orbit_repository::OrbitPool, post_attachment_repository::PostAttachmentPool, post_repository::PostPool,
    user_orbit_repository::UserOrbitPool, user_repository::UserPool,
  },
  helpers::core::unwrap_or_fail,
  logic::LogicErr,
  model::{orbit::Orbit, queue_job::OriginDataEntry, user::User},
  net::http_sig::verify_http_signature,
  settings::SETTINGS,
  work_queue::queue::Queue,
};

use std::{collections::HashMap, str::FromStr};

pub enum FederateActorResult {
  User(User),
  Group(Orbit),
}

async fn federate_get_actor(
  doc: &ActivityPubDocument,
  users: &UserPool,
  orbits: &OrbitPool,
  origin_data: &Option<HashMap<String, OriginDataEntry>>,
) -> Result<FederateActorResult, LogicErr> {
  match federate_get_actor_user(doc, users, origin_data).await {
    Ok(user) => Ok(FederateActorResult::User(user)),
    Err(_) => federate_get_actor_group(doc, orbits, origin_data)
      .await
      .map(FederateActorResult::Group),
  }
}

async fn federate_get_actor_user(
  doc: &ActivityPubDocument,
  users: &UserPool,
  origin_data: &Option<HashMap<String, OriginDataEntry>>,
) -> Result<User, LogicErr> {
  let mut actor_user = match federate_user_actor(&doc.object.actor, users).await {
    Ok(user) => user,
    Err(err) => return Err(err),
  };

  if SETTINGS.app.secure && !verify_http_signature(origin_data, &actor_user.public_key) {
    actor_user = federate_update_user_actor(&doc.object.actor, users).await?;
    if !verify_http_signature(origin_data, &actor_user.public_key) {
      return Err(LogicErr::UnauthorizedError);
    }
  }

  Ok(actor_user)
}

async fn federate_get_actor_group(
  doc: &ActivityPubDocument,
  orbits: &OrbitPool,
  origin_data: &Option<HashMap<String, OriginDataEntry>>,
) -> Result<Orbit, LogicErr> {
  let mut actor_orbit = match federate_orbit_group(&doc.object.actor, orbits).await {
    Ok(user) => user,
    Err(err) => return Err(err),
  };

  if SETTINGS.app.secure && !verify_http_signature(origin_data, &actor_orbit.public_key) {
    actor_orbit = federate_update_orbit_group(&doc.object.actor, orbits).await?;
    if !verify_http_signature(origin_data, &actor_orbit.public_key) {
      return Err(LogicErr::UnauthorizedError);
    }
  }

  Ok(actor_orbit)
}

pub async fn federate(
  doc: ActivityPubDocument,
  origin_data: &Option<HashMap<String, OriginDataEntry>>,
  users: &UserPool,
  follows: &FollowPool,
  posts: &PostPool,
  likes: &LikePool,
  jobs: &JobPool,
  post_attachments: &PostAttachmentPool,
  orbits: &OrbitPool,
  user_orbits: &UserOrbitPool,
  queue: &Queue,
) -> Result<(), LogicErr> {
  let kind = match unwrap_or_fail(doc.object.kind.as_ref().map(|v| ActivityType::from_str(v))) {
    Ok(kind) => kind,
    Err(err) => return Err(err),
  };

  let actor = federate_get_actor(&doc, users, orbits, origin_data).await?;

  let actor_user = match actor {
    FederateActorResult::User(user) => user,
    FederateActorResult::Group(_) => {
      // TODO: Support activities sent from a group
      return Ok(());
    }
  };

  let activity = match &doc.object.activity {
    Some(ac) => ac,
    None => return Err(LogicErr::InvalidData),
  };

  let object = match deref_activitypub_ref(&activity.object).await {
    Some(obj) => obj,
    None => return Err(LogicErr::InvalidData),
  };

  let activity_visibility = determine_activity_visibility(&object.to, &actor_user);

  let target = activitypub_ref_to_uri_opt(&activity.target);

  if let Some(Ok(nested_activity_type)) = object.kind.as_ref().map(|v| ActivityType::from_str(v)) {
    log::warn!(
      "Unimplemented federation task: Activity Type {} on nested Activity Type: {}",
      kind,
      nested_activity_type
    );
    return Ok(());
  };

  let object_type = match &object.kind {
    Some(v) => match ObjectType::from_str(v) {
      Ok(t) => t,
      Err(_) => return Err(LogicErr::InvalidData),
    },
    None => return Err(LogicErr::InvalidData),
  };

  let result = match object_type {
    ObjectType::Note => match kind {
      ActivityType::Create => {
        let activity_visibility = match activity_visibility {
          Some(v) => v,
          None => return Err(LogicErr::InvalidData),
        };

        federate_create_note(
          object,
          &actor_user,
          activity_visibility,
          follows,
          posts,
          jobs,
          post_attachments,
          queue,
        )
        .await
      }
      ActivityType::Update => {
        let activity_visibility = match activity_visibility {
          Some(v) => v,
          None => return Err(LogicErr::InvalidData),
        };

        federate_update_note(object, &actor_user, activity_visibility, posts).await
      }
      ActivityType::Like => federate_like_note(object, &actor_user, posts, likes).await,
      ActivityType::Remove => match determine_activity_target(target) {
        ActivityTarget::PostLikes(target) => federate_unlike_note(target, &actor_user, posts, likes).await,
        ActivityTarget::Unknown(target) => {
          federate_delete_remote_object(target, &actor_user, object_type, origin_data, posts, users, likes).await
        }
        _ => Err(LogicErr::InvalidData),
      },
      ActivityType::Delete => match determine_activity_target(target) {
        ActivityTarget::PostLikes(target) => federate_unlike_note(target, &actor_user, posts, likes).await,
        ActivityTarget::Unknown(target) => {
          federate_delete_remote_object(target, &actor_user, object_type, origin_data, posts, users, likes).await
        }
        _ => Err(LogicErr::InvalidData),
      },
      _ => {
        log::warn!(
          "Unimplemented federation task: Activity Type: {} on Object Type {}",
          kind,
          object_type
        );
        return Ok(());
      }
    },
    ObjectType::Article => match kind {
      ActivityType::Create => {
        federate_create_article(object, &actor_user, posts, jobs, post_attachments, orbits, queue).await
      }
      ActivityType::Update => {
        let activity_visibility = match activity_visibility {
          Some(v) => v,
          None => return Err(LogicErr::InvalidData),
        };

        federate_update_article(object, &actor_user, activity_visibility, posts).await
      }
      ActivityType::Remove => match determine_activity_target(target) {
        ActivityTarget::OrbitMembers(target) => federate_remove_member(target, &actor_user, user_orbits, orbits).await,
        ActivityTarget::Unknown(target) => {
          federate_delete_remote_object(target, &actor_user, object_type, origin_data, posts, users, likes).await
        }
        _ => Err(LogicErr::InvalidData),
      },
      ActivityType::Delete => match determine_activity_target(target) {
        ActivityTarget::OrbitMembers(target) => federate_remove_member(target, &actor_user, user_orbits, orbits).await,
        ActivityTarget::Unknown(target) => {
          federate_delete_remote_object(target, &actor_user, object_type, origin_data, posts, users, likes).await
        }
        _ => Err(LogicErr::InvalidData),
      },
      _ => Err(LogicErr::InternalError("Unimplemented".to_string())),
    },
    ObjectType::Person => match kind {
      ActivityType::Follow => federate_create_follow(object, &actor_user, follows, users).await,
      ActivityType::Remove => match determine_activity_target(target) {
        ActivityTarget::UserFollowers(target) => federate_remove_follow(target, &actor_user, follows, users).await,
        ActivityTarget::Unknown(target) => {
          federate_delete_remote_object(target, &actor_user, object_type, origin_data, posts, users, likes).await
        }
        _ => Err(LogicErr::InvalidData),
      },
      ActivityType::Delete => match determine_activity_target(target) {
        ActivityTarget::UserFollowers(target) => federate_remove_follow(target, &actor_user, follows, users).await,
        ActivityTarget::Unknown(target) => {
          federate_delete_remote_object(target, &actor_user, object_type, origin_data, posts, users, likes).await
        }
        _ => Err(LogicErr::InvalidData),
      },
      _ => {
        log::warn!(
          "Unimplemented federation task: Activity Type: {} on Object Type {}",
          kind,
          object_type
        );
        return Ok(());
      }
    },
    ObjectType::Tombstone => match object.id {
      Some(id) => match id.starts_with(&SETTINGS.server.api_root_fqdn) {
        true => match determine_activity_target(Some(id)) {
          ActivityTarget::UserFollowers(target) => federate_remove_follow(target, &actor_user, follows, users).await,
          ActivityTarget::OrbitMembers(target) => {
            federate_remove_member(target, &actor_user, user_orbits, orbits).await
          }
          ActivityTarget::PostLikes(target) => federate_unlike_note(target, &actor_user, posts, likes).await,
          _ => {
            log::warn!(
              "Unimplemented federation task: Activity Type: {} on Object Type {}",
              kind,
              object_type
            );
            return Ok(());
          }
        },
        false => federate_delete_remote_object(id, &actor_user, object_type, origin_data, posts, users, likes).await,
      },
      None => Err(LogicErr::InvalidData),
    },
    ObjectType::Group => match kind {
      ActivityType::Follow => federate_create_member(object, &actor_user, user_orbits, orbits).await,
      _ => {
        log::warn!(
          "Unimplemented federation task: Activity Type: {} on Object Type {}",
          kind,
          object_type
        );
        return Ok(());
      }
    },
    _ => {
      log::warn!(
        "Unimplemented federation task: Activity Type: {} on Object Type {}",
        kind,
        object_type
      );
      return Ok(());
    }
  };

  match result {
    Ok(result) => {
      let (activity_type, actor_fediverse_uri, actor_private_key) = match result {
        FederateResult::None => return Ok(()),
        FederateResult::Accept(actor) => (ActivityType::Accept, actor.0, actor.1),
        FederateResult::TentativeAccept(actor) => (ActivityType::TentativeAccept, actor.0, actor.1),
        FederateResult::Ignore(actor) => (ActivityType::Ignore, actor.0, actor.1),
        FederateResult::Reject(actor) => (ActivityType::Reject, actor.0, actor.1),
        FederateResult::TentativeReject(actor) => (ActivityType::TentativeReject, actor.0, actor.1),
      };

      let response_object = Object::builder()
        .kind(Some(activity_type.to_string()))
        .id(Some(format!("{}/{}", SETTINGS.server.api_fqdn, Uuid::new_v4())))
        .actor(Some(Reference::Remote(format!(
          "{}{}",
          SETTINGS.server.api_fqdn, actor_fediverse_uri
        ))))
        .activity(Some(
          ActivityProps::builder()
            .object(Some(Reference::Embedded(Box::new(doc.object))))
            .build(),
        ))
        .build();

      let doc = ActivityPubDocument::new(response_object);

      let response_uri = match actor_user.ext_apub_inbox_uri {
        Some(uri) => uri,
        None => return Ok(()),
      };

      send_activitypub_object(&response_uri, doc, &actor_fediverse_uri, &actor_private_key).await
    }
    Err(err) => Err(err),
  }
}

pub async fn federate_posts_collection(
  uri: &str,
  users: &UserPool,
  _follows: &FollowPool,
  posts: &PostPool,
  _likes: &LikePool,
  jobs: &JobPool,
  post_attachments: &PostAttachmentPool,
  orbits: &OrbitPool,
  _user_orbits: &UserOrbitPool,
  queue: &Queue,
) -> Result<(), LogicErr> {
  let obj = match deref_activitypub_ref(&Some(Reference::Remote(uri.to_owned()))).await {
    Some(obj) => obj,
    None => return Err(LogicErr::MissingRecord),
  };

  if obj.kind != Some(ObjectType::OrderedCollectionPage.to_string()) {
    return Err(LogicErr::InvalidData);
  }

  let items = match obj.collection {
    Some(props) => match deref_activitypub_ref_list(&props.ordered_items).await {
      Some(items) => items,
      None => return Err(LogicErr::InvalidData),
    },
    None => return Err(LogicErr::InvalidData),
  };

  for item in items {
    let kind = match unwrap_or_fail(item.kind.as_ref().map(|v| ActivityType::from_str(v))) {
      Ok(kind) => kind,
      Err(err) => {
        log::warn!("Failed to federate article from collection {}: {:?}", uri, err);
        continue;
      }
    };

    if kind != ActivityType::Create {
      continue;
    }

    let actor_user = federate_user_actor(&item.actor, users).await?;

    let activity = match &item.activity {
      Some(ac) => ac,
      None => continue,
    };

    let object = match deref_activitypub_ref(&activity.object).await {
      Some(obj) => obj,
      None => continue,
    };

    if let Some(Ok(_)) = object.kind.as_ref().map(|v| ActivityType::from_str(v)) {
      continue;
    };

    let object_type = match &object.kind {
      Some(v) => match ObjectType::from_str(v) {
        Ok(t) => t,
        Err(err) => {
          log::warn!("Failed to federate article from collection {}: {:?}", uri, err);
          continue;
        }
      },
      None => continue,
    };

    if object_type == ObjectType::Note {
      // TODO: Figure out how to federate user posts that don't go to this orbit instance's users
      continue;
    } else if object_type == ObjectType::Article {
      if let Err(err) = federate_create_article(object, &actor_user, posts, jobs, post_attachments, orbits, queue).await
      {
        log::warn!("Failed to federate article from collection {}: {:?}", uri, err);
      }
    } else {
      continue;
    }
  }

  Ok(())
}

#[derive(Serialize, Deserialize, Clone)]
pub enum FederateExtAction {
  CreatePost(Uuid),
  UpdatePost(Uuid),
  DeletePost(Uuid),
  CreateComment(Uuid),
  UpdateComment(Uuid),
  DeleteComment(Uuid, Uuid),
  FollowProfile,
  UnfollowProfile,
  FollowGroup(Uuid),
  UnfollowGroup(Uuid),
}

#[derive(Serialize, Deserialize)]
pub enum FederateExtActor {
  None,
  Person(User),
  Group(Orbit),
}

#[derive(Serialize, Deserialize)]
pub enum FederateExtActorRef {
  None,
  Person(Uuid),
  Group(Uuid),
}

pub async fn federate_ext(
  action: FederateExtAction,
  actor: &User,
  dest_actor: &FederateExtActor,
  posts: &PostPool,
  orbits: &OrbitPool,
  comments: &CommentPool,
) -> Result<(), LogicErr> {
  match action {
    FederateExtAction::CreatePost(post_id) => match dest_actor {
      FederateExtActor::Person(dest_actor) => federate_ext_create_note(&post_id, actor, dest_actor, posts).await,
      FederateExtActor::Group(dest_actor) => federate_ext_create_article(&post_id, actor, dest_actor, posts).await,
      FederateExtActor::None => Ok(()),
    },
    FederateExtAction::UpdatePost(post_id) => match dest_actor {
      FederateExtActor::Person(dest_actor) => federate_ext_update_note(&post_id, actor, dest_actor, posts).await,
      FederateExtActor::Group(dest_actor) => federate_ext_update_article(&post_id, actor, dest_actor, posts).await,
      FederateExtActor::None => Ok(()),
    },
    FederateExtAction::DeletePost(post_id) => match dest_actor {
      FederateExtActor::Person(dest_actor) => federate_ext_delete_note(&post_id, actor, dest_actor).await,
      FederateExtActor::Group(dest_actor) => federate_ext_delete_article(&post_id, actor, dest_actor).await,
      FederateExtActor::None => Ok(()),
    },
    FederateExtAction::FollowProfile => federate_ext_create_follow(actor, dest_actor).await,
    FederateExtAction::UnfollowProfile => federate_ext_remove_follow(actor, dest_actor).await,
    FederateExtAction::FollowGroup(group_id) => federate_ext_join_group(actor, &group_id, orbits).await,
    FederateExtAction::UnfollowGroup(group_id) => federate_ext_leave_group(actor, &group_id, orbits).await,
    FederateExtAction::CreateComment(comment_id) => match dest_actor {
      FederateExtActor::Person(dest_actor) => {
        federate_ext_create_comment_note(&comment_id, actor, dest_actor, comments).await
      }
      FederateExtActor::Group(dest_actor) => {
        federate_ext_create_article_comment(&comment_id, actor, dest_actor, comments).await
      }
      FederateExtActor::None => Ok(()),
    },
    FederateExtAction::UpdateComment(comment_id) => match dest_actor {
      FederateExtActor::Person(dest_actor) => {
        federate_ext_update_comment_note(&comment_id, actor, dest_actor, comments).await
      }
      FederateExtActor::Group(dest_actor) => {
        federate_ext_update_article_comment(&comment_id, actor, dest_actor, comments).await
      }
      FederateExtActor::None => Ok(()),
    },
    FederateExtAction::DeleteComment(post_id, comment_id) => match dest_actor {
      FederateExtActor::Person(dest_actor) => {
        federate_ext_delete_comment_note(&post_id, &comment_id, actor, dest_actor).await
      }
      FederateExtActor::Group(dest_actor) => {
        federate_ext_delete_article_comment(&post_id, &comment_id, actor, dest_actor).await
      }
      FederateExtActor::None => Ok(()),
    },
  }
}
