//! Durable first-encounter metadata work. External failures never undo admission.

use std::{collections::BTreeSet, sync::Arc, time::Duration};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::OffsetDateTime;
use url::Url;

use super::{CapabilityConfig, canonical::Definition, store::Record};
use crate::core::{
    response::{HttpError, new_etag},
    state::XapiHttpContext,
};

#[derive(Serialize, Deserialize)]
struct Job {
    next_attempt: i64,
    complete: bool,
}

fn job_bytes(complete: bool, next_attempt: i64) -> Vec<u8> {
    serde_json::json!({"complete":complete,"next_attempt":next_attempt})
        .to_string()
        .into_bytes()
}

fn now() -> i64 {
    OffsetDateTime::now_utc().unix_timestamp()
}

fn allowed(url: &Url, config: &CapabilityConfig) -> bool {
    matches!(url.scheme(), "http" | "https")
        && url.username().is_empty()
        && url.password().is_none()
        && config
            .metadata_origins
            .contains(&url.origin().ascii_serialization())
}

fn activity_ids(statement: &Value, ids: &mut BTreeSet<String>) {
    let object = &statement["object"];
    match object["objectType"].as_str().unwrap_or("Activity") {
        "Activity" => {
            if let Some(id) = object["id"].as_str() {
                ids.insert(id.to_owned());
            }
        }
        "SubStatement" => activity_ids(object, ids),
        _ => {}
    }
    for role in ["parent", "grouping", "category", "other"] {
        if let Some(activities) = statement.pointer(&format!("/context/contextActivities/{role}")) {
            for activity in activities
                .as_array()
                .map_or_else(|| vec![activities], |a| a.iter().collect())
            {
                if let Some(id) = activity["id"].as_str() {
                    ids.insert(id.to_owned());
                }
            }
        }
    }
}

pub(crate) async fn encountered(state: &XapiHttpContext, statements: &[Value]) {
    let config = &state.capabilities().config;
    if config.metadata_origins.is_empty() {
        return;
    }
    let mut ids = BTreeSet::new();
    for statement in statements {
        activity_ids(statement, &mut ids);
    }
    ids.retain(|id| Url::parse(id).is_ok_and(|url| allowed(&url, config)));
    let result = state
        .capabilities()
        .store
        .mutate("fetch:", move |records| {
            for id in ids {
                records
                    .entry(format!("fetch:{id}"))
                    .or_insert_with(|| Record {
                        body: job_bytes(false, 0),
                        etag: new_etag(),
                    });
            }
            Ok(())
        })
        .await;
    if let Err(error) = result {
        tracing::error!(error=?error,"metadata work could not be queued");
    }
    kick(state);
}

pub(crate) fn kick(state: &XapiHttpContext) {
    use std::sync::atomic::Ordering;
    if state.capabilities().config.metadata_origins.is_empty() {
        return;
    }
    state
        .capabilities()
        .fetch_pending
        .store(true, Ordering::SeqCst);
    let Ok(guard) = Arc::clone(&state.capabilities().fetch_worker).try_lock_owned() else {
        return;
    };
    let state = state.clone();
    state
        .capabilities()
        .fetch_pending
        .store(false, Ordering::SeqCst);
    tokio::spawn(async move {
        if let Err(error) = drain(&state).await {
            tracing::error!(error=?error,"metadata work could not be completed");
        }
        drop(guard);
        // Recheck after releasing ownership so a concurrent enqueue cannot be lost.
        if state
            .capabilities()
            .fetch_pending
            .swap(false, Ordering::SeqCst)
        {
            kick(&state);
        }
    });
}

#[allow(clippy::too_many_lines)]
async fn drain(state: &XapiHttpContext) -> Result<(), HttpError> {
    let config = Arc::clone(&state.capabilities().config);
    let redirects = Arc::clone(&config);
    let client = match reqwest::Client::builder()
        .no_proxy()
        .timeout(Duration::from_millis(config.metadata_timeout_ms))
        .redirect(reqwest::redirect::Policy::custom(move |attempt| {
            if attempt.previous().len() >= 5 || !allowed(attempt.url(), &redirects) {
                attempt.stop()
            } else {
                attempt.follow()
            }
        }))
        .build()
    {
        Ok(client) => client,
        Err(error) => {
            tracing::error!(error=?error,"metadata HTTP client unavailable");
            return Ok(());
        }
    };
    // Claims are persistent leases, so multiple server processes cannot normally
    // fetch the same identifier simultaneously and a crashed process is recoverable.
    loop {
        let claimed = state
            .capabilities()
            .store
            .mutate("fetch:", |records| {
                for (key, record) in records {
                    let Ok(mut job) = serde_json::from_slice::<Job>(&record.body) else {
                        continue;
                    };
                    if job.complete || job.next_attempt > now() {
                        continue;
                    }
                    job.next_attempt =
                        now() + i64::try_from(config.metadata_timeout_ms / 1000 + 5).unwrap_or(35);
                    record.body = job_bytes(job.complete, job.next_attempt);
                    record.etag = new_etag();
                    return Ok(Some((
                        key.trim_start_matches("fetch:").to_owned(),
                        record.etag.clone(),
                    )));
                }
                Ok(None)
            })
            .await?;
        let Some((id, lease)) = claimed else {
            break;
        };
        let definition = tokio::time::timeout(
            Duration::from_millis(config.metadata_timeout_ms),
            retrieve(&client, &id, &config),
        )
        .await
        .ok()
        .and_then(Result::ok);
        let definition = definition.filter(|def| {
            !config.detailed_interactions || super::interaction::definition(&def.value).is_ok()
        });
        let success = if let Some(definition) = definition {
            let key = format!("metadata:{id}");
            state
                .capabilities()
                .store
                .mutate("metadata:", move |records| {
                    // A retry preserves any metadata fields already established by
                    // a successful owner response, allowing only small corrections.
                    let definition = match records.get(&key) {
                        Some(old) => Definition::parse(&old.body)?.corrected(&definition)?,
                        None => definition,
                    };
                    records.insert(
                        key,
                        Record {
                            body: definition.bytes()?,
                            etag: new_etag(),
                        },
                    );
                    Ok(())
                })
                .await
                .is_ok()
        } else {
            false
        };
        let retry_seconds = config.metadata_retry_seconds;
        state
            .capabilities()
            .store
            .mutate("fetch:", move |records| {
                if let Some(record) = records.get_mut(&format!("fetch:{id}"))
                    && record.etag == lease
                {
                    record.body = job_bytes(success, now() + i64::from(retry_seconds));
                }
                Ok(())
            })
            .await?;
    }
    Ok(())
}

async fn retrieve(
    client: &reqwest::Client,
    id: &str,
    config: &CapabilityConfig,
) -> Result<Definition, String> {
    let mut url = Url::parse(id).map_err(|e| e.to_string())?;
    for depth in 0..2 {
        if !allowed(&url, config) {
            return Err("metadata origin is not permitted".into());
        }
        let mut reply = client
            .get(url.clone())
            .header("Accept", "application/json, */*")
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if !reply.status().is_success() {
            return Err("metadata HTTP status was not successful".into());
        }
        if reply
            .content_length()
            .is_some_and(|n| n > config.metadata_max_bytes as u64)
        {
            return Err("metadata size exceeds limit".into());
        }
        let media = reply
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .split(';')
            .next()
            .unwrap_or("")
            .trim()
            .to_ascii_lowercase();
        let final_url = reply.url().clone();
        let mut bytes = Vec::new();
        while let Some(chunk) = reply.chunk().await.map_err(|e| e.to_string())? {
            if chunk.len() > config.metadata_max_bytes.saturating_sub(bytes.len()) {
                return Err("metadata size exceeds limit".into());
            }
            bytes.extend_from_slice(&chunk);
        }
        if media == "application/json" {
            return Definition::parse(&bytes).map_err(|e| e.message);
        }
        if media == "text/html" && config.metadata_html {
            match html(&bytes)? {
                HtmlMetadata::Definition(definition) => return Ok(definition),
                HtmlMetadata::Alternate(href) if depth == 0 => {
                    url = final_url.join(&href).map_err(|e| e.to_string())?;
                    continue;
                }
                HtmlMetadata::Alternate(_) => {}
            }
        }
        return Err("document has no supported Activity Definition".into());
    }
    Err("metadata alternate depth exceeded".into())
}

enum HtmlMetadata {
    Definition(Definition),
    Alternate(String),
}

fn html(bytes: &[u8]) -> Result<HtmlMetadata, String> {
    let text = std::str::from_utf8(bytes).map_err(|e| e.to_string())?;
    let document = scraper::Html::parse_document(text);
    let embedded = scraper::Selector::parse("script[data-xapi-activity-definition]")
        .map_err(|error| error.to_string())?;
    for element in document.select(&embedded) {
        if element
            .value()
            .attr("type")
            .is_some_and(|v| v.eq_ignore_ascii_case("application/json"))
        {
            return Definition::parse(element.text().collect::<String>().as_bytes())
                .map(HtmlMetadata::Definition)
                .map_err(|e| e.message);
        }
    }
    let links =
        scraper::Selector::parse("link[rel][type][href]").map_err(|error| error.to_string())?;
    for element in document.select(&links) {
        let element = element.value();
        if element.attr("rel").is_some_and(|v| {
            v.split_ascii_whitespace()
                .any(|word| word.eq_ignore_ascii_case("alternate"))
        }) && element
            .attr("type")
            .is_some_and(|v| v.eq_ignore_ascii_case("application/json"))
        {
            return Ok(HtmlMetadata::Alternate(
                element.attr("href").unwrap_or_default().to_owned(),
            ));
        }
    }
    Err("HTML has no embedded or alternate Activity Definition".into())
}
