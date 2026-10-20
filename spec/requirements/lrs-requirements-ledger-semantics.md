# Reading detailed LRS rule records

Read `actor`, `strength`, `when`, `requirement`, `exceptions`, `interpretation`
and `issues` together. A rule's presence in this ledger is not a claim that its
effect is an unconditional LRS rejection requirement. The fixed source is IEEE
Std 9274.1.1-2023; these field conventions describe its projection.

## Strength and polarity

| Value | Meaning in this ledger |
| --- | --- |
| `shall` | A mandatory requirement for the named actor, within the recorded condition and exceptions. An LRP `shall` does not by itself prescribe an LRS error response. |
| `shall-not` | A prohibition on the action named in `requirement`, within the recorded scope. |
| `should` | A recommendation; it must not become a mandatory receiver rejection. |
| `should-not` | A recommendation against the named action, or a Not Recommended table-presence facet. |
| `may` | An expressly permitted course of action. |
| `permission` | Optionality or permission represented by a table or other source prose; absence of an optional value does not itself violate the rule. |
| `data-contract` | A type, value, shape, identity, meaning or resource-behavior definition. This is not a synonym for optional or advisory. Binding supplied-value constraints and resource semantics remain binding where applicable. Data meaning alone does not require the LRS to verify the truth of an experience. |
| `context` | Supporting explanation or role context, without an additional independently graded duty merely from this label. |
| `scope` | An applicability or responsibility boundary. |
| `unresolved` | Competing source requirements or meanings are retained without choosing a universal outcome; consult the named OPEN/POL issue. |

These are projection categories, not a transcription of every original modal
word. For example, optional query parameters may select required response
behavior: `IP-get-param-attachments` is a `data-contract`, but its true/false
response semantics are not optional. Table type/value constraints apply when the
enclosing object and supplied property apply, under the source's table-validation
requirements. A definition for a producer/consumer is not a new LRS obligation
to judge domain truth.

For action-form prohibitions, the negative polarity is carried by `strength`.
`IP-document-precondition-no-write` combines `shall-not` with “Modify the
resource.” `IP-version-admission` combines `should-not` with rejecting a
Statement solely for its version. Read those as prohibiting or discouraging the
action, not requiring it.

Some table-presence descriptions already say “not recommended” and then explain
that absence is not grounds for rejection. Those are complete descriptions of
the same negative recommendation, not instructions to negate the prose again.
Do not implement polarity by mechanically inserting or removing the word “not”
from arbitrary text. Supporting sentences and subordinate clauses retain their
own meaning.

`exceptions` also carries source-conflict reservations and scope qualifications,
as well as ordinary exceptions. For example, the multipart PUT reservation
preserves `OPEN-PUT`; it does not remove the POST admission duty or select a PUT
array policy. `interpretation: unresolved` and a named issue must survive later
test projection even when `strength` retains a source `shall`.

## Source wording, explanations and links

`source-stated` includes faithful paraphrases and direct consequences of the
source; it does not mean that every word is a literal quote. Rejecting every
batch with duplicate IDs includes equal-body duplicates without an additional
project policy. A separate choice between competing readings belongs under
`project-interpretation` or `unresolved`, as appropriate.

`source_units` can include a direct provision and supporting role, exception,
table or conflicting-method context. Listing a contextual unit does not claim
that it independently states the whole rule. Examples include the chapter-4
context and chapter-5 Group provision for `IP-document-group-ifi`, and the
general syntax/meaning boundary in 4.1 for interaction and response-format
rules. Source-unit `counterpart` is a navigation pointer, not proof that every
proposition resides in that one target unit.

## Family and issue boundaries

Family `level` is a historical coarse classification. It does not override
detailed actor, recommendation, permission or unresolved fields. Thirteen
`shall` families in the PR baseline have no linked LRS `shall`/`shall-not`
record: query filters and related-agent/activity filters, time/order/limit,
exact/ids/canonical formats, Activity completeness, Statement version, Agent
IFI identity/formats, Result properties and Score properties. Their definitions,
recommendations and supplied-value constraints still need to be read; the
absence of the literal `shall` tag is not proof of a missing duty.

Issue-to-rule reverse links list explicit `rules[].issues` references. Two
questions intentionally have no direct rule link:

- `OPEN-EXT` is a cross-cutting dependency integration question carried by the
  dependency register and comparison lineage.
- `OPEN-IRI-STRENGTH` concerns producer IRI-governance prose. Its open state
  does not create an LRS ownership, resolution or network-access rejection duty.

Do not attach these issues to unrelated LRS rules solely to eliminate an empty
reverse-link list. Conversely, the multipart PUT applicability question directly
qualifies `IP-multipart-accept` and is explicitly linked there.

## Scope of role context and verification

The ledger covers LRS requirements and supporting role boundaries, not every LRP
recommendation or every explanatory meaning facet. Optional suggestions to add
producer-only rules do not by themselves demonstrate a missing LRS obligation.
Producer/consumer facets are included selectively where they explain LRS-visible
data or distinguish a producer duty from receiver behavior. For example,
`IP-statement-put-id-match` states valid request shape while preserving the lack
of an independently specified receiver status. This is not an exhaustive catalog
of LRP request-shape rules, recommendations or internal responsibilities.
Required presence and a “when present” value restriction can coexist: the first
requires the property and the second constrains its value when supplied.

For current publication state, file identities and validation limits, use
`lrs-requirements-ledger-publication.json` and `lrs-requirements-ledger/ledger.json`.
Publication does not establish runtime conformance or resolve the OPEN/POL questions.

Use the documented `python -B scripts/ci/...` commands with the existing
Nix-provided packages. These repository scripts import sibling modules;
isolated `python -I` invocation is not their supported entry point. Static
parsing, hashes and reference checks do not establish semantic correctness,
execution success or implementation conformance.
