use actix_easy_multipart::{tempfile::Tempfile, MultipartForm};
use actix_web::{web, HttpResponse, Responder};
use rsa::{
  pkcs1::EncodeRsaPrivateKey,
  pkcs8::{EncodePublicKey, LineEnding},
  RsaPrivateKey, RsaPublicKey,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
  activitypub::object::ObjectType,
  cdn::cdn_store::Cdn,
  db::{
    job_repository::JobPool, orbit_moderator_repository::OrbitModeratorPool, orbit_repository::OrbitPool,
    session_repository::SessionPool, tombstone_repository::TombstonePool, user_orbit_repository::UserOrbitPool,
    user_repository::UserPool,
  },
  federation::activitypub::{FederateExtAction, FederateExtActorRef},
  helpers::{
    api::map_db_err,
    auth::{assert_auth, query_auth, require_auth},
    core::{build_api_err, build_api_not_found, map_api_err},
    math::div_up,
  },
  model::{
    job::{JobStatus, NewJob},
    queue_job::{QueueJob, QueueJobType},
    response::{ListResponse, ObjectResponse},
    user_account_pub::UserAccountPub,
  },
  net::jwt::JwtContext,
  work_queue::queue::Queue,
};

#[derive(Deserialize)]
pub struct NewOrbitRequest {
  pub name: String,
  pub description_md: String,
  pub shortcode: Option<String>,
}

#[derive(Serialize)]
pub struct NewOrbitResponse {
  pub id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct OrbitsQuery {
  pub page: Option<i64>,
  pub page_size: Option<i64>,
}

#[derive(Deserialize)]
pub struct NewOrbitModeratorRequest {
  pub user_id: Uuid,
  pub is_owner: Option<bool>,
}

#[derive(Deserialize)]
pub struct MembersQuery {
  pub page: Option<i64>,
  pub page_size: Option<i64>,
}

#[derive(MultipartForm)]
pub struct OrbitAssetsUpload {
  #[multipart(rename = "images[]")]
  images: Vec<Tempfile>,
}

pub async fn api_get_user_orbits(
  orbits: web::Data<OrbitPool>,
  query: web::Query<OrbitsQuery>,
  handle: web::Path<String>,
  sessions: web::Data<SessionPool>,
  users: web::Data<UserPool>,
  jwt: web::ReqData<JwtContext>,
) -> impl Responder {
  match assert_auth(&jwt, &sessions).await {
    Ok(_) => {}
    Err(res) => return res,
  };

  let user_id = match users.fetch_id_by_handle(&handle).await {
    Some(user_id) => user_id,
    None => return build_api_not_found(handle.to_string()),
  };

  let page = query.page.unwrap_or(0);
  let page_size = query.page_size.unwrap_or(20);
  let posts_count = match orbits.count_user_orbits(&user_id).await {
    Ok(count) => count,
    Err(err) => return build_api_err(500, err.to_string(), Some(err.to_string())),
  };

  let orbits = match orbits.fetch_user_orbits(&user_id, page_size, page * page_size).await {
    Ok(posts) => posts,
    Err(err) => return build_api_err(500, err.to_string(), Some(err.to_string())),
  };

  HttpResponse::Ok().json(ListResponse {
    data: orbits,
    page,
    total_items: posts_count,
    total_pages: div_up(posts_count, page_size) + 1,
  })
}

pub async fn api_get_popular_orbits(orbits: web::Data<OrbitPool>) -> impl Responder {
  let orbits = match orbits.fetch_popular_orbits().await {
    Ok(posts) => posts,
    Err(err) => return build_api_err(500, err.to_string(), Some(err.to_string())),
  };

  let total_items: i64 = orbits.len().try_into().unwrap_or_default();

  HttpResponse::Ok().json(ListResponse {
    data: orbits,
    page: 0,
    total_items,
    total_pages: 1,
  })
}

pub async fn api_get_orbits(orbits: web::Data<OrbitPool>, query: web::Query<OrbitsQuery>) -> impl Responder {
  let page = query.page.unwrap_or(0);
  let page_size = query.page_size.unwrap_or(20);
  let posts_count = match orbits.count_orbits().await {
    Ok(count) => count,
    Err(err) => return build_api_err(500, err.to_string(), Some(err.to_string())),
  };

  let orbits = match orbits.fetch_orbits(page_size, page * page_size).await {
    Ok(posts) => posts,
    Err(err) => return build_api_err(500, err.to_string(), Some(err.to_string())),
  };

  HttpResponse::Ok().json(ListResponse {
    data: orbits,
    page,
    total_items: posts_count,
    total_pages: div_up(posts_count, page_size) + 1,
  })
}

pub async fn api_get_orbit(
  orbits: web::Data<OrbitPool>,
  orbit_id: web::Path<Uuid>,
  sessions: web::Data<SessionPool>,
  jwt: web::ReqData<JwtContext>,
) -> impl Responder {
  let user_id = match query_auth(&jwt, &sessions).await {
    Some(props) => Some(props.uid),
    None => None,
  };

  let orbit = match orbits.fetch_orbit_for_user(&orbit_id, &user_id).await {
    Ok(orbit) => orbit,
    Err(err) => return build_api_err(500, err.to_string(), Some(err.to_string())),
  };

  let orbit = match orbit {
    Some(orbit) => orbit,
    None => return build_api_not_found(orbit_id.to_string()),
  };

  HttpResponse::Ok().json(ObjectResponse { data: orbit })
}

pub async fn api_get_orbit_named(
  orbits: web::Data<OrbitPool>,
  orbit_shortcode: web::Path<String>,
  sessions: web::Data<SessionPool>,
  jwt: web::ReqData<JwtContext>,
) -> impl Responder {
  let user_id = match query_auth(&jwt, &sessions).await {
    Some(props) => Some(props.uid),
    None => None,
  };

  let orbit_id = match orbits.fetch_orbit_id_from_shortcode(&orbit_shortcode).await {
    Some(id) => id,
    None => match orbits.fetch_orbit_id_from_fediverse_id(&orbit_shortcode).await {
      Some(id) => id,
      None => return build_api_not_found(orbit_shortcode.to_owned()),
    },
  };

  let orbit = match orbits.fetch_orbit_for_user(&orbit_id, &user_id).await {
    Ok(orbit) => orbit,
    Err(err) => return build_api_err(500, err.to_string(), Some(err.to_string())),
  };

  let orbit = match orbit {
    Some(orbit) => orbit,
    None => return build_api_not_found(orbit_id.to_string()),
  };

  HttpResponse::Ok().json(ObjectResponse { data: orbit })
}

pub async fn api_create_orbit(
  sessions: web::Data<SessionPool>,
  orbits: web::Data<OrbitPool>,
  orbit_moderators: web::Data<OrbitModeratorPool>,
  user_orbits: web::Data<UserOrbitPool>,
  req: web::Json<NewOrbitRequest>,
  jwt: web::ReqData<JwtContext>,
) -> impl Responder {
  let session = match require_auth(&jwt, &sessions).await {
    Ok(session) => session,
    Err(res) => return res,
  };

  let description_html = markdown::to_html(&req.description_md);
  let shortcode = req.shortcode.clone().unwrap_or_else(|| {
    req
      .name
      .clone()
      .replace(|c: char| !c.is_ascii_alphabetic() && !c.is_whitespace(), "")
      .to_ascii_lowercase()
  });

  if shortcode.is_empty() {
    return build_api_err(400, "shortcode".to_string(), None);
  }

  let uri = format!("/orbits/{}", shortcode);

  let mut rng = rand::thread_rng();
  let bits = 2048;
  let priv_key = match RsaPrivateKey::new(&mut rng, bits) {
    Ok(key) => key,
    Err(err) => return build_api_err(500, err.to_string(), Some(err.to_string())),
  };
  let pub_key = RsaPublicKey::from(&priv_key);

  let priv_key = match priv_key.to_pkcs1_pem(LineEnding::LF) {
    Ok(key) => key.to_string(),
    Err(err) => return build_api_err(500, err.to_string(), Some(err.to_string())),
  };

  let pub_key = match pub_key.to_public_key_pem(LineEnding::LF) {
    Ok(key) => key.to_string(),
    Err(err) => return build_api_err(500, err.to_string(), Some(err.to_string())),
  };

  let orbit_id = match orbits
    .create_orbit(
      &req.name,
      &shortcode,
      &req.description_md,
      &description_html,
      &None,
      &None,
      false,
      &priv_key,
      &pub_key,
      &uri,
    )
    .await
  {
    Ok(orbit_id) => orbit_id,
    Err(err) => return build_api_err(500, err.to_string(), Some(err.to_string())),
  };

  match orbit_moderators
    .create_orbit_moderator(&orbit_id, &session.uid, true)
    .await
  {
    Ok(_) => {}
    Err(err) => return build_api_err(500, err.to_string(), Some(err.to_string())),
  };

  match user_orbits.create_user_orbit(&orbit_id, &session.uid).await {
    Ok(_) => {}
    Err(err) => return build_api_err(500, err.to_string(), Some(err.to_string())),
  };

  HttpResponse::Ok().json(NewOrbitResponse { id: orbit_id })
}

pub async fn api_update_orbit(
  sessions: web::Data<SessionPool>,
  orbits: web::Data<OrbitPool>,
  orbit_moderators: web::Data<OrbitModeratorPool>,
  req: web::Json<NewOrbitRequest>,
  orbit_id: web::Path<Uuid>,
  jwt: web::ReqData<JwtContext>,
) -> impl Responder {
  let session = match require_auth(&jwt, &sessions).await {
    Ok(session) => session,
    Err(res) => return res,
  };

  match orbit_moderators.user_is_moderator(&orbit_id, &session.uid).await {
    Ok(is_moderator) => {
      if !is_moderator {
        return build_api_not_found(session.uid.to_string());
      }
    }
    Err(err) => return build_api_err(500, err.to_string(), Some(err.to_string())),
  };

  let description_html = markdown::to_html(&req.description_md);

  let orbit = match orbits.fetch_orbit(&orbit_id).await {
    Ok(orbit) => match orbit {
      Some(orbit) => orbit,
      None => return build_api_not_found(orbit_id.to_string()),
    },
    Err(err) => return build_api_err(500, err.to_string(), Some(err.to_string())),
  };

  match orbits
    .update_orbit(
      &orbit_id,
      &req.name,
      &req.description_md,
      &description_html,
      &orbit.avatar_uri,
      &orbit.banner_uri,
      false,
    )
    .await
  {
    Ok(_) => HttpResponse::Ok().finish(),
    Err(err) => build_api_err(500, err.to_string(), Some(err.to_string())),
  }
}

pub async fn api_update_orbit_assets(
  sessions: web::Data<SessionPool>,
  orbits: web::Data<OrbitPool>,
  orbit_moderators: web::Data<OrbitModeratorPool>,
  cdn: web::Data<Cdn>,
  form: MultipartForm<OrbitAssetsUpload>,
  orbit_id: web::Path<Uuid>,
  jwt: web::ReqData<JwtContext>,
) -> impl Responder {
  if form.images.len() != 2 {
    return build_api_err(400, "Invalid image count".to_string(), None);
  }

  let session = match require_auth(&jwt, &sessions).await {
    Ok(session) => session,
    Err(res) => return res,
  };

  match orbit_moderators.user_is_moderator(&orbit_id, &session.uid).await {
    Ok(is_moderator) => {
      if !is_moderator {
        return build_api_not_found(session.uid.to_string());
      }
    }
    Err(err) => return build_api_err(500, err.to_string(), Some(err.to_string())),
  };

  let orbit = match orbits.fetch_orbit(&orbit_id).await {
    Ok(orbit) => match orbit {
      Some(orbit) => orbit,
      None => return build_api_not_found(orbit_id.to_string()),
    },
    Err(err) => return build_api_err(500, err.to_string(), Some(err.to_string())),
  };

  let avatar_content_type = match mime_guess::from_path(
    &form.images[0]
      .file_name
      .to_owned()
      .unwrap_or_else(|| ".§§§".to_string()),
  )
  .first()
  {
    Some(m) => m.to_string(),
    None => return build_api_err(500, "Unsupported file type".to_string(), None),
  };

  let banner_content_type = match mime_guess::from_path(
    &form.images[1]
      .file_name
      .to_owned()
      .unwrap_or_else(|| ".§§§".to_string()),
  )
  .first()
  {
    Some(m) => m.to_string(),
    None => return build_api_err(500, "Unsupported file type".to_string(), None),
  };

  let avatar_uri = match {
    let file_name = format!("media/{}/or/{}", orbit_id, Uuid::new_v4());

    let path = match cdn
      .upload_tmp_file(&form.images[0], &avatar_content_type, &file_name)
      .await
    {
      Ok(path) => path,
      Err(err) => return build_api_err(500, err.to_string(), Some(err.to_string())),
    };

    Some(format!("/{}", path))
  } {
    Some(uri) => uri,
    None => return build_api_err(500, "Image upload failed".to_string(), None),
  };

  let banner_uri = match {
    let file_name = format!("media/{}/or/{}", orbit_id, Uuid::new_v4());

    let path = match cdn
      .upload_tmp_file(&form.images[1], &banner_content_type, &file_name)
      .await
    {
      Ok(path) => path,
      Err(err) => return build_api_err(500, err.to_string(), Some(err.to_string())),
    };

    Some(format!("/{}", path))
  } {
    Some(uri) => uri,
    None => return build_api_err(500, "Image upload failed".to_string(), None),
  };

  match orbits
    .update_orbit(
      &orbit_id,
      &orbit.name,
      &orbit.description_md,
      &orbit.description_html,
      &Some(avatar_uri),
      &Some(banner_uri),
      false,
    )
    .await
  {
    Ok(_) => HttpResponse::Ok().finish(),
    Err(err) => build_api_err(500, err.to_string(), Some(err.to_string())),
  }
}

pub async fn api_delete_orbit(
  sessions: web::Data<SessionPool>,
  orbits: web::Data<OrbitPool>,
  tombstones: web::Data<TombstonePool>,
  orbit_moderators: web::Data<OrbitModeratorPool>,
  orbit_id: web::Path<Uuid>,
  jwt: web::ReqData<JwtContext>,
) -> impl Responder {
  let session = match require_auth(&jwt, &sessions).await {
    Ok(session) => session,
    Err(res) => return res,
  };

  match orbit_moderators.user_is_owner(&orbit_id, &session.uid).await {
    Ok(is_owner) => {
      if !is_owner {
        return build_api_not_found(session.uid.to_string());
      }
    }
    Err(err) => return build_api_err(500, err.to_string(), Some(err.to_string())),
  };

  let orbit = match orbits.fetch_orbit(&orbit_id).await {
    Ok(orbit) => match orbit {
      Some(orbit) => orbit,
      None => return HttpResponse::Ok().finish(),
    },
    Err(err) => return map_api_err(err),
  };

  match tombstones
    .create_tombstone(&orbit.uri, &ObjectType::Group.to_string())
    .await
  {
    Ok(_) => {}
    Err(err) => return map_api_err(err),
  };

  match orbits.delete_orbit(&orbit_id).await {
    Ok(_) => HttpResponse::Ok().finish(),
    Err(err) => build_api_err(500, err.to_string(), Some(err.to_string())),
  }
}

pub async fn api_join_orbit(
  sessions: web::Data<SessionPool>,
  user_orbits: web::Data<UserOrbitPool>,
  orbits: web::Data<OrbitPool>,
  jobs: web::Data<JobPool>,
  queue: web::Data<Queue>,
  orbit_id: web::Path<Uuid>,
  jwt: web::ReqData<JwtContext>,
) -> impl Responder {
  let session = match require_auth(&jwt, &sessions).await {
    Ok(session) => session,
    Err(res) => return res,
  };

  let orbit = match orbits.fetch_orbit(&orbit_id).await {
    Ok(orbit) => match orbit {
      Some(orbit) => orbit,
      None => return build_api_not_found(orbit_id.to_string()),
    },
    Err(err) => return build_api_err(500, err.to_string(), None),
  };

  if orbit.is_external {
    let job_id = match jobs
      .create(NewJob {
        created_by_id: Some(session.uid),
        status: JobStatus::NotStarted,
        record_id: Some(*orbit_id),
        associated_record_id: None,
      })
      .await
      .map_err(map_db_err)
    {
      Ok(id) => id,
      Err(err) => return build_api_err(500, err.to_string(), None),
    };

    let job = QueueJob::builder()
      .job_id(job_id)
      .job_type(QueueJobType::FederateActivityPubExt)
      .context(vec![session.uid.to_string()])
      .activitypub_federate_ext_action(FederateExtAction::FollowGroup(*orbit_id))
      .activitypub_federate_ext_dest_actor(FederateExtActorRef::None)
      .build();

    match queue.send_job(job).await {
      Ok(_) => {}
      Err(err) => return build_api_err(500, err.to_string(), None),
    }
  }

  match user_orbits.create_user_orbit(&orbit_id, &session.uid).await {
    Ok(_) => HttpResponse::Created().finish(),
    Err(err) => build_api_err(500, err.to_string(), None),
  }
}

pub async fn api_leave_orbit(
  sessions: web::Data<SessionPool>,
  user_orbits: web::Data<UserOrbitPool>,
  orbits: web::Data<OrbitPool>,
  jobs: web::Data<JobPool>,
  queue: web::Data<Queue>,
  orbit_id: web::Path<Uuid>,
  jwt: web::ReqData<JwtContext>,
) -> impl Responder {
  let session = match require_auth(&jwt, &sessions).await {
    Ok(session) => session,
    Err(res) => return res,
  };

  let orbit = match orbits.fetch_orbit(&orbit_id).await {
    Ok(orbit) => match orbit {
      Some(orbit) => orbit,
      None => return build_api_not_found(orbit_id.to_string()),
    },
    Err(err) => return build_api_err(500, err.to_string(), None),
  };

  if orbit.is_external {
    let job_id = match jobs
      .create(NewJob {
        created_by_id: Some(session.uid),
        status: JobStatus::NotStarted,
        record_id: Some(*orbit_id),
        associated_record_id: None,
      })
      .await
      .map_err(map_db_err)
    {
      Ok(id) => id,
      Err(err) => return build_api_err(500, err.to_string(), None),
    };

    let job = QueueJob::builder()
      .job_id(job_id)
      .job_type(QueueJobType::FederateActivityPubExt)
      .context(vec![session.uid.to_string()])
      .activitypub_federate_ext_action(FederateExtAction::UnfollowGroup(*orbit_id))
      .activitypub_federate_ext_dest_actor(FederateExtActorRef::None)
      .build();

    match queue.send_job(job).await {
      Ok(_) => {}
      Err(err) => return build_api_err(500, err.to_string(), None),
    }
  }

  match user_orbits.delete_user_orbit(&orbit_id, &session.uid).await {
    Ok(_) => HttpResponse::Created().finish(),
    Err(err) => build_api_err(500, err.to_string(), None),
  }
}

pub async fn api_get_orbit_moderators(
  orbit_moderators: web::Data<OrbitModeratorPool>,
  orbit_id: web::Path<Uuid>,
  query: web::Query<OrbitsQuery>,
) -> impl Responder {
  let page = query.page.unwrap_or(0);
  let page_size = query.page_size.unwrap_or(20);
  let posts_count = match orbit_moderators.count_users(&orbit_id).await {
    Ok(count) => count,
    Err(err) => return build_api_err(500, err.to_string(), Some(err.to_string())),
  };

  let orbits = match orbit_moderators
    .fetch_users(&orbit_id, page_size, page * page_size)
    .await
  {
    Ok(posts) => posts,
    Err(err) => return build_api_err(500, err.to_string(), Some(err.to_string())),
  };

  HttpResponse::Ok().json(ListResponse {
    data: orbits.into_iter().map(UserAccountPub::from).collect(),
    page,
    total_items: posts_count,
    total_pages: div_up(posts_count, page_size) + 1,
  })
}

pub async fn api_create_orbit_moderator(
  sessions: web::Data<SessionPool>,
  orbits: web::Data<OrbitPool>,
  orbit_moderators: web::Data<OrbitModeratorPool>,
  orbit_id: web::Path<Uuid>,
  req: web::Json<NewOrbitModeratorRequest>,
  jwt: web::ReqData<JwtContext>,
) -> impl Responder {
  let session = match require_auth(&jwt, &sessions).await {
    Ok(session) => session,
    Err(res) => return res,
  };

  match orbits.orbit_is_external(&orbit_id).await {
    Ok(is_external) => {
      if is_external {
        return build_api_not_found(session.uid.to_string());
      }
    }
    Err(err) => return build_api_err(500, err.to_string(), Some(err.to_string())),
  };

  match orbit_moderators.user_is_owner(&orbit_id, &session.uid).await {
    Ok(is_owner) => {
      if !is_owner {
        return build_api_not_found(session.uid.to_string());
      }
    }
    Err(err) => return build_api_err(500, err.to_string(), Some(err.to_string())),
  };

  match orbit_moderators
    .create_orbit_moderator(&orbit_id, &req.user_id, req.is_owner.unwrap_or(false))
    .await
  {
    Ok(_) => HttpResponse::Created().finish(),
    Err(err) => build_api_err(500, err.to_string(), Some(err.to_string())),
  }
}

pub async fn api_delete_orbit_moderator(
  sessions: web::Data<SessionPool>,
  orbit_moderators: web::Data<OrbitModeratorPool>,
  orbit_id: web::Path<Uuid>,
  req: web::Json<NewOrbitModeratorRequest>,
  jwt: web::ReqData<JwtContext>,
) -> impl Responder {
  let session = match require_auth(&jwt, &sessions).await {
    Ok(session) => session,
    Err(res) => return res,
  };

  match orbit_moderators.user_is_owner(&orbit_id, &session.uid).await {
    Ok(is_owner) => {
      if !is_owner {
        return build_api_not_found(session.uid.to_string());
      }
    }
    Err(err) => return build_api_err(500, err.to_string(), Some(err.to_string())),
  };

  match orbit_moderators.delete_orbit_moderator(&orbit_id, &req.user_id).await {
    Ok(_) => HttpResponse::Ok().finish(),
    Err(err) => build_api_err(500, err.to_string(), Some(err.to_string())),
  }
}

pub async fn api_update_orbit_moderator(
  sessions: web::Data<SessionPool>,
  orbit_moderators: web::Data<OrbitModeratorPool>,
  orbit_id: web::Path<Uuid>,
  req: web::Json<NewOrbitModeratorRequest>,
  jwt: web::ReqData<JwtContext>,
) -> impl Responder {
  let session = match require_auth(&jwt, &sessions).await {
    Ok(session) => session,
    Err(res) => return res,
  };

  match orbit_moderators.user_is_owner(&orbit_id, &session.uid).await {
    Ok(is_owner) => {
      if !is_owner {
        return build_api_not_found(session.uid.to_string());
      }
    }
    Err(err) => return build_api_err(500, err.to_string(), Some(err.to_string())),
  };

  match orbit_moderators
    .update_orbit_moderator(&orbit_id, &req.user_id, req.is_owner.unwrap_or(false))
    .await
  {
    Ok(_) => HttpResponse::Ok().finish(),
    Err(err) => build_api_err(500, err.to_string(), Some(err.to_string())),
  }
}
