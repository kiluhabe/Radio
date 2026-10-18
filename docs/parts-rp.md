# 🧰 Parts List (RP2040-Zero 版)

`hardware/diylc/radio-rpv2.diy` に対応するパーツと大体の個数。配線・ピン割り当ては `.diy` を正とする。

## 🔩 コア部品

| Part | Qty |
|---|---|
| Waveshare RP2040-Zero | 1 |
| aitendo M5807PA（RDA5807P FM チューナモジュール） | 1 |
| aitendo P20D10B（M5807PA 用 DIP 化基板） | 1 |
| SSD1306 128×64 I²C OLED | 1 |
| ロータリーエンコーダ 秋月 [EC10E1220501](https://akizukidenshi.com/catalog/g/g115228/) | 1 |
| タクトスイッチ 秋月 [8MS8P1B05VS2QES](https://akizukidenshi.com/catalog/g/g104369/) 系 | 2 |
| WS2812B-B（ステータスLED） | 1 |
| 10kΩ Aカーブ 2 連ボリューム（スイッチ付き） | 1 |
| ステレオミニジャック MJ-355W | 1 |
| アクティブスピーカー（3.5mm 入力） | 1 |
| USB Type-C ケーブル | 1 |

## ⚡ 受動部品

| Part | Qty |
|---|---|
| ダイオード 1N4148 | 1 |
| 積層セラミックコンデンサ 100nF | 2 |
| カーボン抵抗 330Ω | 1 |
| アルミ電解コンデンサ 4.7µF（必要な場合のみ） | 0〜2 |

## 🧱 実装基材

| Part | Qty |
|---|---|
| ユニバーサル基板 | 3 |
| ピンソケット 2.54mm 1×40 | 2 |
| ピンヘッダ 2.54mm 1×40 | 1〜2 |
| JST-XH ハウジング | 11 |
| XH コンタクト付コード 秋月 [g131309](https://akizukidenshi.com/catalog/g/g131309/) / [g131312](https://akizukidenshi.com/catalog/g/g131312/) | 3〜4 パック |
| 配線材・ハンダ | 適宜 |

## 🖨️ 筐体（3Dプリント）

STL は `hardware/models/`（`CaseTop` / `CaseBottom` / `Button` / `Disc` / `VolumeNob`）。

| Part | Qty |
|---|---|
| M2 ヒートセットインサート | 15 程度 |
| M2 ネジ | 15 程度 |
| M2 スペーサ | 8 程度 |
