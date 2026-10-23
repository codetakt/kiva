# Comparing the IEEE issued edition and public LRS documentation (2026-10-05)

Current-policy update (2026-10-06): this document records the historical chapter 4 comparison. The current interpretation uses PDF 5.1.6.1 for related_activities and 5.1.6.6.5 for Activity Profile list scope. Those chapter 5 tables supersede the original overlay/list-exclusion readings. See [lrs-interpretation-policy.yaml](lrs-interpretation-policy.yaml); no external overlay or official adjudication is required or claimed.

The user-supplied `927411-2023.pdf` is the issued **IEEE Std 9274.1.1-2023**, approved by the IEEE SA Standards Board on March 30,2023. It has103 PDF pages, 1,551,523 bytes and is unencrypted. Checked edition marking, contents, clause 4 beginning/end and Annex A presence. This file was received from the user, not downloaded from IEEE in this work. Its identity with ISO/IEC/IEEE39274-1-1:2025 cannot be determined from this PDF.

Original SHA-256: `fa57dbaa0fbc0800b8c5a693af884032e782110c1f446ef865797fd0e0811526`. Original, extracted text, comparison data and visual-check images are stored under `target/ieee-pdf-reconciliation/20261005/`. Distribution assets exclude the original, full extraction and user-specific download markings. [lrs-published-source.yaml](lrs-published-source.yaml) records converters/settings, reading scope and public-document mappings.

Printed and PDF page numbers differ: p. below means printed page; the PDF-viewer page is one higher.

Read all LRS clause 4 (printed pp.16–53, PDF pp.17–54) in extracted text, plus normative terminology, references and common definitions. Visually checked judgment-sensitive State/Activity Profile listings, Statement GET filters, equivalence and identifier validation in the original. Compared paragraph/clause difference candidates from public documentation against the text. Lexical comparison assisted detection of wrapping, table reading-order and typography differences; it was not proof of equivalence.

| Location | Issued-edition observation | Test treatment |
| --- | --- | --- |
| §4.1.6.2.5, p.29 (PDF30) | State-list stateId description prohibits use, while Required says required unless since. §4.1.6.2 prose returns a list when absent. The PDF contains this table conflict. | Follow prose and omit stateId in list GET. |
| §4.1.6.6.5, p.35 (PDF36) | Describes Activity Profile listing but lists required agent. Other operations on this resource use activityId. | Use activityId following the resource subject and other operations. |
| §§4.2.1,4.2.4.1, pp.37–38,48 | Invalid-IRI/format rejection duties coexist with permission not to validate IRIs/UUIDs; the latter says data format, not values. | Interpret syntax as mandatory and resolution/existence validation as optional. This is author interpretation, not PDF adjudication. |
| §§4.1.3.1,4.1.3.5, pp.18–19; §4.1.6.1.1, p.23 | General media requirements accept PUT/POST batches; the individual PUT definition specifies one Statement. | Test single PUT and batch POST under the individual definition. PUT-batch interpretation remains unsettled. |
| §4.2, p.37; §4.2.7.1, p.51 | Case is excluded from Statement equivalence while IRI comparison is limited to RFC3987 §§5.3.1/5.3.2. Scope distinction is not explicit. | Retain existing signature mbox-case vectors. Do not extend to unconditional lowercasing of whole IRIs. |
| §4.1.6.1.3, p.24 (PDF25) | related_activities duplicates the Agent-filter description. Pinned current public documentation corrects it to Activity filtering. | Current rule: use the redundant PDF chapter 5 Activity-filter row. Earlier public-correction adoption is historical; no external overlay is required. |

Calling all five earlier issues the same kind of contradiction is inaccurate. The first two directly show inconsistent tables versus prose/subject. Identifier validation, PUT batches and Case concern interpretation of relationships among clauses. The PDF did not resolve them through correction text.

Among clause 4 differences, the expectation-affecting change was the historical public-side related_activities correction; the current basis is the clause 5 PDF row above. Other differences include clause/subclause numbering, abbreviation expansion, reference wording, table-continuation headings, note formatting, typos, punctuation and line wrapping. For example, the issued edition references Annex A.3.4 for Characterstring Parameters examples. Do not claim whole-original byte identity, ISO-edition equality or exhaustive coverage of external normative-reference boundaries.

Store source comparisons, Suite results and receipt/review judgments outside Git under target/.
