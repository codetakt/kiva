# IEEE発行版と公開LRS文書の照合（2026-10-05）

Current-policy update (2026-10-06): this document records the historical
chapter 4 comparison. The current interpretation uses PDF 5.1.6.1 for
`related_activities` and 5.1.6.6.5 for Activity Profile list scope. Those
chapter 5 tables supersede the original overlay/list-exclusion readings.
See [lrs-interpretation-policy.yaml](lrs-interpretation-policy.yaml); no
external overlay or official adjudication is required or claimed.


利用者が提供した `927411-2023.pdf` は **IEEE Std 9274.1.1-2023** の
発行版である。IEEE SA Standards Boardによる承認日は2023年3月30日。
103 PDFページ、1,551,523 bytesで、暗号化されていない。
文書の版表示、目次、第4章の開始・終了とAnnex Aの存在を確認した。
IEEEサイトから今回取得したファイルではなく、利用者から受領したファイルである。
ISO/IEC/IEEE 39274-1-1:2025との同一性は、このPDFからは判定できない。

原本SHA-256は
`fa57dbaa0fbc0800b8c5a693af884032e782110c1f446ef865797fd0e0811526`。
原本、抽出テキスト、比較用データと表示確認画像は
`target/ieee-pdf-reconciliation/20261005/` に保存した。
配布用資産には原本・全文抽出・利用者固有のダウンロード表示を含めていない。
変換器・設定、読取り範囲、公開文書との対応は
[lrs-published-source.yaml](lrs-published-source.yaml) に記録した。

印字ページとPDFページは異なる。以下の `p.` は印字ページを指し、
PDFビューアのページ番号はそれに1を加えた番号になる。

LRSを定める第4章全体（印字pp.16–53、PDF pp.17–54）を抽出テキストで読んだ。
用語の強度、規範参照、共通定義も読んだ。判断を左右するState一覧、Activity
Profile一覧、Statement GETフィルター、同値比較と識別子検証の箇所は元の図版でも
確認した。公開文書の段落と条番号ごとの比較から出た差分候補を本文と照合した。
字句比較は改行、表内の読取り順、字体の差を探す補助であり、同値性の証明には使っていない。

| 箇所 | 発行版で確認した内容 | テストでの扱い |
| --- | --- | --- |
| §4.1.6.2.5、p.29（PDF 30） | State一覧表の`stateId`欄は、説明で使用禁止、Required欄で`since`がない場合は必須とする。§4.1.6.2の本文は省略時に一覧を返すとする。表内の矛盾はPDFにもある。 | 本文に従い、一覧GETでは`stateId`を省略する。 |
| §4.1.6.6.5、p.35（PDF 36） | ActivityのProfile一覧と説明しながら、引数表は必須`agent`を記載する。他の同リソースの操作は`activityId`を使う。 | リソースの対象と他の操作に従い`activityId`を使う。 |
| §§4.2.1、4.2.4.1、pp.37–38,48 | 不正IRIと形式不正の拒否義務、およびIRI/UUIDを検証しない許可が併存する。後者には「data format, not values」とある。 | 構文は検査必須、解決・実在の検証は任意と解釈している。この区別は作成者の解釈で、PDFによる裁定ではない。 |
| §§4.1.3.1,4.1.3.5、pp.18–19と§4.1.6.1.1、p.23 | 一般のメディア型要件はPUT/POSTのバッチ受入れ、PUT個別定義は単一Statementと記載する。 | 個別定義に従い単一PUT、バッチPOSTを検査する。PUTバッチの解釈は未確定。 |
| §4.2、p.37と§4.2.7.1、p.51 | Statement同値比較からCaseを除外する文言と、IRI比較をRFC 3987 §§5.3.1/5.3.2に限定する文言がある。適用範囲の区別は明示されていない。 | 既存の署名用mbox大小文字ベクトルを維持する。IRI全体を無条件で小文字化する規則には拡張しない。 |
| §4.1.6.1.3、p.24（PDF 25） | `related_activities`行はAgentフィルターの説明を重複している。固定した現行公開文書はActivityフィルターの説明に修正している。 | Current rule: use the redundant PDF chapter 5 Activity-filter row. The earlier public-correction adoption is historical; no external overlay is required. |

前の説明で挙げた五件を、すべて同じ種類の「矛盾」と呼ぶのは正確ではない。
最初の二件は表と本文・対象の不整合を直接確認できる。識別子検証、PUTバッチ、
Caseの三件は条文間の適用関係を決める解釈課題である。
PDFはこれらを修正文で解消していなかった。

第4章で照合した差分のうち、テストの期待値に関わる変更は
the historical public-side `related_activities` correction. The current basis is the chapter 5 PDF row described above.その他に条・項の番号付与、略語の展開、
参照先の表現、表の継続見出し、注記の体裁、誤植・句読点・改行の差がある。
たとえばCharacterstring Parametersの例への参照は発行版ではAnnex A.3.4である。
原本全体のbyte同一性、ISO版との一致、外部規範参照の全境界の網羅は主張していない。

原典照合やSuiteの実行結果、受領・レビュー判定は、Git対象外の`target/`に保存する。
