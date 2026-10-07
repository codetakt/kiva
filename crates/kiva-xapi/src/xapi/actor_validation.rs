use serde_json::{Map, Value};
use thiserror::Error;
use url::Url;

const IFI_PROPERTIES: [&str; 4] = ["mbox", "mbox_sha1sum", "openid", "account"];
const ACTOR_PROPERTIES: [&str; 7] = [
    "objectType",
    "name",
    "mbox",
    "mbox_sha1sum",
    "openid",
    "account",
    "member",
];

/// A precise Actor data-model violation found during Statement admission.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ActorValidationError {
    /// An Actor-bearing property was not a JSON object.
    #[error("{path} must be an Actor object")]
    ExpectedObject {
        /// Logical path of the invalid Actor position.
        path: String,
    },
    /// An Actor property is not defined by the Agent or Group tables.
    #[error("{path} contains an unsupported Actor property")]
    UnsupportedProperty {
        /// Logical path of the invalid Actor position.
        path: String,
    },
    /// `objectType` was not a string.
    #[error("{path} objectType must be a string")]
    ObjectTypeNotString {
        /// Logical path of the invalid Actor position.
        path: String,
    },
    /// `objectType` did not match the role occupied by the Actor.
    #[error("{path} objectType is not permitted in this Actor position")]
    ObjectTypeNotPermitted {
        /// Logical path of the invalid Actor position.
        path: String,
    },
    /// `name` was present but was not a string.
    #[error("{path} name must be a string")]
    NameNotString {
        /// Logical path of the invalid Actor position.
        path: String,
    },
    /// An Agent did not contain exactly one IFI.
    #[error("{path} Agent must contain exactly one inverse functional identifier")]
    AgentIdentifierCount {
        /// Logical path of the invalid Agent.
        path: String,
    },
    /// A Group contained more than one IFI.
    #[error("{path} Group must not contain multiple inverse functional identifiers")]
    GroupIdentifierCount {
        /// Logical path of the invalid Group.
        path: String,
    },
    /// An anonymous Group omitted its required member array.
    #[error("{path} anonymous Group must contain a member array")]
    AnonymousGroupMemberRequired {
        /// Logical path of the invalid anonymous Group.
        path: String,
    },
    /// A Group member property was not an array.
    #[error("{path} Group member must be an array")]
    GroupMemberNotArray {
        /// Logical path of the invalid Group.
        path: String,
    },
    /// A Group member was not an Agent.
    #[error("{path} Group member must be an Agent")]
    GroupMemberNotAgent {
        /// Logical path of the invalid Group member.
        path: String,
    },
    /// `mbox` was not a string.
    #[error("{path} mbox must be a string")]
    MboxNotString {
        /// Logical path of the invalid Actor.
        path: String,
    },
    /// `mbox` was not a mailto IRI containing an email address.
    #[error("{path} mbox must be a mailto IRI containing an email address")]
    InvalidMbox {
        /// Logical path of the invalid Actor.
        path: String,
    },
    /// `mbox_sha1sum` was not a string.
    #[error("{path} mbox_sha1sum must be a string")]
    MboxSha1sumNotString {
        /// Logical path of the invalid Actor.
        path: String,
    },
    /// `mbox_sha1sum` was not a 40-character hexadecimal SHA-1 value.
    #[error("{path} mbox_sha1sum must be a 40-character hexadecimal value")]
    InvalidMboxSha1sum {
        /// Logical path of the invalid Actor.
        path: String,
    },
    /// `openid` was not a string.
    #[error("{path} openid must be a string")]
    OpenidNotString {
        /// Logical path of the invalid Actor.
        path: String,
    },
    /// `openid` was not an absolute URI.
    #[error("{path} openid must be an absolute URI")]
    InvalidOpenid {
        /// Logical path of the invalid Actor.
        path: String,
    },
    /// `account` was not an object.
    #[error("{path} account must be an object")]
    AccountNotObject {
        /// Logical path of the Actor containing the invalid Account.
        path: String,
    },
    /// An Account property was absent or was not a string.
    #[error("{path} account must contain string homePage and name properties")]
    InvalidAccountShape {
        /// Logical path of the Actor containing the invalid Account.
        path: String,
    },
    /// An Account contained a property outside the Account table.
    #[error("{path} account contains an unsupported property")]
    UnsupportedAccountProperty {
        /// Logical path of the Actor containing the invalid Account.
        path: String,
    },
    /// Account `homePage` was not an absolute IRL/IRI.
    #[error("{path} account homePage must be an absolute IRL")]
    InvalidAccountHomePage {
        /// Logical path of the Actor containing the invalid Account.
        path: String,
    },
}

#[derive(Clone, Copy)]
pub(crate) enum ActorRole {
    AgentOrGroup,
    Agent,
    Group,
}

/// Validates every Agent and Group reachable through a Statement's Actor positions.
pub fn validate_statement_actors(statement: &Value) -> Result<(), ActorValidationError> {
    let Some(statement) = statement.as_object() else {
        return Ok(());
    };
    validate_statement_actor_fields(statement, "Statement")
}

/// Validates an Agent supplied outside a Statement, such as an Agents Resource query.
pub fn validate_agent(value: &Value) -> Result<(), ActorValidationError> {
    validate_actor_value(value, "Agent", ActorRole::Agent)
}

fn validate_statement_actor_fields(
    statement: &Map<String, Value>,
    path: &str,
) -> Result<(), ActorValidationError> {
    validate_optional_actor(
        statement.get("actor"),
        &format!("{path} actor"),
        ActorRole::AgentOrGroup,
    )?;
    validate_optional_actor(
        statement.get("authority"),
        &format!("{path} authority"),
        ActorRole::AgentOrGroup,
    )?;
    validate_statement_object_actor(statement.get("object"), &format!("{path} object"))?;
    validate_context_actors(statement.get("context"), &format!("{path} context"))
}

fn validate_statement_object_actor(
    value: Option<&Value>,
    path: &str,
) -> Result<(), ActorValidationError> {
    let Some(object) = value.and_then(Value::as_object) else {
        return Ok(());
    };
    match object.get("objectType").and_then(Value::as_str) {
        Some("Agent") => validate_actor_object(object, path, ActorRole::Agent),
        Some("Group") => validate_actor_object(object, path, ActorRole::Group),
        Some("SubStatement") => validate_statement_actor_fields(object, path),
        _ => Ok(()),
    }
}

fn validate_context_actors(value: Option<&Value>, path: &str) -> Result<(), ActorValidationError> {
    let Some(context) = value.and_then(Value::as_object) else {
        return Ok(());
    };
    validate_optional_actor(
        context.get("instructor"),
        &format!("{path} instructor"),
        ActorRole::AgentOrGroup,
    )?;
    validate_optional_actor(
        context.get("team"),
        &format!("{path} team"),
        ActorRole::Group,
    )?;
    validate_context_actor_relations(
        context.get("contextAgents"),
        path,
        "contextAgents",
        "agent",
        ActorRole::Agent,
    )?;
    validate_context_actor_relations(
        context.get("contextGroups"),
        path,
        "contextGroups",
        "group",
        ActorRole::Group,
    )
}

fn validate_context_actor_relations(
    value: Option<&Value>,
    path: &str,
    collection_name: &str,
    actor_name: &str,
    role: ActorRole,
) -> Result<(), ActorValidationError> {
    let Some(relations) = value.and_then(Value::as_array) else {
        return Ok(());
    };
    for (index, relation) in relations.iter().enumerate() {
        let Some(actor) = relation.as_object().and_then(|value| value.get(actor_name)) else {
            continue;
        };
        validate_optional_actor(
            Some(actor),
            &format!("{path} {collection_name}[{index}] {actor_name}"),
            role,
        )?;
    }
    Ok(())
}

fn validate_optional_actor(
    value: Option<&Value>,
    path: &str,
    role: ActorRole,
) -> Result<(), ActorValidationError> {
    let Some(value) = value else {
        return Ok(());
    };
    let actor = value
        .as_object()
        .ok_or_else(|| ActorValidationError::ExpectedObject {
            path: path.to_owned(),
        })?;
    validate_actor_object(actor, path, role)
}

pub(crate) fn validate_actor_value(
    value: &Value,
    path: &str,
    role: ActorRole,
) -> Result<(), ActorValidationError> {
    let actor = value
        .as_object()
        .ok_or_else(|| ActorValidationError::ExpectedObject {
            path: path.to_owned(),
        })?;
    validate_actor_object(actor, path, role)
}

fn validate_actor_object(
    actor: &Map<String, Value>,
    path: &str,
    role: ActorRole,
) -> Result<(), ActorValidationError> {
    if actor
        .keys()
        .any(|name| !ACTOR_PROPERTIES.contains(&name.as_str()))
    {
        return Err(ActorValidationError::UnsupportedProperty {
            path: path.to_owned(),
        });
    }
    if actor.get("name").is_some_and(|name| !name.is_string()) {
        return Err(ActorValidationError::NameNotString {
            path: path.to_owned(),
        });
    }
    let object_type = match actor.get("objectType") {
        Some(value) => value
            .as_str()
            .ok_or_else(|| ActorValidationError::ObjectTypeNotString {
                path: path.to_owned(),
            })?,
        None => "Agent",
    };
    let is_agent = match (role, object_type) {
        (ActorRole::AgentOrGroup | ActorRole::Agent, "Agent") => true,
        (ActorRole::AgentOrGroup | ActorRole::Group, "Group") => false,
        _ => {
            return Err(ActorValidationError::ObjectTypeNotPermitted {
                path: path.to_owned(),
            });
        }
    };
    validate_ifi_values(actor, path)?;
    let ifi_count = IFI_PROPERTIES
        .iter()
        .filter(|name| actor.contains_key(**name))
        .count();
    if is_agent {
        if ifi_count != 1 {
            return Err(ActorValidationError::AgentIdentifierCount {
                path: path.to_owned(),
            });
        }
        if actor.contains_key("member") {
            return Err(ActorValidationError::UnsupportedProperty {
                path: path.to_owned(),
            });
        }
        return Ok(());
    }
    validate_group(actor, path, ifi_count)
}

fn validate_group(
    group: &Map<String, Value>,
    path: &str,
    ifi_count: usize,
) -> Result<(), ActorValidationError> {
    if ifi_count > 1 {
        return Err(ActorValidationError::GroupIdentifierCount {
            path: path.to_owned(),
        });
    }
    let members = match group.get("member") {
        Some(value) => {
            Some(
                value
                    .as_array()
                    .ok_or_else(|| ActorValidationError::GroupMemberNotArray {
                        path: path.to_owned(),
                    })?,
            )
        }
        None if ifi_count == 0 => {
            return Err(ActorValidationError::AnonymousGroupMemberRequired {
                path: path.to_owned(),
            });
        }
        None => None,
    };
    if let Some(members) = members {
        for (index, member) in members.iter().enumerate() {
            let member_path = format!("{path} member[{index}]");
            let member =
                member
                    .as_object()
                    .ok_or_else(|| ActorValidationError::GroupMemberNotAgent {
                        path: member_path.clone(),
                    })?;
            if member.get("objectType").and_then(Value::as_str) == Some("Group") {
                return Err(ActorValidationError::GroupMemberNotAgent { path: member_path });
            }
            validate_actor_object(member, &member_path, ActorRole::Agent)?;
        }
    }
    Ok(())
}

fn validate_ifi_values(actor: &Map<String, Value>, path: &str) -> Result<(), ActorValidationError> {
    if let Some(value) = actor.get("mbox") {
        let value = value
            .as_str()
            .ok_or_else(|| ActorValidationError::MboxNotString {
                path: path.to_owned(),
            })?;
        if !is_mailto_email(value) {
            return Err(ActorValidationError::InvalidMbox {
                path: path.to_owned(),
            });
        }
    }
    if let Some(value) = actor.get("mbox_sha1sum") {
        let value = value
            .as_str()
            .ok_or_else(|| ActorValidationError::MboxSha1sumNotString {
                path: path.to_owned(),
            })?;
        if value.len() != 40 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(ActorValidationError::InvalidMboxSha1sum {
                path: path.to_owned(),
            });
        }
    }
    if let Some(value) = actor.get("openid") {
        let value = value
            .as_str()
            .ok_or_else(|| ActorValidationError::OpenidNotString {
                path: path.to_owned(),
            })?;
        if Url::parse(value).is_err() {
            return Err(ActorValidationError::InvalidOpenid {
                path: path.to_owned(),
            });
        }
    }
    if let Some(value) = actor.get("account") {
        validate_account(value, path)?;
    }
    Ok(())
}

fn validate_account(value: &Value, path: &str) -> Result<(), ActorValidationError> {
    let account = value
        .as_object()
        .ok_or_else(|| ActorValidationError::AccountNotObject {
            path: path.to_owned(),
        })?;
    if account
        .keys()
        .any(|name| !matches!(name.as_str(), "homePage" | "name"))
    {
        return Err(ActorValidationError::UnsupportedAccountProperty {
            path: path.to_owned(),
        });
    }
    let home_page = account
        .get("homePage")
        .and_then(Value::as_str)
        .ok_or_else(|| ActorValidationError::InvalidAccountShape {
            path: path.to_owned(),
        })?;
    let name = account.get("name").and_then(Value::as_str).ok_or_else(|| {
        ActorValidationError::InvalidAccountShape {
            path: path.to_owned(),
        }
    })?;
    if name.is_empty() {
        return Err(ActorValidationError::InvalidAccountShape {
            path: path.to_owned(),
        });
    }
    if Url::parse(home_page).is_err() {
        return Err(ActorValidationError::InvalidAccountHomePage {
            path: path.to_owned(),
        });
    }
    Ok(())
}

fn is_mailto_email(value: &str) -> bool {
    let Some(address) = value.strip_prefix("mailto:") else {
        return false;
    };
    if address.is_empty()
        || address.chars().any(char::is_whitespace)
        || address.contains(['?', '#'])
    {
        return false;
    }
    let mut parts = address.split('@');
    matches!(
        (parts.next(), parts.next(), parts.next()),
        (Some(local), Some(domain), None) if !local.is_empty() && !domain.is_empty()
    )
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{ActorValidationError, validate_statement_actors};

    fn statement(actor: &Value) -> Value {
        json!({
            "actor": actor,
            "verb": {"id": "https://example.org/verbs/experienced"},
            "object": {"objectType": "Activity", "id": "https://example.org/activity"}
        })
    }

    use serde_json::Value;

    #[test]
    fn accepts_agent_and_identified_or_anonymous_groups() {
        let values = [
            json!({"objectType": "Agent", "mbox": "mailto:learner@example.org"}),
            json!({"objectType": "Group", "openid": "https://example.org/groups/1"}),
            json!({
                "objectType": "Group",
                "member": [{"account": {"homePage": "https://example.org", "name": "learner"}}]
            }),
        ];
        for value in values {
            assert_eq!(validate_statement_actors(&statement(&value)), Ok(()));
        }
    }

    #[test]
    fn rejects_missing_and_multiple_agent_identifiers() {
        assert!(matches!(
            validate_statement_actors(&statement(&json!({"objectType": "Agent"}))),
            Err(ActorValidationError::AgentIdentifierCount { .. })
        ));
        assert!(matches!(
            validate_statement_actors(&statement(&json!({
                "mbox": "mailto:learner@example.org",
                "openid": "https://example.org/learners/1"
            }))),
            Err(ActorValidationError::AgentIdentifierCount { .. })
        ));
    }

    #[test]
    fn rejects_actor_object_type_and_name_shape() {
        assert!(matches!(
            validate_statement_actors(&statement(&json!({
                "objectType": 7,
                "mbox": "mailto:learner@example.org"
            }))),
            Err(ActorValidationError::ObjectTypeNotString { .. })
        ));
        assert!(matches!(
            validate_statement_actors(&statement(&json!({
                "objectType": "Activity",
                "mbox": "mailto:learner@example.org"
            }))),
            Err(ActorValidationError::ObjectTypeNotPermitted { .. })
        ));
        assert!(matches!(
            validate_statement_actors(&statement(&json!({
                "name": {"en-US": "Learner"},
                "mbox": "mailto:learner@example.org"
            }))),
            Err(ActorValidationError::NameNotString { .. })
        ));
    }

    #[test]
    fn rejects_invalid_ifi_values_and_accounts() {
        let invalid = [
            json!({"mbox": "learner.example.org"}),
            json!({"mbox_sha1sum": "not-a-sha1"}),
            json!({"openid": "relative/openid"}),
            json!({"account": {"homePage": "relative", "name": "learner"}}),
            json!({"account": {"homePage": "https://example.org"}}),
        ];
        for actor in invalid {
            assert!(validate_statement_actors(&statement(&actor)).is_err());
        }
    }

    #[test]
    fn rejects_invalid_account_shapes() {
        let invalid = [
            json!({"account": "not-an-object"}),
            json!({"account": {"homePage": "https://example.org", "name": ""}}),
            json!({"account": {"homePage": "https://example.org", "name": "learner", "extra": true}}),
        ];
        for actor in invalid {
            assert!(validate_statement_actors(&statement(&actor)).is_err());
        }
    }

    #[test]
    fn rejects_invalid_group_members_and_anonymous_group_shape() {
        let invalid = [
            json!({"objectType": "Group"}),
            json!({"objectType": "Group", "member": {}}),
            json!({
                "objectType": "Group",
                "mbox": "mailto:group@example.org",
                "openid": "https://example.org/groups/1"
            }),
            json!({"objectType": "Group", "member": [{}]}),
            json!({
                "objectType": "Group",
                "member": [{"objectType": "Group", "member": []}]
            }),
        ];
        for actor in invalid {
            assert!(validate_statement_actors(&statement(&actor)).is_err());
        }
    }

    #[test]
    fn validates_nested_and_context_actor_positions() {
        let invalid = json!({
            "actor": {"mbox": "mailto:learner@example.org"},
            "verb": {"id": "https://example.org/verbs/experienced"},
            "object": {
                "objectType": "SubStatement",
                "actor": {"mbox": "mailto:nested@example.org"},
                "verb": {"id": "https://example.org/verbs/experienced"},
                "object": {"objectType": "Activity", "id": "https://example.org/activity"},
                "context": {
                    "team": {"objectType": "Agent", "mbox": "mailto:not-a-team@example.org"}
                }
            }
        });
        assert!(matches!(
            validate_statement_actors(&invalid),
            Err(ActorValidationError::ObjectTypeNotPermitted { .. })
        ));
    }

    #[test]
    fn validates_statement_object_and_authority_actor_positions() {
        let invalid_object = json!({
            "actor": {"mbox": "mailto:learner@example.org"},
            "verb": {"id": "https://example.org/verbs/experienced"},
            "object": {
                "objectType": "Agent",
                "mbox": "mailto:object@example.org",
                "openid": "https://example.org/object"
            }
        });
        assert!(matches!(
            validate_statement_actors(&invalid_object),
            Err(ActorValidationError::AgentIdentifierCount { .. })
        ));

        let invalid_authority = json!({
            "actor": {"mbox": "mailto:learner@example.org"},
            "verb": {"id": "https://example.org/verbs/experienced"},
            "object": {"objectType": "Activity", "id": "https://example.org/activity"},
            "authority": {"objectType": "Group", "member": [{"objectType": "Group"}]}
        });
        assert!(matches!(
            validate_statement_actors(&invalid_authority),
            Err(ActorValidationError::GroupMemberNotAgent { .. })
        ));
    }
}
