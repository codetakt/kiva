//! RFC4518 caseIgnoreMatch preparation with the pinned Unicode 3.2 repertoire.
//! This comparison view is never used as certificate signature input.

#[cfg(not(feature = "std"))]
use alloc::{string::String, vec::Vec};

mod unicode32;
use unicode32::{COMBINING, COMPOSE, DECOMPOSE, FOLD, MARKS, PROHIBITED,
    CANONICAL_DECOMPOSE, B1, NAMEPREP_PROHIBITED, RANDAL, LCAT};

fn in_ranges(point: u32, ranges: &[(u32, u32)]) -> bool {
    let index = ranges.partition_point(|(_, end)| *end < point);
    ranges.get(index).is_some_and(|(start, _)| *start <= point)
}

fn ccc(point: u32) -> u8 {
    COMBINING.binary_search_by_key(&point, |(point, _)| *point).ok().map_or(0, |index| COMBINING[index].1)
}

fn mapping(point: u32) -> Option<u32> {
    match point {
        0x9..=0xd | 0x85 | 0x20 | 0xa0 | 0x1680 | 0x2000..=0x200a | 0x2028..=0x2029 | 0x202f | 0x205f | 0x3000 => Some(0x20),
        0xad | 0x1806 | 0x34f | 0x180b..=0x180d | 0xfe00..=0xfe0f | 0xfffc |
        0..=8 | 0xe..=0x1f | 0x7f..=0x84 | 0x86..=0x9f | 0x6dd | 0x70f | 0x180e |
        0x200b..=0x200f | 0x202a..=0x202e | 0x2060..=0x2063 | 0x206a..=0x206f | 0xfeff |
        0xfff9..=0xfffb | 0x1d173..=0x1d17a | 0xe0001 | 0xe0020..=0xe007f => None,
        other => Some(other),
    }
}

fn decompose(point: u32, output: &mut Vec<u32>, compatibility: bool) {
    let table = if compatibility { DECOMPOSE } else { CANONICAL_DECOMPOSE };
    if (0xac00..0xd7a4).contains(&point) {
        let syllable = point - 0xac00;
        output.push(0x1100 + syllable / 588);
        output.push(0x1161 + (syllable % 588) / 28);
        if syllable % 28 != 0 { output.push(0x11a7 + syllable % 28); }
    } else if let Ok(index) = table.binary_search_by_key(&point, |(point, _)| *point) {
        output.extend_from_slice(table[index].1);
    } else { output.push(point); }
}

fn compose(left: u32, right: u32) -> Option<u32> {
    if (0x1100..0x1113).contains(&left) && (0x1161..0x1176).contains(&right) {
        return Some(0xac00 + (left - 0x1100) * 588 + (right - 0x1161) * 28);
    }
    if (0xac00..0xd7a4).contains(&left) && (left - 0xac00) % 28 == 0 && (0x11a8..0x11c3).contains(&right) {
        return Some(left + right - 0x11a7);
    }
    COMPOSE.binary_search_by_key(&(left, right), |(pair, _)| *pair).ok().map(|index| COMPOSE[index].1)
}

fn normalized(points: Vec<u32>) -> Vec<u32> {
    normalized_with(points, true)
}

fn normalized_with(points: Vec<u32>, compatibility: bool) -> Vec<u32> {
    let mut ordered = Vec::new();
    for point in points {
        let start = ordered.len();
        decompose(point, &mut ordered, compatibility);
        for index in start..ordered.len() {
            let class = ccc(ordered[index]);
            let mut place = index;
            while class != 0 && place > 0 && ccc(ordered[place - 1]) > class {
                ordered.swap(place, place - 1); place -= 1;
            }
        }
    }
    let mut output = Vec::new();
    let mut starter = None;
    let mut previous_class = 0;
    for point in ordered {
        let class = ccc(point);
        if let Some(index) = starter {
            if previous_class == 0 || previous_class < class {
                if let Some(composite) = compose(output[index], point) {
                    output[index] = composite; continue;
                }
            }
        }
        if class == 0 { starter = Some(output.len()); }
        output.push(point); previous_class = class;
    }
    output
}

pub(super) fn normalize(value: &str) -> String {
    // Decomposition/composition tables contain Unicode scalar values only.
    normalized(value.chars().map(u32::from).collect())
        .into_iter().filter_map(char::from_u32).collect()
}

pub(super) fn nfc(value: &str) -> String {
    normalized_with(value.chars().map(u32::from).collect(), false)
        .into_iter().filter_map(char::from_u32).collect()
}

pub(super) fn nameprep(value: &str) -> Option<String> {
    let mut mapped = Vec::new();
    for point in value.chars().map(u32::from).filter(|p| !in_ranges(*p, B1)) {
        if let Ok(index) = FOLD.binary_search_by_key(&point, |(point, _)| *point) {
            mapped.extend_from_slice(FOLD[index].1);
        } else { mapped.push(point); }
    }
    let normalized = normalized(mapped);
    if normalized.iter().any(|p| in_ranges(*p, NAMEPREP_PROHIBITED)) { return None; }
    if normalized.iter().any(|p| in_ranges(*p, RANDAL)) &&
        (normalized.iter().any(|p| in_ranges(*p, LCAT))
         || !normalized.first().is_some_and(|p| in_ranges(*p, RANDAL))
         || !normalized.last().is_some_and(|p| in_ranges(*p, RANDAL))) { return None; }
    normalized.into_iter().map(char::from_u32).collect()
}

pub(super) fn prepare(value: &[u8]) -> Option<String> {
    let text = core::str::from_utf8(value).ok()?;
    let mut mapped = Vec::new();
    for point in text.chars().filter_map(|value| mapping(u32::from(value))) {
        if let Ok(index) = FOLD.binary_search_by_key(&point, |(point, _)| *point) { mapped.extend_from_slice(FOLD[index].1); }
        else { mapped.push(point); }
    }
    let normalized = normalized(mapped);
    if normalized.iter().any(|point| in_ranges(*point, PROHIBITED)) { return None; }
    // RFC4518 section 2.5 deliberately imposes no bidi condition.
    // A SPACE followed by any combining mark is significant (section 2.6.1).
    let insignificant = |index: usize| normalized[index] == 0x20 && !normalized.get(index + 1).is_some_and(|point| in_ranges(*point, MARKS));
    let first = (0..normalized.len()).find(|index| !insignificant(*index));
    let last = (0..normalized.len()).rfind(|index| !insignificant(*index));
    let (Some(first), Some(last)) = (first, last) else { return Some(String::from("  ")); };
    let mut output = String::from(" ");
    let mut index = first;
    while index <= last {
        if insignificant(index) {
            output.push_str("  ");
            while index <= last && insignificant(index) { index += 1; }
        } else { output.push(char::from_u32(normalized[index])?); index += 1; }
    }
    output.push(' ');
    Some(output)
}
