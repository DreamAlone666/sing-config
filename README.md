# sing-config

[![LICENSE](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

[sing-box](https://github.com/SagerNet/sing-box) 配置文件预处理器，支持更多的配置语言和 Providers。

## 功能特性

- 🚀 **Providers**: 从外部文件和链接引入 sing-box 出站。
- 💡 **更多语言**: 用 TOML、YAML、JSON 编写配置并编译为 sing-box 支持的 JSON。
- 🔎 **过滤器**: 对 Provider 的出站进行正则表达式过滤。
- ⚠️ **冲突检测**: 出站标签冲突时程序报错并提供信息。
- ✨ **原生体验**: 完全兼容原生 sing-box 配置，同时无缝集成特色功能。

## 使用方法

### 我有一些外部的 sing-box 出站

像这个 `external.json`：

```json
{
  "outbounds": [
    { "type": "direct", "tag": "EXT-DIRECT-01" },
    { "type": "block", "tag": "EXT-BLOCK-01" }
  ],
}
```

使用 provider 引入到我的配置文件 `sing-config.json` 中，并在 `selector` / `urltest` 中使用它：

```jsonc
{
  "providers": {
    // 这里起个 `external` 作为 provider 的标签
    "external": {
      "path": "external.json"
    }
  },

  "outbounds": [
    {
      "type": "urltest",
      "tag": "Auto",
      // 在 `outbound_providers` 这个字段里使用 provider
      "outbound_providers": ["external"]
    },
    {
      "type": "selector",
      "tag": "Select",
      // 原本的 `outbounds` 也可以正常用来引用配置里本就有的出站
      "outbounds": ["LOCAL-DIRECT"],
      "outbound_providers": ["external"]
    },
    // 👇 这就是一个配置里本就有的出站
    {
      "type": "direct",
      "tag": "LOCAL-DIRECT"
    }
  ]

  // 其他配置字段正常写，编译时按原样透传 (passthrough)
  "log": {
    "disabled": false,
    "level": "info"
  }
}
```

然后输入到 `sing-config` 编译成 `sing-box.json`：

```sh
sing-config sing-config.json --output sing-box.json
```

编译好后的 `sing-box.json` 长这样，`provider.external` 的出站会合并到 `outbounds` 并展开到 `selector` / `urltest` 中：

```json
{
  "outbounds": [
    { "type": "urltest", "tag": "Auto", "outbounds": ["EXT-DIRECT-01", "EXT-BLOCK-01"] },
    { "type": "selector", "tag": "Select", "outbounds": ["LOCAL-DIRECT", "EXT-DIRECT-01", "EXT-BLOCK-01"] },
    { "type": "direct", "tag": "LOCAL-DIRECT" },
    { "type": "direct", "tag": "EXT-DIRECT-01" },
    { "type": "block", "tag": "EXT-BLOCK-01" }
  ],
  "log": {
    "disabled": false,
    "level": "info"
  }
}
```

### 我的出站是通过链接来的（比如订阅）

直接把 `path` 方式改成 `url` 方式就行：

```jsonc
{
  "providers": {
    "external": {
      // path: "external.json"
      "url": "https://example.com/external.json"
    }
  }
}
```

### 我只想要某些出站 / 我不想要某些出站

`actions` 里可以写一串正则表达式过滤器，编译时会按顺序执行：

```jsonc
{
  "providers": {
    "external": {
      // path: "external.json"
      "url": "https://example.com/external.json",
      "actions": [
        // 1. 只要 `tag` 包含 `direct` （不区分大小写）出站
        { "type": "include", "field": "tag", "regex": "(?i:direct)" },
        // 2. 不要 `tag` 包含 `block` 的出站
        { "type": "exclude", "field": "tag", "regex": "block" },
        // 3. 只要 `type` 是 `direct` 或 `block` 的出站
        { "type": "include", "field": "type", "regex": "^(direct|block)$" },
        // 4. 不要 `type` 是 `block` 的出站
        { "type": "exclude", "field": "type", "regex": "^block$" }
      ]
    }
  }
}
```

### 我还想对源 provider 用另一套 `actions`

用 `ref` 方式引用源 provider，就可以派生出很多子 provider，各自套 `actions` 就行。

```jsonc
{
  "providers": {
    "external": {
      // path: "external.json"
      "url": "https://example.com/external.json"
    },
    // 这是一个新的叫 `allow` 的 provider
    "allow": {
      // 它引用了 `external`，相当于 `external` 的输出作为它的输入
      "ref": "external",
      "actions": [
        { "type": "include", "field": "tag", "regex": "(?i:direct)" }
      ]
    },
    // 同理这也是一个新的，引用了 `external` 的，叫 `deny` 的 provider
    "deny": {
      "ref": "external",
      "actions": [
        { "type": "include", "field": "tag", "regex": "(?i:block)" }
      ]
    }
  }
}
```

### 我受够了 JSON

没问题，TOML / YAML 照样写，最后编译成 JSON。

```toml
[providers.external]
# path = "external.json"
url = "https://example.com/external.json"

# 这是一个新的叫 `allow` 的 provider
[providers.allow]
# 它引用了 `external`，相当于 `external` 的输出作为它的输入
ref = "external"
actions = [
    { type = "include", field = "tag", regex = '(?i:direct)' },
]

# 同理这也是一个新的，引用了 `external` 的，叫 `deny` 的 provider
[providers.deny]
ref = "external"
actions = [
    { type = "include", field = "tag", regex = '(?i:block)' },
]
```

### 其他格式的 provider 能用吗（比如 Clash）

不能，provider 的内容必须是 **sing-box config 格式**（包含 `outbounds`），不支持嵌套 `sing-config`、TOML、YAML，也不支持 Clash 等其他常见订阅格式。

## 使用场景

### ✅ 适用

- 🤓 我对 sing-box 配置有一定了解并且能撰写
- 🤔 我想引入外部出站，同时在其他部分保持自己的配置
- 😡 我受够了 JSON，希望使用 TOML / YAML 来编写配置

### ❌ 不适用

- 🔄 引入/转换其他格式的配置到 sing-box（这不是订阅转换）
- 🪄 一键编写好用的配置（这不是配置模板）
- 🤖 配置会自动更新并让 sing-box 自动重载（我只负责编译配置）

### 🔒 已知限制

- `ref` 方式只能引用非 `ref` 的 provider，即不能嵌套引用
- 过滤器的出站字段目前只支持 `tag` 和 `type`
- 任何出站标签的冲突都会让编译主动中止并报错

## 安装

### 方法一：从 GitHub Releases 下载 (推荐)

你可以从项目的 [GitHub Releases](https://github.com/DreamAlone666/sing-config/releases) 页面下载预编译的二进制文件，解压后直接使用。这是最简单的安装方式。

### 方法二：从源码构建

如果你希望自行构建，你先需要安装 [Rust](https://www.rust-lang.org/tools/install) 工具链。

```bash
# 1. 克隆仓库
git clone https://github.com/DreamAlone666/sing-config.git

# 2. 进入项目目录
cd sing-config

# 3. 构建
cargo build --release

# 生成的可执行文件位于 ./target/release/sing-config
```

## 许可证

本项目使用 [MIT](LICENSE) 许可证。
