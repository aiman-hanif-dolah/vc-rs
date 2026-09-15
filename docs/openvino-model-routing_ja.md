# OpenVINO モデル別構成の実装・検証（2026-09-15）

## 採用したGPU精度指定なしの設定

以下の計測はGPUのFP32指定があった時点の結果。
その後のユーザー依頼により、試聴用にGPUの `load_config` を削除した。
共通の `precision=ACCURACY` は維持するが、GPUの推論精度と実行モードは
個別に上書きしない。位相計算にも個別のFP32指定は加えない。
ContentVec固定／RVC動的／RMVPE OpenVINO CPUという構成は維持する。
これはFP16を強制する変更ではなく、実効精度はEPの選択に委ねる。
以前の位相数値差は観測済みだが、RMVPE修正後の聴感への影響は別途比較する。
実機の試聴用CLIではEPから `Provider option precision is ignored` が出た。
従って共通の `precision=ACCURACY` を送っているだけでは、CPU/GPUの
実効精度を保証しない。CPUの実効精度も未確認として扱う。
ユーザーは試聴用release GUIで速度改善と聴感上の差が小さいことを確認し、
GPUの明示的な精度指定なしの設定を採用した。これは聴感による採用判断であり、
FP16実行の確認や既存の数値比較テスト合格を意味しない。
CPU固定shapeは別途検証中で、通常のCPU経路は動的shapeを維持する。

## 実装した構成

ユーザーが既知の微小音声差を確認したうえで承認した候補を、共有パイプラインへ実装した。

| モデル | GPU選択時の実行先 | shape / 設定 |
|---|---|---|
| ContentVec | OpenVINO GPU | 共有計算から導出した固定入力、FP32 / ACCURACY |
| RMVPE | OpenVINO CPU | 動的入力、共通ACCURACY要求、CPUのスレッド/ストリームは自動 |
| RVC | OpenVINO GPU | 元の動的入力、FP32 / ACCURACY |

LATENCY追加、RVCのGather書換え、修正版OpenVINOの配布・導入、RMVPE GPU化は含まない。
CPU設定は先行RMVPE単体比較の「shared loader既定」と同じで、CPU用LATENCY・固定スレッド数・
固定ストリーム数を追加していない。GPU用load_configと共通 `precision=ACCURACY` は維持した。

製品コードに実験環境変数への依存はない。CLI/GUI/VST3が利用する `RvcPipeline::load` と
共通session loaderで適用し、通常の動的ロード経路を維持した。
本worktreeへの実装であり、元checkoutへの反映・コミット・プッシュは行っていない。

## 変更箇所と不変条件

- `pipeline.rs`: チャンク・サンプルレート・出力余裕・追加文脈から既存の
  `tensor_rt_model_input_samples_16k` で入力長を求め、ContentVecだけにprofileを渡す。
  RVC/RMVPEをまとめて固定する実験用ロード経路は採用していない。
- `sessions.rs`: 現在のOpenVINO候補のフィルター済みデバイス一覧にGPUが含まれ、
  モデルの役割がContentVecの場合だけORTのsymbolic dimension overrideを設定する。
  CPU上のロード時probeで次元名を取得し、元ONNXを書き換えない。
- 静的な寸法も要求値と照合する。名前のない動的軸、同じ記号への異なるサイズ指定、
  非対応rank、静的寸法の不一致は、切詰めや無条件の上書きで回避せず明示エラーにする。
- RMVPEの既存回避策をORT CPUからOpenVINO CPUへ変更。
  明示OpenVINO CPU/NPU、他のbackendには役割の置換を加えない。
- Autoは実際のcatalog候補ごとに判断し、OpenVINOの試行が失敗した場合の
  DirectML→CPU再試行を維持する。後続のbuilderへ固定化設定は引き継がれない。
  無指定OpenVINOの既存デバイス選択方針も維持する。
- 変更はモデルロード時だけ。音声callbackへの確保・I/O・ロック・ログ追加はない。
  稼働中のチャンク長/サンプルレート変更は既存の入力契約で拒否し、設定変更時に再ロードする。

入力長はハードコードしていない。今回の通常設定（出力余裕107 ms、追加文脈100 ms）では
20/30 msチャンクが3840、500 msが11520、2000 msが35520サンプル。
44.1/48 kHz入力・30 msチャンク・追加文脈125 msでは4480サンプルとなった。

## 回帰検証

- Windows ML CLIのreleaseビルド成功。
- Windows ML featureのコア通常テスト **195件合格、6件ignored**。
- 実機のignored test **4件合格**:
  `openvino_pipeline_reloads_shape_for_changed_timing`、`openvino_support_model_diagnostic`、
  `openvino_cpu_tiny_rvc`、`openvino_gpu_tiny_rvc`。
  再ロード後の固定長、誤入力拒否、RVCの動的入力維持、RMVPEのOpenVINO CPU選択を検証。
  無音/160/220 Hzで特徴量相対RMS差は最大約1.01e-5、pitch差は0。
- Auto候補の失敗・再試行、モデル別provider対応、静的寸法不一致・名前なし軸・
  記号衝突の単体回帰テストを追加。既存の精度閾値は緩めていない。
- TensorRT-only featureの `cargo check` 成功（native SDK無効でコンパイル互換性を確認）。
  native TensorRT実行を検証したという意味ではない。
- Clippy完了。既存の未使用項目と `chunks_exact` に関する警告は残るが、変更箇所に新規警告なし。
  対象Rustファイルのformat check、diff check実施。
- Auto・無指定OpenVINO・明示OpenVINO CPUで短い変換がすべて正常終了。
  このPCのAutoはOpenVINOを選択し、GPU候補でContentVec固定/RMVPE CPUを確認。
  明示CPUではContentVec固定化も役割置換も発生しない。
  NPUは実機がないため実行せず、provider対応テストと適用条件の確認に限定した。

最初の3通常テスト失敗はテストexe隣のbootstrap DLL不足だった。
既存DLLを `VC_RS_WINDOWSML_BOOTSTRAP_DLL` で指定し、全通常テストを再実行して合格した。
システムのruntimeやドライバーは変更していない。

## 組み合わせの性能

実機はCore i7-1195G7 / Iris Xe、Windows 11、Intelドライバー32.0.101.7088、
Windows ML OpenVINO EP 1.8.84.0。既存のcatalog runtimeを使った。

同じ60秒人工信号、500 msチャンク。初期3チャンク・排出・起動を除く117チャンク/回。
旧構成B、以前の固定ContentVecのみF、今回の製品構成Pを
`B,F,P` → `F,P,B` → `P,B,F` の3巡、各回別プロセスで実行。
各構成351チャンクからnearest-rankのp95/p99を再計算した。
RMVPE担当のビルド・推論終了を確認して測定し、こちらも測定中のビルド・重い解析を避けた。
温度・電力・外部アプリを完全に統制した試験ではなく、チャンクを独立した351反復とは扱わない。

| 構成 | 平均 ms | p95 ms | p99 ms | 最大 ms | 500 ms超過 / 351 |
|---|---:|---:|---:|---:|---:|
| 旧構成（両モデル動的GPU、RMVPE ORT CPU） | 445.41 | 474.85 | 585.62 | 924.56 | 9 |
| ContentVec固定のみ、RMVPE ORT CPU | 463.93 | 512.33 | 559.17 | 952.29 | 21 |
| 今回の製品構成 | 446.59 | 462.07 | 508.74 | 536.70 | 7 |

平均は旧構成とほぼ同じ（+1.18 ms）。集計p95は約12.78 ms、p99は約76.88 ms短縮した。
期限超過は9→7回で、ゼロを保証するものではない。
これはオフライン処理時間の予算超過であり、実測の音声デバイスアンダーランではない。

| 巡回 | 旧構成 p95 / p99 / 超過 | 固定のみ p95 / p99 / 超過 | 製品構成 p95 / p99 / 超過 |
|---|---|---|---|
| 1 | 525.84 / 594.03 / 8 | 546.86 / 560.14 / 16 | 461.53 / 508.61 / 2 |
| 2 | 457.60 / 483.50 / 1 | 473.06 / 510.69 / 2 | 475.86 / 514.47 / 5 |
| 3 | 459.15 / 486.58 / 0 | 483.86 / 527.89 / 3 | 443.33 / 444.57 / 0 |

旧構成に対するp95/p99改善は2/3巡で、常に改善したわけではない。
モデル段階の平均はContentVec 34.05→27.42 ms、RMVPE 27.07→22.54 ms。
RMVPE p95は39.29→26.35 ms、p99は44.87→29.58 msだった。
RVC平均は381.27→393.83 msと逆に増えており、段階の短縮を足した値を全体の改善幅にはしない。

固定のみの比較には以前の実験ビルドを使った。このビルドにはロード時の段階warmupがあり、
今回の製品コードにはない。定常時間から起動は除外したが、構成間の全差をRMVPE変更だけに帰属させない。
製品構成のロードは約5.7～6.5秒、プリロール約3.7～3.8秒。
旧構成は約5.0～6.0秒＋約4.0～4.1秒で、起動全体の大幅短縮は確認していない。

## 音声とshapeの整合性

旧構成と製品構成を同じ入力・設定で比較。最終PCM16 WAVに対して既存 `audio_compare` を使用。
閾値は最大絶対差1e-4、相対RMS差1e-3、LSD 0.5 dBのまま。

| 入力 / チャンク / 追加文脈 | 最大絶対差 | 相対RMS差 | LSD dB | 既存判定 |
|---|---:|---:|---:|---|
| 2秒調波16 kHz / 20 ms / 100 ms | 6.10e-5 | 3.26e-5 | 0.5059 | LSD不合格 |
| 同 / 30 ms / 100 ms | 9.16e-5 | 3.63e-5 | 0.5318 | LSD不合格 |
| 同 / 500 ms / 100 ms | 3.05e-5 | 4.78e-5 | 0.8493 | LSD不合格 |
| 同 / 2000 ms / 100 ms | 3.05e-5 | 4.49e-5 | 0.6543 | LSD不合格 |
| 3.173秒変動信号44.1 kHz / 30 ms / 125 ms | 2.14e-4 | 9.02e-5 | 0.3381 | 最大差不合格 |
| 同48 kHz / 30 ms / 125 ms | 1.53e-4 | 7.42e-5 | 0.2586 | 最大差不合格 |
| 60秒人工信号16 kHz / 500 ms / 100 ms | 2.14e-4 | 5.41e-5 | 0.7757 | 最大差/LSD不合格 |

短い6条件の12変換は正常終了し、各参照と出力サンプル数が一致した。
端数の最終チャンク、44.1/48 kHzからの変換、追加文脈変更を含む。
60秒出力は3巡とも「固定ContentVecのみ」とPCM16で完全一致した。
44.1/48 kHzでは先行固定のみ試験より相対RMS差が少し増えたが、最大差は同じ範囲だった。

これらは**閾値を満たしたという結果ではない**。ユーザー承認済みの微小差を記録したまま実装した。
[採用前のfloat/PCM16切り分け](openvino-contentvec-validation_ja.md)では丸めがLSDに大きく寄与する一方、
量子化前の最大差超過も確認している。今回の製品比較は最終PCM16であり、
その先行結果を使って新たな合格判定へ置き換えてはいない。
実音声試聴、GUI/VST3ホスト上の実時間音切れ、他モデル・他GPUは未検証。

## 保存物と元checkoutへの反映

`target/openvino-production/` に、変更前ソースのsnapshot、ビルド/テストログ、
実行引数とexe/モデル/EPハッシュ、全チャンク時間、音声、比較JSON、集計JSONを保存した。
再実行は `run-validation.ps1`、集計は `summarize.py`。
比較に使用した旧/製品CLIは `bin/baseline.exe` / `bin/candidate.exe`。

`openvino-model-routing.patch` は製品変更4ファイルと関連する検証文書2ファイルを含み、
既存の計測機能など他の未コミット変更を含めない。元checkoutに対する `git apply --check` は成功した。
適用後はWindows ML版を通常の `just build-cli windowsml` 等で再ビルドし、モデルを再ロードする。
パッチには実行ファイル・ローカルモデル・ログ・機械固有パスを含めない。
