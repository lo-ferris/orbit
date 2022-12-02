use actix_web::{web, HttpResponse, Responder};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};
use uuid::Uuid;

use crate::{
  db::{app_repository::AppPool, session_repository::SessionPool, user_repository::UserPool},
  helpers::{
    auth::require_auth,
    core::build_api_err,
    html::{handle_oauth_app_body, handle_oauth_app_err, oauth_app_unwrap_result},
  },
  logic::{user::authorize_user, LogicErr},
  net::{
    jwt::{JwtContext, JwtFactory},
    templates::HANDLEBARS,
  },
};

#[derive(Debug, EnumString, Display, Serialize, Deserialize)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum OAuthAuthorizeResponseType {
  Code,
}

#[derive(Debug, EnumString, Display, Serialize, Deserialize)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum OAuthGrantType {
  AuthorizationCode,
  ClientCredentials,
  RefreshToken,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OAuthAuthorizeQuery {
  pub response_type: OAuthAuthorizeResponseType,
  pub client_id: String,
  pub redirect_uri: String,
  // TODO: Support scopes when we have permission controls
  #[serde(skip_serializing_if = "Option::is_none")]
  pub scope: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OAuthAuthorizeRequest {
  pub username: String,
  pub password: String,
}

#[derive(Debug, Serialize)]
struct OAuthAuthorizeData<'a> {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub username: Option<&'a str>,
  pub blessed: bool,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub app_name: Option<&'a str>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OAuthTokenRequest {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub code: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub refresh_token: Option<String>,
  pub grant_type: OAuthGrantType,
  pub client_id: String,
  pub client_secret: String,
  pub redirect_uri: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub scope: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OAuthTokenResponse {
  pub access_token: String,
  pub refresh_token: String,
  pub token_type: &'static str,
  pub scope: String,
  pub created_at: i64,
  pub expires_at: i64,
  pub refresh_expires_at: i64,
}

pub async fn api_oauth_authorize(apps: web::Data<AppPool>, query: web::Query<OAuthAuthorizeQuery>) -> impl Responder {
  match query.response_type {
    OAuthAuthorizeResponseType::Code => {
      let app = match oauth_app_unwrap_result(
        apps.fetch_by_client_id(&query.client_id).await,
        "This application is not configured correctly to authenticate with Chameleon",
      ) {
        Ok(app) => app,
        Err(res) => return res,
      };

      if app.client_id.to_string() != query.client_id {
        return handle_oauth_app_err(
          "The provided parameters do not match the parameters set for the registered appliction",
        );
      }

      if app.redirect_uri != query.redirect_uri {
        return handle_oauth_app_err(
          "The provided parameters do not match the parameters set for the registered appliction",
        );
      }

      let body = match HANDLEBARS.render(
        "oauth_authorize",
        &OAuthAuthorizeData {
          username: None,
          blessed: app.blessed,
          app_name: Some(&app.name),
        },
      ) {
        Ok(body) => body,
        Err(_) => return HttpResponse::InternalServerError().finish(),
      };

      HttpResponse::Ok().content_type("text/html; charset=utf-8").body(body)
    }
  }
}

pub async fn api_oauth_authorize_post(
  apps: web::Data<AppPool>,
  users: web::Data<UserPool>,
  query: web::Query<OAuthAuthorizeQuery>,
  req: web::Form<OAuthAuthorizeRequest>,
) -> impl Responder {
  let app = match oauth_app_unwrap_result(
    apps.fetch_by_client_id(&query.client_id).await,
    "This application is not configured correctly to authenticate with Chameleon",
  ) {
    Ok(app) => app,
    Err(res) => return res,
  };

  if app.client_id.to_string() != query.client_id {
    return handle_oauth_app_err(
      "The provided parameters do not match the parameters set for the registered appliction",
    );
  }

  if app.redirect_uri != query.redirect_uri {
    return handle_oauth_app_err(
      "The provided parameters do not match the parameters set for the registered appliction",
    );
  }

  let authorization_code = match authorize_user(&req.username, &req.password, &users).await {
    Ok(code) => code,
    Err(err) => match err {
      LogicErr::UnauthorizedError => {
        return handle_oauth_app_body(
          &app,
          "The credentials you provided did not match our records, please check you've entered your username and password correctly.",
        )
      }
      _ => {
        return handle_oauth_app_body(
          &app,
          "Something went wrong, please try again later",
        )
      }
    },
  };

  HttpResponse::Found()
    .insert_header((
      "location",
      format!("{}?code={}", query.redirect_uri, authorization_code),
    ))
    .finish()
}

pub async fn api_oauth_token(
  apps: web::Data<AppPool>,
  users: web::Data<UserPool>,
  sessions: web::Data<SessionPool>,
  session: web::ReqData<JwtContext>,
  req: web::Form<OAuthTokenRequest>,
) -> impl Responder {
  let app = match oauth_app_unwrap_result(
    apps.fetch_by_client_id(&req.client_id).await,
    "This application is not configured correctly to authenticate with Chameleon",
  ) {
    Ok(app) => app,
    Err(res) => return res,
  };

  if app.client_id.to_string() != req.client_id {
    return build_api_err(401, "Invalid client configuration".to_string(), None);
  }

  if app.client_secret.to_string() != req.client_secret {
    return build_api_err(401, "Invalid client configuration".to_string(), None);
  }

  if app.redirect_uri != req.redirect_uri {
    return build_api_err(401, "Invalid client configuration".to_string(), None);
  }

  match req.grant_type {
    OAuthGrantType::AuthorizationCode => {
      let code = req.code.clone().unwrap_or_default();
      let claims = match JwtFactory::parse_jwt_props(&code) {
        Some(claims) => claims,
        None => return build_api_err(401, "Invalid authorization token".to_string(), None),
      };

      let user = match users.fetch_by_handle(&claims.sub).await {
        Ok(user) => match user {
          Some(user) => user,
          None => return build_api_err(401, "Invalid authorization token".to_string(), None),
        },
        Err(_) => return build_api_err(401, "Invalid authorization token".to_string(), None),
      };

      let session_id = Uuid::new_v4();

      let session = match JwtFactory::generate_jwt_long_lived(&user, &session_id) {
        Ok(session) => session,
        Err(_) => return build_api_err(401, "Invalid authorization token".to_string(), None),
      };

      match sessions
        .insert_session(
          &session_id,
          &user.user_id,
          &app.app_id,
          &session.refresh_token,
          &session.access_expiry,
          &session.refresh_expiry,
        )
        .await
      {
        Ok(_) => {}
        Err(err) => return build_api_err(500, "Internal server error".to_string(), Some(err.to_string())),
      };

      HttpResponse::Ok().json(OAuthTokenResponse {
        access_token: session.access_token,
        refresh_token: session.refresh_token,
        token_type: "Bearer",
        scope: "".to_string(),
        created_at: Utc::now().timestamp(),
        expires_at: session.access_expiry.timestamp(),
        refresh_expires_at: session.refresh_expiry.timestamp(),
      })
    }
    OAuthGrantType::ClientCredentials => build_api_err(400, "Not implemented".to_string(), None),
    OAuthGrantType::RefreshToken => match require_auth(&session, &sessions).await {
      Ok(session) => {
        let refresh_token = req.refresh_token.clone().unwrap_or_default();

        match sessions.query_session_exists_for_refresh_token(&refresh_token).await {
          true => {}
          false => return build_api_err(401, "Invalid refresh token".to_string(), None),
        };

        let user = match users.fetch_by_fediverse_id(&session.sub).await {
          Ok(user) => match user {
            Some(user) => user,
            None => return build_api_err(401, "Invalid authorization token".to_string(), None),
          },
          Err(_) => return build_api_err(401, "Invalid authorization token".to_string(), None),
        };

        match sessions
          .delete_session(&user.user_id, &app.app_id, &refresh_token)
          .await
        {
          Ok(_) => {}
          Err(_) => return build_api_err(500, "Internal server error".to_string(), None),
        }

        let session_id = Uuid::new_v4();

        let session = match JwtFactory::generate_jwt_long_lived(&user, &session_id) {
          Ok(session) => session,
          Err(_) => return build_api_err(401, "Invalid authorization token".to_string(), None),
        };

        match sessions
          .insert_session(
            &session_id,
            &user.user_id,
            &app.app_id,
            &session.refresh_token,
            &session.access_expiry,
            &session.refresh_expiry,
          )
          .await
        {
          Ok(_) => {}
          Err(err) => return build_api_err(500, "Internal server error".to_string(), Some(err.to_string())),
        };

        HttpResponse::Ok().json(OAuthTokenResponse {
          access_token: session.access_token,
          refresh_token: session.refresh_token,
          token_type: "Bearer",
          scope: "".to_string(),
          created_at: Utc::now().timestamp(),
          expires_at: session.access_expiry.timestamp(),
          refresh_expires_at: session.refresh_expiry.timestamp(),
        })
      }
      Err(err) => err,
    },
  }
}
