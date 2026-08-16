# i18n-dict

Compile-time key abstraction for Rust i18n dictionaries: key names are mapped to enum variants at compile time, entries are addressed by key directly — zero lookup, zero runtime overhead.

编译期键抽象的 Rust 词典库:键名 → 变体值的映射在编译期完成,词条容器按键直接寻址,零查找、零运行时开销。

## 特性

- **`DictKey` trait**:键名抽象。五个关联常量(`COUNT` / `VARIANTS` / `KEYS` / `SORTED_KEYS` / `SORTED_VARIANTS`)+ `find`(键名 → 变体值,变体数 > 16 自动二分)
- **`#[dictkey]` 属性宏 / `#[derive(DictKey)]`**:两种实现形态,接口一致
  - 默认保留声明顺序(判别值 = 声明顺序索引);`sort` 参数重排字母序
  - `deserialize` 参数同时生成 serde 反序列化
- **`subset!` 宏**:页面子集声明。编译期生成子集变体数组 `VARIANTS` 与每 key 的位置常量(渲染直接寻址,零错位风险)
- **`Dict` trait**:词条容器抽象。`get` / `get_sub` 按键取词条,存储结构(静态数组多语言一体 / `Vec` 稀疏 / 流式加载)与缺失回退策略由实现者自定

## 安装

```toml
[dependencies]
i18n-dict = { version = "0.1", features = ["macros"] }
```

## 快速开始

```rust
use i18n_dict::{dictkey, subset, Dict};

#[dictkey]
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Key {
    welcome_title,
    welcome_body,
}

// 词条容器:存储结构与回退策略由实现者自定(此处为多语言一体静态数组)
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

// 页面子集:编译期拿到变体数组与位置
subset!(Key, HomePage, Key::welcome_title, Key::welcome_body);

let dict = MyDict {
    lang: Lang::En,
    en: ["Welcome", "Hello"],
    cn: ["欢迎", "你好"],
};

// 按键取词条(单条 / 批量)
assert_eq!(dict.get(Key::welcome_title), "Welcome");
let strs = dict.get_sub(&HomePage::VARIANTS);
// 位置寻址:子集内位置 = 判别值,渲染直接寻址
assert_eq!(strs[HomePage::welcome_title as usize], "Welcome");

// 语言文件加载:键名 → 变体值(find 校验未知键名)
let key = Key::find("welcome_body").expect("未知键名");
assert_eq!(dict.get(key), "Hello");
```

## 文档

- [docs.rs](https://docs.rs/i18n-dict)

## 许可证

MIT
