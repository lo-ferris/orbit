use std::{collections::HashMap, str::FromStr};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};
use typed_builder::TypedBuilder;

use super::{
  activity::ActivityProps,
  actor::ActorProps,
  collection::{CollectionPageProps, CollectionProps},
  key::KeyProps,
  link::LinkProps,
  orbit::OrbitProps,
  place::PlaceProps,
  profile::ProfileProps,
  question::QuestionProps,
  rdf_string::RdfString,
  reference::Reference,
  relationship::RelationshipProps,
  tombstone::TombstoneProps,
};

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Display, Debug, EnumString)]
#[serde(untagged)]
pub enum ObjectType {
  Article,
  Audio,
  Document,
  Event,
  Image,
  Note,
  Page,
  Place,
  Profile,
  Relationship,
  Tombstone,
  Video,
  Application,
  Group,
  Organization,
  Person,
  Service,
  Mention,
  OrderedCollection,
  OrderedCollectionPage,
}

impl ObjectType {
  pub fn from_str_opt(data: &Option<String>) -> Option<Self> {
    match &data {
      Some(val) => match ObjectType::from_str(val) {
        Ok(val) => Some(val),
        Err(_) => None,
      },
      None => None,
    }
  }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug, TypedBuilder)]
#[builder(field_defaults(default))]
pub struct ObjectSource {
  pub content: String,
  #[serde(rename = "mediaType")]
  pub media_type: String,
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug, TypedBuilder)]
#[builder(field_defaults(default))]
pub struct Object {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub id: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  #[serde(rename = "type")]
  pub kind: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub actor: Option<Reference<Object>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub attachment: Option<Reference<Object>>,
  #[serde(rename = "attributedTo", skip_serializing_if = "Option::is_none")]
  pub attributed_to: Option<Reference<Object>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub audience: Option<Reference<Object>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub bcc: Option<Reference<Object>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub bto: Option<Reference<Object>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub cc: Option<Reference<Object>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub context: Option<Reference<Object>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub generator: Option<Reference<Object>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub icon: Option<Reference<Object>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub image: Option<Reference<Object>>,
  #[serde(rename = "inReplyTo", skip_serializing_if = "Option::is_none")]
  pub in_reply_to: Option<Reference<Object>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub location: Option<Reference<Object>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub preview: Option<Reference<Object>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub replies: Option<Box<Object>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub tag: Option<Reference<Object>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub to: Option<Reference<Object>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub altitude: Option<f32>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub content: Option<RdfString>,
  #[serde(rename = "contentMap", skip_serializing_if = "Option::is_none")]
  pub content_map: Option<HashMap<String, RdfString>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub name: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub duration: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub width: Option<u32>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub height: Option<u32>,
  #[serde(rename = "mediaType", skip_serializing_if = "Option::is_none")]
  pub media_type: Option<String>,
  #[serde(rename = "startTime", skip_serializing_if = "Option::is_none")]
  pub start_time: Option<DateTime<Utc>>,
  #[serde(rename = "endTime", skip_serializing_if = "Option::is_none")]
  pub end_time: Option<DateTime<Utc>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub published: Option<DateTime<Utc>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub summary: Option<RdfString>,
  #[serde(skip_serializing_if = "Option::is_none")]
  #[serde(rename = "summaryMap")]
  pub summary_map: Option<HashMap<String, RdfString>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub updated: Option<DateTime<Utc>>,
  #[serde(
    rename(serialize = "sensitive", deserialize = "as:sensitive"),
    skip_serializing_if = "Option::is_none"
  )]
  pub sensitive: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub url: Option<Reference<Object>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub source: Option<ObjectSource>,
  #[serde(rename = "publicKey", skip_serializing_if = "Option::is_none")]
  pub key: Option<KeyProps>,

  #[serde(flatten, skip_serializing_if = "Option::is_none")]
  pub link: Option<LinkProps>,

  #[serde(flatten, skip_serializing_if = "Option::is_none")]
  pub collection: Option<CollectionProps>,

  #[serde(flatten, skip_serializing_if = "Option::is_none")]
  pub collection_page: Option<CollectionPageProps>,

  #[serde(flatten, skip_serializing_if = "Option::is_none")]
  pub activity: Option<ActivityProps>,

  #[serde(flatten, skip_serializing_if = "Option::is_none")]
  pub question: Option<QuestionProps>,

  #[serde(flatten, skip_serializing_if = "Option::is_none")]
  pub actors: Option<ActorProps>,

  #[serde(flatten, skip_serializing_if = "Option::is_none")]
  pub place: Option<PlaceProps>,

  #[serde(flatten, skip_serializing_if = "Option::is_none")]
  pub profile: Option<ProfileProps>,

  #[serde(flatten, skip_serializing_if = "Option::is_none")]
  pub relationship: Option<RelationshipProps>,

  #[serde(flatten, skip_serializing_if = "Option::is_none")]
  pub tombstone: Option<TombstoneProps>,

  #[serde(flatten, skip_serializing_if = "Option::is_none")]
  pub orbit: Option<OrbitProps>,

  #[serde(flatten, skip_serializing_if = "Option::is_none")]
  pub extra: Option<serde_json::Value>,
}

#[cfg(test)]
mod tests {
  use chrono::{DateTime, Utc};
  use serde_json::{json, Error};

  use crate::activitypub::{
    activity::ActivityProps,
    actor::ActorProps,
    collection::{CollectionPageProps, CollectionProps},
    document::{ActivityPubDocument, RawActivityPubDocument},
    link::LinkProps,
    object::Object,
    orbit::OrbitProps,
    place::PlaceProps,
    profile::ProfileProps,
    question::QuestionProps,
    reference::Reference,
    relationship::RelationshipProps,
    tombstone::TombstoneProps,
  };

  #[test]
  pub fn parses_mastodon_post() {
    let raw_json = r#"{
      "@context": [
          "https://www.w3.org/ns/activitystreams",
          {
              "atomUri": "ostatus:atomUri",
              "conversation": "ostatus:conversation",
              "inReplyToAtomUri": "ostatus:inReplyToAtomUri",
              "ostatus": "http://ostatus.org#",
              "sensitive": "as:sensitive",
              "toot": "http://joinmastodon.org/ns#",
              "votersCount": "toot:votersCount"
          }
      ],
      "atomUri": "https://fosstodon.org/users/lyptt/statuses/109438776152157552",
      "attachment": [],
      "attributedTo": "https://fosstodon.org/users/lyptt",
      "cc": [
          "https://fosstodon.org/users/lyptt/followers",
          "https://fosstodon.org/users/msprout"
      ],
      "content": "<p><span class=\"h-card\"><a href=\"https://fosstodon.org/@msprout\" class=\"u-url mention\">@<span>msprout</span></a></span> Happy birthday! 🎂</p>",
      "contentMap": {
          "en": "<p><span class=\"h-card\"><a href=\"https://fosstodon.org/@msprout\" class=\"u-url mention\">@<span>msprout</span></a></span> Happy birthday! 🎂</p>"
      },
      "conversation": "tag:fosstodon.org,2022-12-01:objectId=64973737:objectType=Conversation",
      "id": "https://fosstodon.org/users/lyptt/statuses/109438776152157552",
      "inReplyTo": "https://fosstodon.org/users/msprout/statuses/109438763691474657",
      "inReplyToAtomUri": "https://fosstodon.org/users/msprout/statuses/109438763691474657",
      "published": "2022-12-01T14:00:00Z",
      "replies": {
          "first": {
              "items": [],
              "next": "https://fosstodon.org/users/lyptt/statuses/109438776152157552/replies?only_other_accounts=true&page=true",
              "partOf": "https://fosstodon.org/users/lyptt/statuses/109438776152157552/replies",
              "type": "CollectionPage"
          },
          "id": "https://fosstodon.org/users/lyptt/statuses/109438776152157552/replies",
          "type": "Collection"
      },
      "sensitive": false,
      "summary": null,
      "tag": [
          {
              "href": "https://fosstodon.org/users/msprout",
              "name": "@msprout",
              "type": "Mention"
          }
      ],
      "to": [
          "https://www.w3.org/ns/activitystreams#Public"
      ],
      "type": "Note",
      "url": "https://fosstodon.org/@lyptt/109438776152157552"
  }"#;

    let raw_doc_result: Result<RawActivityPubDocument, Error> = serde_json::from_str(raw_json);
    assert!(raw_doc_result.is_ok());

    let doc_result = ActivityPubDocument::from(raw_doc_result.unwrap());
    assert!(doc_result.is_ok());

    let doc = doc_result.unwrap();

    assert!(doc.object.extra.is_some());

    let extras = doc.object.extra.unwrap();

    assert_eq!(
      extras["ostatus:atomUri"].as_str(),
      Some("https://fosstodon.org/users/lyptt/statuses/109438776152157552")
    );
    assert_eq!(doc.object.attachment, Some(Reference::Mixed(vec![])));
    assert_eq!(
      doc.object.attributed_to,
      Some(Reference::Remote("https://fosstodon.org/users/lyptt".to_string()))
    );
    assert_eq!(
      doc.object.cc,
      Some(Reference::Mixed(vec![
        Reference::Remote("https://fosstodon.org/users/lyptt/followers".to_string()),
        Reference::Remote("https://fosstodon.org/users/msprout".to_string()),
      ]))
    );
    assert!(doc.object.content.is_some());
    assert!(doc.object.content_map.is_some());
    assert!(doc.object.content_map.unwrap().contains_key("en"));
    assert_eq!(
      extras["ostatus:conversation"].as_str(),
      Some("tag:fosstodon.org,2022-12-01:objectId=64973737:objectType=Conversation")
    );
    assert_eq!(
      doc.object.id,
      Some("https://fosstodon.org/users/lyptt/statuses/109438776152157552".to_string())
    );
    assert_eq!(
      doc.object.in_reply_to,
      Some(Reference::Remote(
        "https://fosstodon.org/users/msprout/statuses/109438763691474657".to_string()
      ))
    );
    assert_eq!(
      extras["ostatus:inReplyToAtomUri"].as_str(),
      Some("https://fosstodon.org/users/msprout/statuses/109438763691474657")
    );
    assert_eq!(
      doc.object.published,
      Some(
        DateTime::parse_from_rfc3339("2022-12-01T14:00:00Z")
          .unwrap()
          .with_timezone(&Utc)
      )
    );
    assert!(doc.object.replies.is_some());
    // EDITOR'S NOTE: inb4 ActivityPub is fucking crazy 🤯
    assert_eq!(
      doc.object.replies,
      Some(Box::new(
        Object::builder()
          .id(Some(
            "https://fosstodon.org/users/lyptt/statuses/109438776152157552/replies".to_string()
          ))
          .kind(Some("Collection".to_string()))
          .collection(Some(
            CollectionProps::builder()
              .first(Some(Reference::Embedded(Box::new(
                Object::builder()
                  .kind(Some("CollectionPage".to_string()))
                  .collection(Some(
                    CollectionProps::builder().items(Some(Reference::Mixed(vec![]))).build()
                  ))
                  .collection_page(Some(CollectionPageProps::builder().next(Some(Reference::Remote("https://fosstodon.org/users/lyptt/statuses/109438776152157552/replies?only_other_accounts=true&page=true".to_string()))).part_of(Some(Reference::Remote("https://fosstodon.org/users/lyptt/statuses/109438776152157552/replies".to_string()))).build()))
                  .extra(Some(json!({})))
                  .link(Some(LinkProps::builder().build()))
                  .activity(Some(ActivityProps::builder().build()))
                  .question(Some(QuestionProps::builder().build()))
                  .actors(Some(ActorProps::builder().build()))
                  .place(Some(PlaceProps::builder().build()))
                  .profile(Some(ProfileProps::builder().build()))
                  .relationship(Some(RelationshipProps::builder().build()))
                  .tombstone(Some(TombstoneProps::builder().build()))
                  .orbit(Some(OrbitProps::builder().build()))
                  .build()
              ))))
              .build()
          ))
          .collection_page(Some(CollectionPageProps::builder().build()))
          .link(Some(LinkProps::builder().build()))
          .activity(Some(ActivityProps::builder().build()))
          .question(Some(QuestionProps::builder().build()))
          .actors(Some(ActorProps::builder().build()))
          .place(Some(PlaceProps::builder().build()))
          .profile(Some(ProfileProps::builder().build()))
          .relationship(Some(RelationshipProps::builder().build()))
          .tombstone(Some(TombstoneProps::builder().build()))
          .orbit(Some(OrbitProps::builder().build()))
          .extra(Some(json!({})))
          .build()
      ))
    );
    assert_eq!(doc.object.sensitive, Some(false));
    assert_eq!(doc.object.summary, None);
    assert_eq!(
      doc.object.tag,
      Some(Reference::Mixed(vec![Reference::Embedded(Box::new(
        Object::builder()
          .kind(Some("Mention".to_string()))
          .name(Some("@msprout".to_string()))
          .link(Some(
            LinkProps::builder()
              .href(Some(Reference::Remote(
                "https://fosstodon.org/users/msprout".to_string()
              )))
              .build()
          ))
          .collection(Some(CollectionProps::builder().build()))
          .collection_page(Some(CollectionPageProps::builder().build()))
          .activity(Some(ActivityProps::builder().build()))
          .question(Some(QuestionProps::builder().build()))
          .actors(Some(ActorProps::builder().build()))
          .place(Some(PlaceProps::builder().build()))
          .profile(Some(ProfileProps::builder().build()))
          .relationship(Some(RelationshipProps::builder().build()))
          .tombstone(Some(TombstoneProps::builder().build()))
          .orbit(Some(OrbitProps::builder().build()))
          .extra(Some(json!({})))
          .build()
      ))]))
    );
    assert_eq!(
      doc.object.to,
      Some(Reference::Mixed(vec![Reference::Remote(
        "https://www.w3.org/ns/activitystreams#Public".to_string()
      )]))
    );
    assert_eq!(doc.object.kind, Some("Note".to_string()));
    assert_eq!(
      doc.object.url,
      Some(Reference::Remote(
        "https://fosstodon.org/@lyptt/109438776152157552".to_string()
      ))
    );
  }
}
