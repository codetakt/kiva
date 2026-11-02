//! Canonical metadata is a projection; immutable Statement values never change.

use std::collections::BTreeMap;

use axum::http::StatusCode;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::core::{response::HttpError, state::XapiHttpContext};
use crate::surfaces::xapi::{
    lossless::{self, OpaqueValues},
    service,
};

pub(crate) const INTERACTION_FIELDS: [&str; 7] = [
    "interactionType",
    "correctResponsesPattern",
    "choices",
    "scale",
    "source",
    "target",
    "steps",
];

// Interaction properties describe one experience. Filling missing fields from
// another definition must not splice its answers into a different kind or set
// of components. Language additions remain independently mergeable.
pub(crate) fn compatible_interactions(existing: &Value, incoming: &Value) -> bool {
    for key in ["interactionType", "correctResponsesPattern"] {
        if let (Some(old), Some(new)) = (existing.get(key), incoming.get(key))
            && old != new
        {
            return false;
        }
    }
    for key in ["choices", "scale", "source", "target", "steps"] {
        if let (Some(old), Some(new)) = (
            existing.get(key).and_then(Value::as_array),
            incoming.get(key).and_then(Value::as_array),
        ) {
            let ids = |array: &[Value]| {
                array
                    .iter()
                    .filter_map(|item| item.get("id").and_then(Value::as_str))
                    .map(str::to_owned)
                    .collect::<std::collections::BTreeSet<_>>()
            };
            if ids(old) != ids(new) {
                return false;
            }
        }
    }
    let mut candidate = existing.clone();
    for key in INTERACTION_FIELDS {
        if candidate.get(key).is_none()
            && let Some(value) = incoming.get(key)
        {
            candidate[key] = value.clone();
        }
    }
    // Optional admission validation can be disabled. Do not reclassify the
    // stored definitions here, but never turn two valid definitions into an
    // invalid one by combining them (including partially specified patterns).
    super::interaction::definition(existing).is_err()
        || super::interaction::definition(incoming).is_err()
        || super::interaction::definition(&candidate).is_ok()
}

#[derive(Debug, Clone)]
pub(crate) struct Definition {
    pub(crate) value: Value,
    pub(crate) opaque: OpaqueValues,
}

impl Definition {
    pub(crate) fn parse(bytes: &[u8]) -> Result<Self, HttpError> {
        let mut wrapped=b"{\"verb\":{\"id\":\"urn:kiva:metadata\"},\"object\":{\"id\":\"urn:kiva:metadata-validation\",\"definition\":".to_vec();
        wrapped.extend_from_slice(bytes);
        wrapped.extend_from_slice(b"}}");
        let (statement, opaque) = lossless::parse_statement(&wrapped, "Activity Definition")?;
        kiva_xapi::validate_statement_object_model(&lossless::contract_view(&statement))
            .map_err(|error| HttpError::new(StatusCode::BAD_REQUEST, error.to_string()))?;
        Ok(Self {
            value: statement["object"]["definition"].clone(),
            opaque: lossless::subtree(&opaque, "/object/definition"),
        })
    }

    pub(crate) fn bytes(&self) -> Result<Vec<u8>, HttpError> {
        lossless::render(&self.value, &self.opaque).map_err(|_| {
            HttpError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Activity Definition could not be serialized",
            )
        })
    }

    pub(crate) fn corrected(&self, incoming: &Self) -> Result<Self, HttpError> {
        if !compatible_interactions(&self.value, &incoming.value) {
            return Err(HttpError::new(
                StatusCode::CONFLICT,
                "incompatible interaction changes require a new identifier",
            ));
        }
        for key in ["type", "moreInfo"] {
            if let (Some(old), Some(new)) = (self.value.get(key), incoming.value.get(key))
                && old != new
            {
                return Err(HttpError::new(
                    StatusCode::CONFLICT,
                    "significant Activity changes require a new identifier",
                ));
            }
        }
        let mut merged = self.clone();
        merged.merge(incoming, true);
        Ok(merged)
    }

    fn merge(&mut self, incoming: &Self, may_correct: bool) {
        let compatible = compatible_interactions(&self.value, &incoming.value);
        let Some(source) = incoming.value.as_object() else {
            return;
        };
        let Some(target) = self.value.as_object_mut() else {
            return;
        };
        for (key, value) in source {
            if !compatible && INTERACTION_FIELDS.contains(&key.as_str()) {
                continue;
            }
            if !target.contains_key(key) {
                target.insert(key.clone(), value.clone());
                if key == "extensions" {
                    self.opaque.extend(incoming.opaque.clone());
                }
                continue;
            }
            if key == "name" || key == "description" {
                if let Some(current) = target.get_mut(key) {
                    merge_languages(current, value, may_correct);
                }
            } else if key == "extensions" {
                if let (Some(current), Some(additions)) = (
                    target.get_mut(key).and_then(Value::as_object_mut),
                    value.as_object(),
                ) {
                    for (owner, content) in additions {
                        if !current.contains_key(owner) {
                            current.insert(owner.clone(), content.clone());
                            let path = lossless::pointer("/extensions", owner);
                            if let Some(raw) = incoming.opaque.get(&path) {
                                self.opaque.insert(path, raw.clone());
                            }
                        }
                    }
                }
            } else if ["choices", "scale", "source", "target", "steps"].contains(&key.as_str())
                && let Some(current) = target.get_mut(key)
            {
                merge_components(current, value, may_correct);
            }
            // Type, interaction kind, correct responses and other established
            // semantics stay stable. A different experience requires a new IRI.
        }
    }
}

fn merge_languages(target: &mut Value, source: &Value, may_correct: bool) {
    if let (Some(target), Some(source)) = (target.as_object_mut(), source.as_object()) {
        for (language, value) in source {
            if may_correct || !target.contains_key(language) {
                target.insert(language.clone(), value.clone());
            }
        }
    }
}

pub(crate) fn merge_components(target: &mut Value, source: &Value, may_correct: bool) {
    let (Some(target), Some(source)) = (target.as_array_mut(), source.as_array()) else {
        return;
    };
    for component in target {
        if let Some(update) = source
            .iter()
            .find(|item| item.get("id") == component.get("id"))
            && let Some(description) = update.get("description")
        {
            if let Some(current) = component.get_mut("description") {
                merge_languages(current, description, may_correct);
            } else {
                component["description"] = description.clone();
            }
        }
    }
}

#[derive(Debug, Default)]
pub(crate) struct Inventory {
    pub(crate) activities: BTreeMap<String, Definition>,
    verbs: BTreeMap<String, Value>,
}

impl Inventory {
    pub(crate) async fn load(state: &XapiHttpContext) -> Result<Self, HttpError> {
        let sources = service::canonical_sources(state.xapi_services())
            .await
            .map_err(|_| {
                HttpError::new(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "canonical metadata could not be read",
                )
            })?;
        let all = sources
            .iter()
            .map(|source| source.statement.clone())
            .collect::<Vec<_>>();
        super::fetch::encountered(state, &all).await;
        let mut inventory = Self::default();
        for source in sources {
            if state.capabilities().config.rollback_voided_metadata
                && source.statement["id"]
                    .as_str()
                    .and_then(|s| Uuid::parse_str(s).ok())
                    .is_some_and(|id| kiva_xapi::statement_is_voided(&all, id))
            {
                continue;
            }
            inventory.add_statement(
                &source.statement,
                "",
                &source.opaque_values,
                source.may_update,
            );
        }
        for (key, record) in state.capabilities().store.list("metadata:").await? {
            let id = key.trim_start_matches("metadata:");
            let authoritative = Definition::parse(&record.body)?;
            inventory.add_authoritative(id, authoritative);
        }
        for (key, record) in state.capabilities().store.list("host:").await? {
            if !state.capabilities().config.activity_hosting {
                continue;
            }
            let Some(base) = &state.capabilities().config.public_base_url else {
                continue;
            };
            let id = format!(
                "{base}/extensions/activities/{}",
                key.trim_start_matches("host:")
            );
            inventory.add_authoritative(&id, Definition::parse(&record.body)?);
        }
        Ok(inventory)
    }

    fn add_authoritative(&mut self, id: &str, mut source: Definition) {
        // The identifier owner's hosted document is the canonical source;
        // preserve previously known fields/languages omitted by that source.
        if let Some(previous) = self.activities.remove(id) {
            source.merge(&previous, false);
        }
        self.activities.insert(id.to_owned(), source);
    }

    fn add_statement(
        &mut self,
        statement: &Value,
        path: &str,
        opaque: &OpaqueValues,
        may_correct: bool,
    ) {
        if let (Some(id), Some(display)) = (
            statement.pointer("/verb/id").and_then(Value::as_str),
            statement.pointer("/verb/display"),
        ) {
            if let Some(current) = self.verbs.get_mut(id) {
                merge_languages(current, display, may_correct);
            } else {
                self.verbs.insert(id.to_owned(), display.clone());
            }
        }
        if let Some(object) = statement.get("object") {
            if object["objectType"] == "SubStatement" {
                self.add_statement(object, &format!("{path}/object"), opaque, may_correct);
            } else {
                self.add_activity(object, &format!("{path}/object"), opaque, may_correct);
            }
        }
        for role in ["parent", "grouping", "category", "other"] {
            let local = format!("/context/contextActivities/{role}");
            if let Some(value) = statement.pointer(&local) {
                if let Some(array) = value.as_array() {
                    for (index, activity) in array.iter().enumerate() {
                        self.add_activity(
                            activity,
                            &format!("{path}{local}/{index}"),
                            opaque,
                            may_correct,
                        );
                    }
                } else {
                    self.add_activity(value, &format!("{path}{local}"), opaque, may_correct);
                }
            }
        }
    }

    fn add_activity(
        &mut self,
        activity: &Value,
        path: &str,
        opaque: &OpaqueValues,
        may_correct: bool,
    ) {
        if activity
            .get("objectType")
            .is_some_and(|kind| kind != "Activity")
        {
            return;
        }
        let (Some(id), Some(value)) = (
            activity.get("id").and_then(Value::as_str),
            activity.get("definition"),
        ) else {
            return;
        };
        let incoming = Definition {
            value: value.clone(),
            opaque: lossless::subtree(opaque, &format!("{path}/definition")),
        };
        if let Some(current) = self.activities.get_mut(id) {
            current.merge(&incoming, may_correct);
        } else {
            self.activities.insert(id.to_owned(), incoming);
        }
    }

    pub(crate) fn apply(&self, statement: &mut Value, opaque: &mut OpaqueValues) {
        self.apply_statement(statement, "", opaque);
    }

    fn apply_statement(&self, statement: &mut Value, path: &str, opaque: &mut OpaqueValues) {
        if let Some(display) = statement
            .pointer("/verb/id")
            .and_then(Value::as_str)
            .and_then(|id| self.verbs.get(id))
        {
            statement["verb"]["display"] = display.clone();
        }
        if let Some(object) = statement.get_mut("object") {
            if object["objectType"] == "SubStatement" {
                self.apply_statement(object, &format!("{path}/object"), opaque);
            } else {
                self.apply_activity(object, &format!("{path}/object"), opaque);
            }
        }
        for role in ["parent", "grouping", "category", "other"] {
            let local = format!("/context/contextActivities/{role}");
            if let Some(value) = statement.pointer_mut(&local) {
                if let Some(array) = value.as_array_mut() {
                    for (index, activity) in array.iter_mut().enumerate() {
                        self.apply_activity(activity, &format!("{path}{local}/{index}"), opaque);
                    }
                } else {
                    self.apply_activity(value, &format!("{path}{local}"), opaque);
                }
            }
        }
    }

    fn apply_activity(&self, activity: &mut Value, path: &str, opaque: &mut OpaqueValues) {
        if activity
            .get("objectType")
            .is_some_and(|kind| kind != "Activity")
        {
            return;
        }
        let Some(definition) = activity
            .get("id")
            .and_then(Value::as_str)
            .and_then(|id| self.activities.get(id))
        else {
            return;
        };
        let prefix = format!("{path}/definition");
        opaque.retain(|key, _| !key.starts_with(&format!("{prefix}/")));
        lossless::extend_prefixed(opaque, &definition.opaque, &prefix);
        activity["definition"] = definition.value.clone();
    }

    pub(crate) fn activity_bytes(&self, id: &str) -> Result<Vec<u8>, HttpError> {
        let mut response = json!({"objectType":"Activity","id":id});
        let mut opaque = OpaqueValues::new();
        if let Some(definition) = self.activities.get(id) {
            response["definition"] = definition.value.clone();
            lossless::extend_prefixed(&mut opaque, &definition.opaque, "/definition");
        }
        lossless::render(&response, &opaque).map_err(|_| {
            HttpError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Activity metadata could not be serialized",
            )
        })
    }
}
