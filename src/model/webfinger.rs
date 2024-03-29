use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::settings::SETTINGS;

#[derive(Deserialize, Serialize, Clone)]
pub struct WebfingerRecordLink {
  pub rel: String,
  #[serde(skip_serializing_if = "Option::is_none", rename = "type")]
  pub link_type: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub href: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub template: Option<String>,
}

impl WebfingerRecordLink {
  pub fn build_user_self_uri(id: &Uuid) -> String {
    format!("{}/user/{}", SETTINGS.server.api_fqdn, id)
  }

  pub fn build_user_self_link(id: &Uuid) -> WebfingerRecordLink {
    WebfingerRecordLink {
      rel: "self".to_string(),
      link_type: Some("application/activity+json".to_string()),
      href: Some(format!("{}/user/{}", SETTINGS.server.api_fqdn, id)),
      template: None,
    }
  }

  pub fn build_user_feed_link(id: &Uuid) -> WebfingerRecordLink {
    WebfingerRecordLink {
      rel: "feed".to_string(),
      link_type: Some("application/activity+json".to_string()),
      href: Some(format!("{}/user/{}/feed", SETTINGS.server.api_fqdn, id)),
      template: None,
    }
  }

  pub fn build_profile_page_link(handle: &str) -> WebfingerRecordLink {
    WebfingerRecordLink {
      rel: "http://webfinger.net/rel/profile-page".to_string(),
      link_type: Some("text/html".to_string()),
      href: Some(format!("{}/users/{}", SETTINGS.server.fqdn, handle)),
      template: None,
    }
  }

  pub fn build_orbit_page_link(shortcode: &str) -> WebfingerRecordLink {
    WebfingerRecordLink {
      rel: "http://webfinger.net/rel/profile-page".to_string(),
      link_type: Some("text/html".to_string()),
      href: Some(format!("{}/orbits/{}", SETTINGS.server.fqdn, shortcode)),
      template: None,
    }
  }

  pub fn build_orbit_self_uri(id: &Uuid) -> String {
    format!("{}/orbit/{}", SETTINGS.server.api_fqdn, id)
  }

  pub fn build_orbit_self_link(id: &Uuid) -> WebfingerRecordLink {
    WebfingerRecordLink {
      rel: "self".to_string(),
      link_type: Some("application/activity+json".to_string()),
      href: Some(format!("{}/orbit/{}", SETTINGS.server.api_fqdn, id)),
      template: None,
    }
  }

  pub fn build_orbit_feed_link(id: &Uuid) -> WebfingerRecordLink {
    WebfingerRecordLink {
      rel: "feed".to_string(),
      link_type: Some("application/activity+json".to_string()),
      href: Some(format!("{}/orbit/{}/feed", SETTINGS.server.api_fqdn, id)),
      template: None,
    }
  }
}

#[derive(Deserialize, Serialize, Clone)]
pub struct WebfingerRecord {
  pub subject: String,
  pub links: Vec<WebfingerRecordLink>,
  pub aliases: Option<Vec<String>>,
}
