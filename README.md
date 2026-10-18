# 📻 Minimal Desktop FM Radio (RP2040-Zero 版)

Waveshare RP2040-Zero + RDA5807/M5807PA チューナーで作る、最小構成のデスクトップFMラジオ。物理ノブ（ロータリーエンコーダ＋タクトスイッチ＋スイッチ付き音量ポット）だけで操作する専用機。ファームウェアは Rust (`no_std`)。

## 🎯 Why

- スマホでBluetoothストリーミングは手軽だが、毎回ロック解除するのが面倒だった。ラジオ専用の物理インターフェースが欲しかった
- スマホアプリのクリアな音より、ラジオらしい飽和した音の質感が恋しかった

## ✅ Goals

- 日本のFM帯（76〜108MHz）を受信
- 周波数・お気に入りを電源off後も保持
- ロータリーエンコーダ＋タクトスイッチ＋物理ボリュームだけで操作

## 📍 Status — v1

**動作確認済み:**

- チューニング（1クリック＝0.1MHz、76.00/108.00MHzでラップ）
- OLED表示（周波数のみのミニマル画面。Radio Canada (OFL) のビットマップ数字）
- モード切替：短押しで `TUNE ⇔ FAV`
- 電源on/off：**音量ポット内蔵スイッチ**（長押しではない）
- お気に入り：16スロット、ダブルプレスで追加/削除
- フラッシュ永続化（最後の変更から2秒後に自動保存）
- 配線は `hardware/diylc/radio-rpv2.diy` で検証済み
- ケース（`hardware/models/*.stl`）

**既知の問題:**

- 選局後、時間経過でブチブチノイズが増えることがある。原因未特定（PLLドリフト説は周期再チューニングを試して否定済み。アンテナ結合など物理要因の可能性）
- ステータスLED（WS2812B / GP9）は未接続のため動作未確認
- I²Cにプルアップ抵抗を実装していない。RP2040内蔵の弱プルアップのみで動作しており、ノイズマージンは薄い

**設計上の変更履歴:**

- D級アンプ経由のスピーカー駆動は、BTL出力を3極ジャックへ配線できない構造上の問題で撤去。アクティブスピーカー前提とし、音量はフロントパネルのパッシブな物理ポット（10kΩ Aカーブ2連）で調整する
- 天面ディスク＋磁気エンコーダ（MA735, SPI）は実機で動作が安定せず撤去。機械式ロータリーエンコーダに戻した
- マイコンは Raspberry Pi Pico から RP2040-Zero へ変更。USBコネクタをケース面に出すため、Zeroは専用の小基板に分離している

## 🔌 ピン割り当て

| 信号 | GPIO |
|---|---|
| I²C SDA / SCL（OLED・チューナー共用） | GP26 / GP27 |
| ロータリーエンコーダ A / B | GP12 / GP13 |
| アクションボタン | GP11 |
| 電源スイッチ（音量ポット内蔵） | GP10 |
| ステータスLED（WS2812B） | GP9 |

## 🧱 構成

`radio-core`（ハードウェア非依存のロジック、`no_std`）と `firmware/rp2040`（アダプタ＋合成ルート）に分離したポート＆アダプタ構成。基板は3枚構成で、Qi/JSTコネクタで接続する。

## 📚 Docs

- [docs/parts-rp.md](docs/parts-rp.md) — 部品リスト・ピン割り当て
- `hardware/diylc/radio-rpv2.diy` — 配線の正本（[DIYLC](https://github.com/bancika/diy-layout-creator) で開く）
- `hardware/models/*.stl` — ケース・ノブ・ディスク
- [ROADMAP.md](ROADMAP.md) — 今後の検討メモ

## 🔨 Build & Flash

```bash
cd firmware/rp2040
cargo build --release
```

書き込みは UF2（RP2040-Zero には SWD パッドが無いため、BOOTSEL のみ）：

```bash
# BOOTSELボタンを押しながらUSB接続してから
elf2uf2-rs -d target/thumbv6m-none-eabi/release/firmware-rp2040
```

ロジックのテストはリポジトリルートから（`firmware/rp2040` 配下には組み込みターゲット設定があるため）：

```bash
cargo test -p radio-core
```

## 📄 License

Unspecified (personal hobby log).
