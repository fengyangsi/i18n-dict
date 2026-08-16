# i18n-dict

[![crates.io](https://img.shields.io/crates/v/i18n-dict)](https://crates.io/crates/i18n-dict)
[![docs.rs](https://docs.rs/i18n-dict/badge.svg)](https://docs.rs/i18n-dict)
[![CI](https://github.com/fengyangsi/i18n-dict/actions/workflows/ci.yml/badge.svg)](https://github.com/fengyangsi/i18n-dict/actions/workflows/ci.yml)
[![Coverage](https://coveralls.io/repos/github/fengyangsi/i18n-dict/badge.svg?branch=master)](https://coveralls.io/github/fengyangsi/i18n-dict)
[![License](https://img.shields.io/crates/l/i18n-dict)](LICENSE-MIT)

Compile-time key abstraction for Rust i18n dictionaries: key names are mapped to enum variants at compile time, entries are addressed by key directly — zero lookup, zero runtime overhead.

编译期键抽象的 Rust 词典库:键名 → 变体值的映射在编译期完成,词条容器按键直接寻址,零查找、零运行时开销。

## Features / 特性

- **`DictKey` trait** — key-name abstraction: five associated constants (`COUNT` / `VARIANTS` / `KEYS` / `SORTED_KEYS` / `SORTED_VARIANTS`) plus `find` (name → variant, auto binary search beyond 16 variants);键名抽象:五个关联常量 + `find`(键名 → 变体值,变体数 > 16 自动二分)
- **`#[dictkey]` attribute / `#[derive(DictKey)]`** — two equivalent ways to implement the trait;两种实现形态,接口一致
  - declaration order by default (discriminant = declaration index); `sort` reorders to alphabetical;默认保留声明顺序(判别值 = 声明顺序索引);`sort` 参数重排字母序
  - `deserialize` additionally generates serde deserialization;`deserialize` 参数同时生成 serde 反序列化
- **`subset!` macro** — page subset declarations: compile-time subset variant array `VARIANTS` and per-key position constants (direct render addressing, zero mismatch risk);页面子集声明:编译期生成子集变体数组 `VARIANTS` 与每 key 的位置常量(渲染直接寻址,零错位风险)
- **`Dict` trait** — entry container abstraction: `get` / `get_sub` fetch entries by key; storage layout (static multi-language arrays / sparse `Vec` / streamed loading) and missing-key fallbacks are up to you;词条容器抽象:`get` / `get_sub` 按键取词条,存储结构(静态数组多语言一体 / `Vec` 稀疏 / 流式加载)与缺失回退策略由实现者自定

## Installation / 安装

```toml
[dependencies]
i18n-dict = { version = "0.1", features = ["macros"] }
```

The `macros` feature provides the `#[dictkey]` attribute, the `DictKey` / `DictKeyDeserialize` derives and the `subset!` macro; without it the crate ships only the two traits (`DictKey`, `Dict`) — usable, but the enum implementations must be written by hand.

`macros` feature 提供 `#[dictkey]` 属性宏、`DictKey` / `DictKeyDeserialize` derive 与 `subset!` 宏;不带该 feature 时 crate 仅含两个 trait(`DictKey` / `Dict`),enum 实现需手写。

## Quick Start / 快速开始

```rust
use i18n_dict::{dictkey, subset, Dict};

#[dictkey]
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Key {
    welcome_title,
    welcome_body,
}

// Entry container: storage layout and fallback strategy are up to you
// (multi-language static arrays here).
#[derive(Clone, Copy)]
enum Lang { En, Cn }

struct MyDict {
    lang: Lang,
    en: [&'static str; Key::COUNT],
    cn: [&'static str; Key::COUNT],
}

impl Dict for MyDict {
    type DICTKEY = Key;
    type VALUE = &'static str;
    fn get(&self, key: Key) -> &'static str {
        match self.lang {
            Lang::En => self.en[key as usize],
            Lang::Cn => self.cn[key as usize],
        }
    }
}

// Page subset: the variant array and per-key positions are available at
// compile time.
subset!(Key, HomePage, Key::welcome_title, Key::welcome_body);

let dict = MyDict {
    lang: Lang::En,
    en: ["Welcome", "Hello"],
    cn: ["欢迎", "你好"],
};

// Fetch entries by key (single / batch).
assert_eq!(dict.get(Key::welcome_title), "Welcome");
let strs = dict.get_sub(&HomePage::VARIANTS);
// Position addressing: position within the subset = discriminant.
assert_eq!(strs[HomePage::welcome_title as usize], "Welcome");

// Language file loading: name -> variant via find (rejects unknown names).
let key = Key::find("welcome_body").expect("unknown key");
assert_eq!(dict.get(key), "Hello");
```

## Documentation / 文档

- [docs.rs](https://docs.rs/i18n-dict)

## License / 许可证

Dual-licensed under [MIT](LICENSE-MIT) OR [Apache-2.0](LICENSE-APACHE), at your option.

双许可:[MIT](LICENSE-MIT) 或 [Apache-2.0](LICENSE-APACHE),使用者任选其一。
