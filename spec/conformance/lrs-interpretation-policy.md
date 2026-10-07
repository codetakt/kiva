# LRSテストの解釈方針と判定の確実性

採用する解釈を固定し、その解釈から期待値、実行、結果まで追跡可能にする。
原文が不整合な場合でも、この条件で再現可能なテストSuiteを作れる。
ただし、プロジェクトの解釈採用とIEEEによる公式な解釈の確定は異なる。

現在のプロファイルは **`xapi-lrs-ieee-2023-interpretation-3`**。
機械可読の採用規則、理由、採用しない読み方、検査対象は
[lrs-interpretation-policy.yaml](lrs-interpretation-policy.yaml) に記録する。
原典の事実と座標は既存の[lrs-published-source.md](lrs-published-source.md)に保持する。

| 論点 | 採用する解釈 | 判定上の扱い |
| --- | --- | --- |
| State一覧 | `stateId`の有無で単一／一覧を切り替える。一覧では省略し、`since`は絞込み条件とする。 | 本文の明示的な振分けと表の説明欄を保存する読み方。該当GET/HEADの期待値への依存を表示する。 |
| Activity Profile一覧 | `activityId`でActivityの範囲を選び、一覧では`profileId`を省略する。 | 第5章のactivityId必須・since任意表で第4章のcellを読む。条件除外を撤回。該当GET/HEADへの依存を表示する。 |
| IRI/UUID | 所定の形式は検査する。解決可能性、実在や実世界の意味の検証は要求しない。 | `data format, not values`と形式拒否義務をともに保つ解釈。不正形式の期待値と対象の型エラーは区別する。UUID variantの推奨を必須化しない。 |
| PUTバッチ | 単一StatementのPUT、単一／バッチPOSTの必須動作を検査する。PUT配列を拒否する義務は断定しない。 | 単一PUTの正常ケースまで曖昧扱いにしない。PUT配列の期待値がある場合に依存を表示する。該当ケースがなければ未検査である。 |
| 大小文字 | JSONのキーと列挙値のcaseは厳密。IRIは許可されたRFC比較方式に従う。 | 署名のmbox Case衝突status leafを診断へ移し、実応答statusを記録する。他のheader・暗号assertionは必須。IRIのパスやメールアドレス全体の無条件小文字化は導かない。 |
| `related_activities` | PDF第5章の重複表を根拠に、Object・contextActivities・SubStatementのActivityを絞り込む。 | `related_agents`とは別のフィルターとして検査する。第4章の誤記と第5章の明示文を並べ、外部overlayを要求しない。 |

この方針は、観測されたKivaの動作に合わせて期待値を決めない。
期待値を訂正するときは原文の理由と差分を記録し、旧候補と実行結果を保存する。
違反を無視するskipや、失敗を成功へ変更する処理は追加しない。

判定の確実性は、次の点を検証して担保する。

1. **根拠と期待値**：原典の役割、shall/should/may、条件・例外を保存し、
   根拠と具体的な応答検査を結ぶ。表の型違反、必須条件、不正形式を混同しない。
2. **結果の解釈**：生の成功／失敗／未実行を保持し、既知の解釈への依存を併記する。
   ケース内の準備・後続観測にも依存があれば表示する。ケース単位の表示から、
   その失敗の原因が必ず解釈問題だったとは推定しない。
3. **検査器の信頼性**：正例・負例、状態の不変性、境界値に加え、応答や判定を改変した
   対照で誤って通過しないことを検査する。期待値を削除して通る仕組みにしない。
4. **実行の再現性**：原典・プロファイル・ケース・実行器・対象の構成を固定する。
   過去の実行はreceiptでその入力と結果を照合してから利用し、新規実行とは区別する。

`scripts/ci/report_lrs_interpretations.py` は、六つの既知の論点と、クロスレビュー後に追加した判断を具体的な
ケース／期待値の座標・digestに対応付ける。既存の原典監査は、解釈方針、
対応付けの処理と結果もassertion manifestに束縛する。
これにより、解釈を変更しても以前の結果を同じ意味の証拠として黙って再利用することを防ぐ。
ただし、原典の意味判断そのものを機械的に証明する検査ではない。

```sh
python scripts/ci/report_lrs_interpretations.py --json
python scripts/ci/report_lrs_interpretations.py \
  --run-report target/source-projection-work/memory.json \
  --evidence-receipt target/source-projection-work/final-receipt.json \
  --require-complete-report --json
```

実行レポートの利用には、レポート、ケースファイルと実行器の契約を束縛する
`input_sha256` / `artifacts_sha256`付きreceiptが必要である。
レポートの自己申告だけから現行ケースの実行とみなさない。
対象の認証・権限・時計・可視化遅延等は、元の実行fixtureに従って解釈する。
このコマンドの終了コード0は対応付けと証拠照合の成功を表し、LRSの適合合格を表さない。

結果の`no_registered_dependency`は「登録した判断への依存を識別していない」という意味である。
ここに分類された有限ケースだけで、条文全体の解釈や完全性が証明されたとはしない。
`interpretation_dependent`も、生の失敗を取り消す分類ではない。
一覧表の解釈は一覧の成功応答を期待する箇所に結ぶ。
引数名のcase誤りなど、両方の読み方で拒否義務がある負例は通常判定の範囲に残す。

寄贈時の表明は「この版のLRS仕様に対し、この解釈プロファイルと対象構成で、
列挙した要求を検査できる」とする。そのためには、個々の義務から実際のassertionまでの
意味照合、取り込んだ規範参照の境界、適用する時計・遅延・ポリシー等の手順を閉じる。
ケース数、hash、HTTP全件成功だけを完全投射の根拠にしない。
IEEEの公式回答等が得られた場合は、該当解釈と期待値の差分をレビューして新しい版を作る。
その回答の入手を、今できる検査の実装を止める条件にはしない。

機械可読policyの`additional_judgments`は、次の追加の読み方を定義する。

| 判断 | 原文と処置 |
| --- | --- |
| State複数DELETE | registration指定時はその登録だけ、省略時はActivity/Agentに属する全登録。別Agent・Activityの残存をbytesで観測する。 |
| Attachmentの順序 | 論理的同値性の「Any attachments」例外と、exact/default応答のOrdered arrayを区別。異なる2件のmetadataは順序付き一致、MIMEの両データpartは完全性を検査し、part順序は指定しない。 |
| relevantTypes空配列 | 本文の0件は省略時、表の1件以上はproperty存在時。[]を400必須へ移行し、型・null・キーcaseの独立義務を残す。 |
| その他の空配列 | 最小件数がない配列は受理。attachments、Group member、Context各配列、Context Activities、interaction各listをそれぞれ表に照合。必須propertyやIFIの欠落とは区別する。 |
| canonical情報 | 提出metadataが維持されるとは仮定しない。直接ケースではIDと返却mapの言語数を検査し、言語選択の内容は宣言された正規情報のinventoryと比較する手順へ結ぶ。 |
| interaction構造 | 共通のnull・重複キー・キーcaseは禁止。48件を必須400へ戻し、任意のinteraction固有検証から分ける。 |

これらの判断も原文anchorと実caseへの依存表示を持ち、再レビューの対象である。
旧プロファイルの空配列4件の両許容は撤回した。省略・非空の受理と空配列の拒否を別々に対照実行する。

fixture不足は`not-run`であり、実対象の成功にも失敗にも数えない。
条件不存在の証拠がないため`not-applicable`にはしない。

`unknown-local-offset-boundary`は、`-00:00`入力に200または400を許容する。
ISO 8601 / RFC 3339の境界はRFC 9557 §§2.1–2.3とあわせて解釈する。
受理した場合のみ同じ時点をUTCで読み戻す。公式裁定とは扱わない。
解釈の確認と実行結果は、Git対象外の`target/`に保存する。
