# 📻 Simple FM Radio

## 📍 Status

- チューニング（1クリック＝0.1MHz、76.00/108.00MHzでラップ）
- OLED表示（周波数のみのミニマル画面。Radio Canada (OFL) のビットマップ数字）
- モード切替：短押しで `TUNE ⇔ FAV`（FAV は画面を白黒反転）
- 電源on/off：**音量ポット内蔵スイッチ**
- お気に入り：16スロット、ダブルプレスで追加/削除

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
