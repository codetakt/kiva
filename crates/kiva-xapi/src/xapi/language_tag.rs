//! RFC 5646 syntax with the same dated IANA inventory as DC-LANGUAGE.
//!
//! The parser's built-in registry can age independently of the test contract.
//! Use it for syntax only and check membership against the pinned inventory.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

use language_tags::LanguageTag;

const REGISTRY: &str =
    include_str!("../../../../spec/conformance/reference/language-subtag-registry-20260917.txt");

#[derive(Default)]
struct Registry {
    entries: BTreeSet<(String, String)>,
    ranges: Vec<(String, String, String)>,
    extlang_prefixes: BTreeMap<String, String>,
}

impl Registry {
    fn load() -> Self {
        let mut registry = Self::default();
        for record in REGISTRY.split("%%") {
            let fields: BTreeMap<_, _> = record
                .lines()
                .filter_map(|line| line.split_once(": "))
                .collect();
            let Some(kind) = fields.get("Type") else {
                continue;
            };
            let Some(value) = fields.get("Subtag").or_else(|| fields.get("Tag")) else {
                continue;
            };
            let value = value.to_ascii_lowercase();
            if let Some((start, end)) = value.split_once("..") {
                registry
                    .ranges
                    .push(((*kind).to_owned(), start.to_owned(), end.to_owned()));
            } else {
                registry.entries.insert(((*kind).to_owned(), value.clone()));
            }
            if *kind == "extlang"
                && let Some(prefix) = fields.get("Prefix")
            {
                registry
                    .extlang_prefixes
                    .insert(value, prefix.to_ascii_lowercase());
            }
        }
        registry
    }

    fn contains(&self, kind: &str, value: &str) -> bool {
        let value = value.to_ascii_lowercase();
        self.entries.contains(&(kind.to_owned(), value.clone()))
            || self.ranges.iter().any(|(category, start, end)| {
                category == kind && value.len() == start.len() && start <= &value && &value <= end
            })
    }
}

pub(super) fn valid_language_tag(value: &str) -> bool {
    static INVENTORY: OnceLock<Registry> = OnceLock::new();
    // Apply lengths to private-use subtags too; they have no registry entries.
    if !value.split('-').all(|part| {
        (1..=8).contains(&part.len()) && part.bytes().all(|byte| byte.is_ascii_alphanumeric())
    }) {
        return false;
    }
    let Ok(tag) = LanguageTag::parse(value) else {
        return false;
    };
    let lower = value.to_ascii_lowercase();
    if lower.starts_with("x-") {
        return true;
    }
    let registry = INVENTORY.get_or_init(Registry::load);
    if registry.contains("grandfathered", &lower) {
        return true;
    }
    if !registry.contains("language", tag.primary_language()) {
        return false;
    }
    if let Some(extlang) = tag.extended_language()
        && registry.extlang_prefixes.get(extlang).map(String::as_str)
            != Some(tag.primary_language())
    {
        return false;
    }
    if tag
        .script()
        .is_some_and(|script| !registry.contains("script", script))
        || tag
            .region()
            .is_some_and(|region| !registry.contains("region", region))
    {
        return false;
    }
    let mut variants = BTreeSet::new();
    if tag
        .variant_subtags()
        .any(|variant| !registry.contains("variant", variant) || !variants.insert(variant))
    {
        return false;
    }
    // Variant Prefix fields recommend usage; they are not a new rejection rule.
    let mut singletons = BTreeSet::new();
    tag.extension_subtags()
        .all(|(singleton, _)| singletons.insert(singleton))
}

#[cfg(test)]
mod tests {
    use super::{REGISTRY, valid_language_tag};

    #[test]
    fn validates_registered_grandfathered_and_private_language_tags() {
        assert!(REGISTRY.starts_with("File-Date: 2026-09-17\n"));
        for tag in [
            "en",
            "JA-jp",
            "zh-Hant",
            "zh-cmn-Hans-CN",
            "i-klingon",
            "en-GB-oed",
            "x-private",
            "X-a-12345678",
            "en-x-private",
            "sl-rozaj",
            "de-1901",
            "qaa-Qaaa-QM",
            "en-u-ca-gregory",
            "en-rozaj",
        ] {
            assert!(valid_language_tag(tag), "valid: {tag}");
        }
        for tag in [
            "",
            "abcdefgh",
            "en_Us",
            "en--US",
            "en-",
            "x",
            "x-",
            "x-123456789",
            "en-x-123456789",
            "sl-rozaj-rozaj",
            "en-a-foo-a-bar",
            "en-Zzzz-999",
            "en-cmn",
            "zh-cmn-yue",
            "en-unlisted",
            "日本語",
        ] {
            assert!(!valid_language_tag(tag), "invalid: {tag}");
        }
    }
}
