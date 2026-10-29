#![allow(clippy::wildcard_imports)]

use super::*;

/// Applies xAPI Statement collection filtering, voided target exclusion, stored
/// time ordering, and limit truncation.
pub fn filter_statements(
    statements: Vec<Value>,
    query: &BTreeMap<String, String>,
    limit: Option<usize>,
) -> Result<Vec<Value>, StatementFilterError> {
    let filters = StatementFilters::from_query(query)?;
    let statement_index = statement_index(&statements);
    let voided_statement_ids = voided_statement_ids(&statements);
    let mut statements = statements
        .into_iter()
        .filter(|statement| !is_voided_target(statement, &voided_statement_ids))
        .filter(|statement| filters.matches(statement, &statement_index))
        .collect::<Vec<_>>();
    statements.sort_by(compare_statement_stored_time);
    if !filters.ascending {
        statements.reverse();
    }
    Ok(statements
        .into_iter()
        .take(limit.unwrap_or(usize::MAX))
        .collect())
}

fn compare_statement_stored_time(left: &Value, right: &Value) -> std::cmp::Ordering {
    let left_stored = statement_stored_at(left);
    let right_stored = statement_stored_at(right);
    match (left_stored, right_stored) {
        (Some(left_time), Some(right_time)) => left_time
            .cmp(&right_time)
            .then_with(|| statement_id_text(left).cmp(&statement_id_text(right))),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => statement_id_text(left).cmp(&statement_id_text(right)),
    }
}

/// Extracts the LRS-managed `stored` timestamp from a Statement JSON value.
#[must_use]
pub fn statement_stored_at(statement: &Value) -> Option<OffsetDateTime> {
    statement
        .get("stored")
        .and_then(Value::as_str)
        .and_then(|stored| OffsetDateTime::parse(stored, &Rfc3339).ok())
}

fn statement_id_text(statement: &Value) -> Option<&str> {
    statement.get("id").and_then(Value::as_str)
}

fn statement_index(statements: &[Value]) -> BTreeMap<Uuid, Value> {
    statements
        .iter()
        .filter_map(|statement| {
            statement
                .get("id")
                .and_then(Value::as_str)
                .and_then(|id| Uuid::parse_str(id).ok())
                .map(|id| (id, statement.clone()))
        })
        .collect()
}

fn voided_statement_ids(statements: &[Value]) -> BTreeSet<Uuid> {
    let index = statement_index(statements);
    statements
        .iter()
        .filter_map(voided_statement_id)
        .filter(|statement_id| {
            index.get(statement_id).is_none_or(|target| {
                target.pointer("/verb/id").and_then(Value::as_str)
                    != Some("http://adlnet.gov/expapi/verbs/voided")
            })
        })
        .collect::<BTreeSet<_>>()
}

/// Reports whether a stored Statement id is the target of a valid voiding Statement.
#[must_use]
pub fn statement_is_voided(statements: &[Value], statement_id: Uuid) -> bool {
    voided_statement_ids(statements).contains(&statement_id)
}

fn voided_statement_id(statement: &Value) -> Option<Uuid> {
    if statement.pointer("/verb/id").and_then(Value::as_str)
        != Some("http://adlnet.gov/expapi/verbs/voided")
    {
        return None;
    }
    if statement
        .pointer("/object/objectType")
        .and_then(Value::as_str)
        != Some("StatementRef")
    {
        return None;
    }
    statement
        .pointer("/object/id")
        .and_then(Value::as_str)
        .and_then(|id| Uuid::parse_str(id).ok())
}

fn is_voided_target(statement: &Value, voided_statement_ids: &BTreeSet<Uuid>) -> bool {
    statement
        .get("id")
        .and_then(Value::as_str)
        .and_then(|id| Uuid::parse_str(id).ok())
        .is_some_and(|statement_id| voided_statement_ids.contains(&statement_id))
}

#[derive(Debug, Default)]
struct StatementFilters {
    actor_ifi: Option<String>,
    verb_id: Option<String>,
    activity_id: Option<String>,
    registration: Option<Uuid>,
    related_activities: bool,
    related_agents: bool,
    since: Option<OffsetDateTime>,
    until: Option<OffsetDateTime>,
    ascending: bool,
}

impl StatementFilters {
    fn from_query(query: &BTreeMap<String, String>) -> Result<Self, StatementFilterError> {
        Ok(Self {
            actor_ifi: query
                .get("agent")
                .map(|agent| {
                    validate_unique_json_properties(agent.as_bytes())
                        .and_then(|()| serde_json::from_str::<Value>(agent))
                        .map_err(|_error| StatementFilterError::InvalidAgentJson)
                        .and_then(|agent| {
                            actor_validation::validate_filter_agent(&agent)
                                .map_err(|_error| StatementFilterError::InvalidAgent)?;
                            canonical_actor_ifi(&agent).ok_or(
                                StatementFilterError::AgentMissingInverseFunctionalIdentifier,
                            )
                        })
                })
                .transpose()?,
            verb_id: query.get("verb").cloned(),
            activity_id: query.get("activity").cloned(),
            registration: query
                .get("registration")
                .map(|value| {
                    Uuid::parse_str(value)
                        .map_err(|_error| StatementFilterError::InvalidRegistrationUuid)
                })
                .transpose()?,
            related_activities: query
                .get("related_activities")
                .is_some_and(|value| value == "true"),
            related_agents: query
                .get("related_agents")
                .is_some_and(|value| value == "true"),
            since: query
                .get("since")
                .map(|value| {
                    parse_query_timestamp(value, StatementFilterError::InvalidSinceTimestamp)
                })
                .transpose()?,
            until: query
                .get("until")
                .map(|value| {
                    parse_query_timestamp(value, StatementFilterError::InvalidUntilTimestamp)
                })
                .transpose()?,
            ascending: query.get("ascending").is_some_and(|value| value == "true"),
        })
    }

    fn matches(&self, statement: &Value, statement_index: &BTreeMap<Uuid, Value>) -> bool {
        self.matches_time(statement)
            && self
                .actor_ifi
                .as_deref()
                .is_none_or(|actor_ifi| self.matches_agent(statement, statement_index, actor_ifi))
            && self.verb_id.as_deref().is_none_or(|verb_id| {
                statement.pointer("/verb/id").and_then(Value::as_str) == Some(verb_id)
                    || Self::matches_referenced_statement(statement, statement_index, |target| {
                        target.pointer("/verb/id").and_then(Value::as_str) == Some(verb_id)
                    })
            })
            && self.activity_id.as_deref().is_none_or(|activity_id| {
                self.matches_activity(statement, statement_index, activity_id)
            })
            && self.registration.is_none_or(|registration| {
                statement
                    .pointer("/context/registration")
                    .and_then(Value::as_str)
                    .and_then(|value| Uuid::parse_str(value).ok())
                    == Some(registration)
                    || Self::matches_referenced_statement(statement, statement_index, |target| {
                        target
                            .pointer("/context/registration")
                            .and_then(Value::as_str)
                            .and_then(|value| Uuid::parse_str(value).ok())
                            == Some(registration)
                    })
            })
    }

    fn matches_time(&self, statement: &Value) -> bool {
        let Some(stored_at) = statement_stored_at(statement) else {
            return false;
        };
        self.since.is_none_or(|since| stored_at > since)
            && self.until.is_none_or(|until| stored_at <= until)
    }

    fn matches_agent(
        &self,
        statement: &Value,
        statement_index: &BTreeMap<Uuid, Value>,
        actor_ifi: &str,
    ) -> bool {
        self.matches_agent_without_statement_ref(statement, actor_ifi)
            || Self::matches_referenced_statement(statement, statement_index, |target| {
                self.matches_agent_without_statement_ref(target, actor_ifi)
            })
    }

    fn matches_agent_without_statement_ref(&self, statement: &Value, actor_ifi: &str) -> bool {
        actor_value_matches_ifi(statement.get("actor"), actor_ifi)
            || actor_value_matches_ifi(statement.get("object"), actor_ifi)
            || (self.related_agents
                && related_agent_matches_statement_or_substatement(statement, actor_ifi))
    }

    fn matches_activity(
        &self,
        statement: &Value,
        statement_index: &BTreeMap<Uuid, Value>,
        activity_id: &str,
    ) -> bool {
        if statement.pointer("/object/id").and_then(Value::as_str) == Some(activity_id) {
            return true;
        }
        if !self.related_activities {
            return Self::matches_referenced_statement(statement, statement_index, |target| {
                target.pointer("/object/id").and_then(Value::as_str) == Some(activity_id)
            });
        }
        related_activity_matches_statement_or_substatement(statement, activity_id)
            || Self::matches_referenced_statement(statement, statement_index, |target| {
                self.matches_activity_without_statement_ref(target, activity_id)
            })
    }

    fn matches_activity_without_statement_ref(&self, statement: &Value, activity_id: &str) -> bool {
        statement.pointer("/object/id").and_then(Value::as_str) == Some(activity_id)
            || (self.related_activities
                && related_activity_matches_statement_or_substatement(statement, activity_id))
    }

    fn matches_referenced_statement(
        statement: &Value,
        statement_index: &BTreeMap<Uuid, Value>,
        predicate: impl Fn(&Value) -> bool,
    ) -> bool {
        let mut visited = BTreeSet::new();
        let mut current = statement;
        while let Some(statement_id) = referenced_statement_id(current) {
            if !visited.insert(statement_id) {
                return false;
            }
            let Some(target) = statement_index.get(&statement_id) else {
                return false;
            };
            if predicate(target) {
                return true;
            }
            current = target;
        }
        false
    }
}

fn parse_query_timestamp(
    value: &str,
    error: StatementFilterError,
) -> Result<OffsetDateTime, StatementFilterError> {
    OffsetDateTime::parse(value, &Rfc3339).map_err(|_error| error)
}

fn actor_value_matches_ifi(value: Option<&Value>, actor_ifi: &str) -> bool {
    let Some(value) = value else {
        return false;
    };
    canonical_actor_ifi(value).as_deref() == Some(actor_ifi)
        || value
            .get("member")
            .and_then(Value::as_array)
            .is_some_and(|members| {
                members
                    .iter()
                    .any(|member| actor_value_matches_ifi(Some(member), actor_ifi))
            })
}

fn related_agent_values(statement: &Value) -> impl Iterator<Item = &Value> {
    let single_values = [
        statement.get("authority"),
        statement.pointer("/context/instructor"),
        statement.pointer("/context/team"),
    ]
    .into_iter()
    .flatten();
    let collection_values = [
        (statement.pointer("/context/contextAgents"), "agent"),
        (statement.pointer("/context/contextGroups"), "group"),
    ]
    .into_iter()
    .flat_map(|(values, property)| {
        values
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(move |relation| relation.get(property))
    });
    single_values.chain(collection_values)
}

fn related_agent_matches_statement_or_substatement(statement: &Value, actor_ifi: &str) -> bool {
    related_agent_values(statement).any(|agent| actor_value_matches_ifi(Some(agent), actor_ifi))
        || substatement_value(statement).is_some_and(|substatement| {
            actor_value_matches_ifi(substatement.get("actor"), actor_ifi)
                || actor_value_matches_ifi(substatement.get("object"), actor_ifi)
                || related_agent_values(substatement)
                    .any(|agent| actor_value_matches_ifi(Some(agent), actor_ifi))
        })
}

fn statement_related_activity_ids(statement: &Value) -> impl Iterator<Item = &str> {
    std::iter::once(statement.pointer("/object/id").and_then(Value::as_str))
        .flatten()
        .chain(
            statement
                .pointer("/context/contextActivities")
                .into_iter()
                .flat_map(context_activity_ids),
        )
}

fn related_activity_matches_statement_or_substatement(
    statement: &Value,
    activity_id: &str,
) -> bool {
    statement_related_activity_ids(statement).any(|id| id == activity_id)
        || substatement_value(statement).is_some_and(|substatement| {
            statement_related_activity_ids(substatement).any(|id| id == activity_id)
        })
}

fn context_activity_ids(value: &Value) -> impl Iterator<Item = &str> {
    // Both accepted representations name Activities directly. Their definitions
    // and extension values do not introduce additional related Activities.
    ["parent", "grouping", "category", "other"]
        .into_iter()
        .filter_map(move |role| value.get(role))
        .flat_map(|activity| {
            std::iter::once(activity)
                .filter(|value| value.is_object())
                .chain(activity.as_array().into_iter().flatten())
        })
        .filter_map(activity_id)
}

fn activity_id(value: &Value) -> Option<&str> {
    value.get("id").and_then(Value::as_str)
}

fn substatement_value(statement: &Value) -> Option<&Value> {
    if statement
        .pointer("/object/objectType")
        .and_then(Value::as_str)
        == Some("SubStatement")
    {
        statement.get("object")
    } else {
        None
    }
}

fn referenced_statement_id(statement: &Value) -> Option<Uuid> {
    if statement
        .pointer("/object/objectType")
        .and_then(Value::as_str)
        != Some("StatementRef")
    {
        return None;
    }
    let statement_id = statement.pointer("/object/id").and_then(Value::as_str)?;
    Uuid::parse_str(statement_id).ok()
}
