# 📻 Minimal Desktop FM Radio (RP2040-Zero 版)

Waveshare RP2040-Zero + RDA5807/M5807PA チューナーで作る、最小構成のデスクトップFMラジオ。物理ノブ（ロータリーエンコーダ＋タクトスイッチ＋スイッチ付き音量ポット）だけで操作する専用機。ファームウェアは Rust (`no_std`)。

- 日本のFM帯（76〜108MHz）を受信
- 周波数・お気に入りを電源off後も保持
- ロータリーエンコーダ＋タクトスイッチ＋物理ボリュームでの操作

## 📍 Status

- チューニング（1クリック＝0.1MHz、76.00/108.00MHzでラップ）
- OLED表示（周波数のみのミニマル画面。Radio Canada (OFL) のビットマップ数字）
- モード切替：短押しで `TUNE ⇔ FAV`
- 電源on/off：**音量ポット内蔵スイッチ**
- お気に入り：16スロット、ダブルプレスで追加/削除

## 🧱 構成

`radio-core`（ハードウェア非依存のロジック、`no_std`）と `firmware/rp2040`（アダプタ＋合成ルート）に分離したポート＆アダプタ構成。

## 📚 Docs

- [docs/parts-rp.md](docs/parts-rp.md) — 部品リスト
- `hardware/diylc/radio-rpv2.diy` — 配線・ピン割り当ての正本（[DIYLC](https://github.com/bancika/diy-layout-creator) で開く）
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
