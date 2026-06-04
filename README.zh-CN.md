# Noteva

<p align="center">
  <a href="README.md">English</a> | <a href="README.zh-CN.md">绠€浣撲腑鏂?/a>
</p>

<p align="center">
  <a href="https://github.com/noteva26/Noteva/releases"><img alt="鐗堟湰" src="https://img.shields.io/badge/version-0.3.3-111827"></a>
  <a href="LICENSE"><img alt="璁稿彲璇? src="https://img.shields.io/badge/license-GPL--3.0--or--later-blue"></a>
  <img alt="Rust" src="https://img.shields.io/badge/Rust-1.75%2B-b7410e">
  <img alt="SQLite" src="https://img.shields.io/badge/SQLite-default-0f766e">
</p>

Noteva 鏄竴涓娇鐢?Rust 鏋勫缓鐨勮交閲忕幇浠ｅ崥瀹㈢郴缁熴€傚畠榛樿浣跨敤 SQLite锛屽彲浠ュ崟鏂囦欢閮ㄧ讲锛屽悓鏃朵繚鐣欑畝娲佺殑鍐欎綔銆佷富棰樹笌鎻掍欢浣撻獙锛岄€傚悎涓汉绔欑偣锛屼篃鑳芥寜闇€鎵╁睍銆?
<p align="center">
  <img src="docs/images/noteva-hero.png" alt="Noteva 姒傝" width="900">
</p>

## 涓轰粈涔堥€夋嫨 Noteva

- 杞婚噺閮ㄧ讲锛氬崟浜岃繘鍒舵枃浠讹紝榛樿鏈湴 SQLite锛屽彲閫?MySQL 涓?Redis銆?- 娓呯埥鍚庡彴锛氭枃绔犮€侀〉闈€佸垎绫绘爣绛俱€佽瘎璁恒€佹枃浠躲€佹彃浠躲€佷富棰樸€佸畨鍏ㄦ棩蹇椼€佸浠藉拰绔欑偣璁剧疆銆?- Markdown 鍐欎綔锛氶瑙堛€佷唬鐮侀珮浜€佸獟浣撲笂浼犮€佸浘鐗囩綉鏍间笌 Shortcode 鎵╁睍銆?- 娌欑鎻掍欢锛歐ASM 鍚庣閽╁瓙銆佸墠绔?JS/CSS 娉ㄥ叆銆佹潈闄愩€佽缃€佸瓨鍌ㄤ笌澶氳瑷€鏂囦欢銆?- 鍓嶇妗嗘灦鏃犲叧涓婚锛氶€氳繃鑷姩娉ㄥ叆鐨?`window.Noteva` SDK锛屽彲鐢?React銆乂ue銆佸師鐢?JavaScript 鎴栦换鎰忓墠绔爤寮€鍙戜富棰樸€?- 鍐呯疆鍥介檯鍖栵細甯哥敤鍚庡彴涓庨粯璁や富棰樿瑷€鐩存帴闅忕▼搴忔彁渚涖€?- SEO 鍩虹鑳藉姏锛氬浐瀹氶摼鎺ャ€丼itemap銆丷SS Feed銆乺obots.txt 涓庣珯鐐瑰厓淇℃伅銆?
## 鎴浘

鎴浘浣跨敤婕旂ず鍐呭锛屼富瑕佺敤浜庡睍绀烘暣浣撶晫闈㈠拰浣跨敤娴佺▼銆?
| 鍓嶅彴棣栭〉 | 鏂囩珷闃呰 |
| --- | --- |
| ![鍓嶅彴棣栭〉](docs/images/frontend-home.png) | ![鏂囩珷闃呰](docs/images/post-reading.png) |

| 鏂囩珷绠＄悊 | Markdown 缂栬緫鍣?|
| --- | --- |
| ![鏂囩珷绠＄悊](docs/images/admin-articles.png) | ![Markdown 缂栬緫鍣╙(docs/images/admin-editor.png) |

| 鎻掍欢绠＄悊 | 涓婚绠＄悊 |
| --- | --- |
| ![鎻掍欢绠＄悊](docs/images/admin-plugins.png) | ![涓婚绠＄悊](docs/images/admin-themes.png) |

<p align="center">
  <img src="docs/images/mobile-post.png" alt="绉诲姩绔槄璇婚〉" width="360">
</p>

## 蹇€熷紑濮?
Linux 鎴?macOS 鍙互浣跨敤瀹夎鑴氭湰銆傝剼鏈細妫€娴嬪钩鍙帮紝涓嬭浇鏈€鏂板彂甯冨寘锛屽垱寤哄伐浣滅洰褰曪紝骞跺彲娉ㄥ唽绯荤粺鏈嶅姟銆?
```bash
curl -fsSL https://raw.githubusercontent.com/noteva26/Noteva/main/install.sh | bash
```

棣栨鍚姩鍚庤闂細

```text
http://localhost:8080/manage/setup
```

鍦ㄨ繖閲屽垱寤虹鐞嗗憳璐﹀彿锛岀劧鍚庤繘鍏?`/manage` 绠＄悊鍚庡彴銆?
## Docker

```bash
docker run -d \
  -p 8080:8080 \
  -v ./data:/app/data \
  -v ./uploads:/app/uploads \
  --name noteva \
  ghcr.io/noteva26/noteva:latest
```

Docker Compose锛?
```yaml
services:
  noteva:
    image: ghcr.io/noteva26/noteva:latest
    ports:
      - "8080:8080"
    volumes:
      - ./data:/app/data
      - ./uploads:/app/uploads
    restart: unless-stopped
```

## 浠庢簮鐮佹瀯寤?
鐜瑕佹眰锛?
- Rust 1.75+
- Node.js 20+
- pnpm

```bash
git clone https://github.com/noteva26/Noteva.git
cd Noteva
pnpm run install:all
pnpm run build:frontend
cargo run --bin noteva
```

寮€鍙戝懡浠わ細

```bash
pnpm run dev:web      # 绠＄悊鍚庡彴
pnpm run dev:theme    # 榛樿涓婚
cargo run --bin noteva
```

鍙戝竷鏋勫缓锛?
```bash
pnpm run build:frontend
cargo build --release
```

## 閰嶇疆

Noteva 浼氫粠宸ヤ綔鐩綍璇诲彇 `config.yml`銆備竴涓渶灏忛厤缃ず渚嬪涓嬶細

```yaml
server:
  host: "0.0.0.0"
  port: 8080
  cors_origin: "*"

database:
  driver: "sqlite"
  url: "data/noteva.db"
  # driver: "mysql"
  # url: "mysql://username:password@localhost:3306/noteva"

cache:
  driver: "memory"
  # driver: "redis"
  # redis_url: "redis://127.0.0.1:6379"

upload:
  path: "uploads"
  max_file_size: 10485760
  max_plugin_file_size: 52428800

theme:
  path: "themes"
  active: "default"
```

瀹屾暣绀轰緥瑙?[config.example.yml](config.example.yml)銆?
## 鎻掍欢

鎻掍欢浣嶄簬 `plugins/<plugin-id>/`锛屽苟閫氳繃 `plugin.json` 鎻忚堪銆傛彃浠跺彲浠ュ寘鍚祻瑙堝櫒璧勬簮銆乄ASM 鍚庣妯″潡銆佽缃〃鍗曘€佺紪杈戝櫒鎸夐挳鍜屽璇█鏂囦欢銆?
```text
plugins/my-plugin/
|-- plugin.json
|-- frontend.js
|-- frontend.css
|-- backend.wasm
|-- settings.json
|-- editor.json
`-- locales/
```

鍚庣鎻掍欢閫氳繃 `wasmtime` 鍦?WASM 娌欑涓繍琛屻€傛潈闄愩€侀挬瀛愬０鏄庛€佸瓨鍌ㄥ拰璁剧疆閮芥槸鏄惧紡鐨勶紝渚夸簬鎻掍欢涓庢牳蹇冨簲鐢ㄤ繚鎸侀殧绂汇€?
瀹屾暣璇存槑瑙侊細[鎻掍欢寮€鍙慮(docs/plugin-development.md)銆?
## 涓婚

涓婚浣嶄簬 `themes/<theme-name>/`銆備竴涓富棰樺彧闇€瑕佹竻鍗曟枃浠跺拰鏋勫缓鍚庣殑鍓嶇鍏ュ彛锛屽洜姝ゅ彲浠ヤ娇鐢ㄤ换鎰忓墠绔鏋跺疄鐜般€?
```text
themes/my-theme/
|-- theme.json
|-- settings.json
|-- dist/index.html
`-- preview.png
```

杩愯鏃朵細鑷姩娉ㄥ叆 `window.Noteva` SDK銆備富棰樺簲閫氳繃璇?SDK 鑾峰彇绔欑偣鏁版嵁銆佹枃绔犮€侀〉闈€佽瘎璁恒€佸鑸€佽缃拰鎻掍欢鎻掓Ы銆?
瀹屾暣璇存槑瑙侊細[涓婚寮€鍙慮(docs/theme-development.md)銆?
## 鏂囨。

| 鏂囨。 | 璇存槑 |
| --- | --- |
| [鎻掍欢寮€鍙慮(docs/plugin-development.md) | 鎻掍欢鍖呯粨鏋勩€侀挬瀛愩€佹潈闄愩€乄ASM 妗ユ帴銆佽缃拰鍓嶇闆嗘垚銆?|
| [涓婚寮€鍙慮(docs/theme-development.md) | 涓婚鍖呯粨鏋勩€丼DK 浣跨敤銆佽缃€佹繁鑹叉ā寮忋€佹彃浠舵彃妲藉拰鍏煎瑙勫垯銆?|
| [English Changelog](CHANGELOG.en.md) | 鑻辨枃鏇存柊鏃ュ織銆?|
| [涓枃鏇存柊鏃ュ織](CHANGELOG.md) | 涓枃鏇存柊鏃ュ織銆?|
| [璁稿彲璇乚(LICENSE) | 璁稿彲璇佹鏂囦笌闄勫姞鏉℃銆?|

## 浜у搧鏂瑰悜

Noteva 浼氫繚鎸佸厠鍒讹細瀹夐潤鐨勫啓浣滄祦绋嬨€佺揣鍑戠殑绠＄悊鐣岄潰銆佸畨鍏ㄧ殑鎵╁睍鐐瑰拰绠€鍗曢儴缃层€備换浣曟槑鏄惧鍔犲鏉傚害鐨勫姛鑳斤紝閮藉簲璇ヨ兘璇佹槑鑷繁鍊煎緱鐣欎笅銆?
## 璧炲姪

濡傛灉 Noteva 瀵逛綘鏈夊府鍔╋紝娆㈣繋璧炲姪鏀寔椤圭洰缁х画寮€鍙戯細

- [Bronze ($1)](https://www.creem.io/payment/prod_NLloGph4FdG0QH5BN2DXr)
- [Silver ($5)](https://www.creem.io/payment/prod_1FqirOkv4JY21wExvWN3PW)
- [Gold ($10)](https://www.creem.io/payment/prod_2wV2YqQHJHsqrpWAipx40s)

## 璁稿彲璇?
Noteva 浣跨敤 [GPL-3.0-or-later](LICENSE)锛屽苟甯︽湁鎻掍欢鍜屼富棰樹緥澶栨潯娆俱€?
鏍稿績绋嬪簭淇敼浠嶉伒寰?GPL銆備粎閫氳繃鍏紑 SDK/API 涓?Noteva 浜や簰鐨勬彃浠跺拰涓婚锛屽彲浠ヤ娇鐢ㄨ嚜宸辩殑璁稿彲璇併€傝鎯呰 [LICENSE](LICENSE) 鍜?[COPYING](COPYING)銆?
