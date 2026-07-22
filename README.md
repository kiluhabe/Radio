# 📻 Minimal Desktop FM Radio (Raspberry Pi Pico 版)

Raspberry Pi Pico + RDA5807/M5807PA チューナーで作る、最小構成のデスクトップFMラジオ。物理ノブ（ロータリーエンコーダ＋タクトスイッチ＋音量ポット）だけで操作する専用機を目指している。

## 🎯 Why

- スマホでBluetoothストリーミングは手軽だが、毎回ロック解除するのが面倒だった。ラジオ専用の物理インターフェースが欲しかった
- スマホアプリのクリアな音より、ラジオらしい飽和した音の質感が恋しかった

## ✅ Goals

- 日本のFM帯（76〜108MHz）を受信
- 周波数・お気に入りを電源off後も保持
- ロータリーエンコーダ＋タクトスイッチ＋物理ボリュームだけで操作

## 📍 Status

**Working:**

- チューニング（100kHzステップ、76.00/108.00MHzでラップ）
- OLED表示（周波数 / RSSI信号バー）
- モード切替：短押しで `TUNE ⇔ FAV`
- 電源on/off：長押し（1.5秒）でトグル
- お気に入り：16スロット、ダブルプレスで追加/削除
- EEPROM永続化（最後の変更から2秒後に自動保存）
- 配線は `hardware/diylc/radio-rp.diy` で検証済み

**設計上の変更点：**

- 当初はD級アンプ（八ヶ岳クラブ EEHD-4RWA）経由でスピーカーを駆動する計画だったが、BTL出力を単純に3極ジャックへ配線できない構造的な問題が判明したため撤去。アクティブスピーカー前提の構成に変更し、音量調整はソフトウェアではなくフロントパネルのパッシブな物理ポット（10kΩ Aカーブ2連）で行う

**Open issues:**

- まだブレッドボード/検証段階。ケース・パネル未加工
- ロータリーエンコーダ・タクトスイッチ・音量ポット・ステレオジャックの実配線・実装は進行中

## 📚 Docs

- [docs/parts-rp.md](docs/parts-rp.md) — 部品リスト
- [docs/soldering-rp.md](docs/soldering-rp.md) — はんだ付け手順
- [docs/jst-cables-rp.md](docs/jst-cables-rp.md) — JSTケーブル製作・取り付け手順
- [hardware/diylc/README.md](hardware/diylc/README.md) — 基板レイアウト・座標・配線ネット一覧（正本は `hardware/diylc/radio-rp.diy`）

## 🔨 Build

```bash
arduino-cli compile --fqbn rp2040:rp2040:rpipico firmware/radio-rp
arduino-cli upload -p <ポート> --fqbn rp2040:rp2040:rpipico firmware/radio-rp
```

ボードコア（[arduino-pico](https://github.com/earlephilhower/arduino-pico) by Earle Philhower）の追加が必要：

```bash
arduino-cli config add board_manager.additional_urls https://github.com/earlephilhower/arduino-pico/releases/download/global/package_rp2040_index.json
arduino-cli core install rp2040:rp2040
```

Required libraries (via Arduino Library Manager):

- PU2CLR RDA5807
- U8g2
- RotaryEncoder (by Matthias Hertel)
- EEPROM（rp2040コア組み込み、フラッシュエミュレーション）

## 📄 License

Unspecified (personal hobby log).
