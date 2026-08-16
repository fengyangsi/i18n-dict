//! Runtime crate for i18n dictionaries.
//!
//! The core is the [`DictKey`] trait — key-name abstraction for a dict
//! enum. Two ways to implement it (requires the `macros` feature):
//!
//! ```toml
//! i18n-dict = { features = ["macros"] }
//! ```
//!
//! - **Form A (recommended)** `#[dictkey]`: attribute macro. Defaults to
//!   the same behaviour as form B: keeps declaration order, discriminant =
//!   declaration index; with `sort` variants are reordered alphabetically
//!   (discriminant = alphabetical position, depending only on the key-name
//!   set, not on declaration order); with `deserialize` a `Deserialize`
//!   impl is generated as well (requires a direct serde dependency)
//! - **Form B** `#[derive(DictKey)]`: implements the trait keeping
//!   declaration order, discriminant = declaration index; pair it with
//!   `#[derive(DictKeyDeserialize)]` for deserialization (goes through
//!   `find`, inheriting the lookup strategy automatically)
//!
//! Both forms expose the same trait interface: five constants + `find`
//! (name → variant). `KEYS` / `VARIANTS` follow declaration order
//! (alphabetical in sort mode) and are **not guaranteed sorted**; whenever
//! ordered access is needed use `SORTED_KEYS` / `SORTED_VARIANTS` instead
//! (always alphabetical; aliases of `KEYS` / `VARIANTS` in sort mode).
//! With more than 16 variants `find` switches to binary search, otherwise
//! linear — the macro picks the strategy by variant count. The trait
//! requires `Copy` (`find` returns variants by value), so derive
//! `Clone, Copy` on the enum.
//!
//! `subset!(Dict, Name, Dict::a, Dict::b)` declares a page subset (only
//! for enums implementing `DictKey`; a trait-bound assertion makes others
//! fail to compile), generating `Name::VARIANTS` (subset variant array,
//! fetched by `dict.get_sub`) and a per-key position constant (enum
//! discriminant = position within the subset). Global indices need no
//! generation: `Dict::key as usize` is the discriminant. Keys may also be
//! written as bare variants — `subset!(Dict, Name, a, b)` (with
//! `use Dict::*`).
//!
//! [`Dict`] is the entry-container abstraction: you declare the key type
//! (`type DICTKEY: DictKey`) and the value type (`type VALUE`), and provide
//! `get(&self, key) -> VALUE` plus `get_sub(&self, keys) -> Vec<VALUE>`
//! (`key` / `keys` are `DICTKEY` variants; `keys` usually comes from
//! `subset::VARIANTS`). Storage layout (static arrays / `Vec` / language
//! switching), missing-key fallbacks, loading and deserialization are all
//! up to the implementor.
//!
//! ---
//!
//! 中文说明:词典运行时 crate,核心是 [`DictKey`] trait(dict enum 的键名抽象)。
//! 实现方式二选一(需 `macros` feature):
//!
//! - **形态 A(推荐)** `#[dictkey]`:属性宏,默认与形态 B 一致——保留声明
//!   顺序,判别值 = 声明顺序索引;参数 `sort` 时重排变体为字母序(判别值 =
//!   字母序位置,只依赖键名集合,与声明顺序无关);参数 `deserialize` 同时
//!   生成 `Deserialize`(需直接依赖 serde)
//! - **形态 B** `#[derive(DictKey)]`:保留声明顺序实现 trait,判别值 =
//!   声明顺序索引;反序列化搭配 `#[derive(DictKeyDeserialize)]`(走 `find`,
//!   自动继承查找策略)
//!
//! 两种形态 trait 接口一致:五常量 + `find`(键名 → 变体值)。`KEYS` /
//! `VARIANTS` 的顺序 = 声明顺序(sort 模式下为字母序),**不保证排序**;
//! 需要有序访问时一律用 `SORTED_KEYS` / `SORTED_VARIANTS`(恒为字母序,
//! sort 模式下是 `KEYS` / `VARIANTS` 的别名)。变体数量 > 16 时 `find`
//! 自动切二分,小则线性——策略由宏按数量决定。trait 要求类型 `Copy`
//! (`find` 按值返回变体),使用宏的 enum 请同时 `#[derive(Clone, Copy)]`。
//!
//! `subset!(Dict, Name, Dict::a, Dict::b)` 声明页面子集(仅限已实现
//! `DictKey` 的 enum,未实现会编译报错),生成 `Name::VARIANTS`(子集变体
//! 数组,`dict.get_sub` 按此拉取)与每 key 的位置常量(枚举判别值 =
//! 子集内位置)。全局下标无需生成:`Dict::key as usize` 即判别值。
//! key 参数也可写裸变体名 `subset!(Dict, Name, a, b)`(需 `use Dict::*`)。
//!
//! [`Dict`] 是词条容器抽象:实现者声明键类型(`type DICTKEY: DictKey`)
//! 与值类型(`type VALUE`),提供 `get(&self, key) -> VALUE`
//! 与 `get_sub(&self, keys) -> Vec<VALUE>`(key/keys 均为 `DICTKEY`
//! 变体值,`keys` 通常来自 `subset::VARIANTS`)。存储结构(静态数组 /
//! `Vec` / 多语言切换)、缺失回退策略、加载与反序列化均由实现者自定。
//!
//! ```text
//! use i18n_dict::{dictkey, subset};
//!
//! #[dictkey(sort, deserialize)]
//! enum MyKey {
//!     welcome_title,
//!     welcome_body,
//! }
//!
//! subset!(MyKey, SettingsSubset, MyKey::welcome_title);
//! ```
//!
//! (The example needs the `macros` feature and a serde dependency, so it is
//! not compiled as a doctest; 示例需 `macros` feature 且依赖 serde,故不作 doctest)

/// Key-name abstraction for a dict enum.
///
/// Five associated constants describe the entry set:
/// - [`COUNT`](Self::COUNT): total entry count
/// - [`VARIANTS`](Self::VARIANTS): variant array (index = discriminant =
///   full-dictionary index)
/// - [`KEYS`](Self::KEYS): key-name array (same order as `VARIANTS`)
/// - [`SORTED_KEYS`](Self::SORTED_KEYS): alphabetical key-name array
///   (binary-search table)
/// - [`SORTED_VARIANTS`](Self::SORTED_VARIANTS): variants in `SORTED_KEYS`
///   order
///
/// `KEYS` / `VARIANTS` follow declaration order (alphabetical in sort mode)
/// and are **not guaranteed sorted**; for ordered access (e.g. binary
/// search) always use `SORTED_KEYS` / `SORTED_VARIANTS` — always
/// alphabetical, generated unconditionally, aliases of `KEYS` / `VARIANTS`
/// in sort mode.
///
/// [`find`](Self::find) maps a key name to a variant — shared by
/// deserialization and language-file loading. The default is linear search;
/// `#[dictkey]` / `#[derive(DictKey)]` generate a binary-search override
/// beyond 16 variants (based on `SORTED_KEYS` / `SORTED_VARIANTS`; linear
/// is faster for small tables).
///
/// The type must be `Copy`: `find` returns variants by value, and
/// `VARIANTS[i]` copies.
///
/// 中文:dict enum 的键名抽象。五个关联常量描述词条集合:`COUNT`(词条总数)、
/// `VARIANTS`(变体值数组,下标 = 判别值 = full 词典下标)、`KEYS`(键名数组,
/// 与 `VARIANTS` 同序)、`SORTED_KEYS` / `SORTED_VARIANTS`(字母序表,二分查找
/// 专用;sort 模式下与 `KEYS` / `VARIANTS` 同内容)。`KEYS` / `VARIANTS` 的
/// 顺序 = 声明顺序,**不保证排序**;需要有序访问时一律用 `SORTED_KEYS` /
/// `SORTED_VARIANTS`(恒为字母序,由宏无条件生成)。[`find`](Self::find) 把
/// 键名映射为变体值,供反序列化与语言文件加载共用:默认线性查找,变体数 >
/// 16 时宏生成二分覆写。类型必须 `Copy`(`find` 按值返回变体,`VARIANTS[i]`
/// 取值即拷贝)。
pub trait DictKey: Sized + Copy + 'static {
    /// Total entry count (full-dictionary size checks, placeholder capacity).
    /// 词条总数(full 词典条目数校验、占位容量)。
    const COUNT: usize;
    /// Variant array — index = discriminant = full-dictionary index.
    /// Static slice: the trait cannot express a fixed-length array type,
    /// so it points at a compile-time-generated static array.
    /// 变体值数组(下标 = 判别值 = full 词典下标);静态 slice(指向编译期
    /// 生成的静态数组,trait 无法表达定长数组类型)。
    const VARIANTS: &'static [Self];
    /// Key-name array (same order as `VARIANTS`).
    /// 键名数组(与 VARIANTS 同序)。
    const KEYS: &'static [&'static str];
    /// Alphabetical key-name array (binary-search table; identical to
    /// `KEYS` in sort mode). 字母序键名数组(二分查找专用表;sort 模式下与
    /// KEYS 同内容)。
    const SORTED_KEYS: &'static [&'static str];
    /// Variants in `SORTED_KEYS` order.
    /// 与 SORTED_KEYS 同序的变体值数组。
    const SORTED_VARIANTS: &'static [Self];

    /// Maps a key name to a variant; `None` if the name does not exist.
    /// Default is linear search; macros generate a binary-search override
    /// for large tables. 键名 → 变体值;键名不存在返回 `None`。默认线性查找,
    /// 数量大时宏生成二分覆写。
    fn find(name: &str) -> Option<Self> {
        Self::KEYS
            .iter()
            .position(|k| *k == name)
            .map(|i| Self::VARIANTS[i])
    }
}

/// Entry-container abstraction. Keys are [`DICTKEY`](Self::DICTKEY) variant
/// values (single [`get`](Self::get) / batch [`get_sub`](Self::get_sub)).
/// Storage layout and missing-key fallbacks are up to the implementor —
/// multi-language static arrays and sparse `Vec` storage both work:
///
/// ```ignore
/// use i18n_dict::{dictkey, subset, Dict};
///
/// #[dictkey]
/// enum MyKey { hello, world }
///
/// #[derive(Clone, Copy)]
/// enum Lang { Key, En, Cn }
///
/// struct MyDict {
///     lang: Lang,
///     en: [&'static str; MyKey::COUNT],
///     cn: [&'static str; MyKey::COUNT],
/// }
///
/// impl Dict for MyDict {
///     type DICTKEY = MyKey;
///     type VALUE = &'static str;
///     fn get(&self, key: MyKey) -> &'static str {
///         match self.lang {
///             // fallback: show the key name
///             Lang::Key => Self::DICTKEY::KEYS[key as usize],
///             Lang::En => self.en[key as usize],
///             Lang::Cn => self.cn[key as usize],
///         }
///     }
/// }
///
/// subset!(MyKey, UseKey, MyKey::world);
///
/// let dict = MyDict {
///     lang: Lang::En,
///     en: ["hello", "world"],
///     cn: ["你好", "世界"],
/// };
/// let strs = dict.get_sub(&UseKey::VARIANTS);
/// // position addressing: position within the subset = discriminant
/// assert_eq!(strs[UseKey::world as usize], "world");
/// ```
///
/// (The example needs the `macros` feature, so it is not compiled as a
/// doctest; 示例需 `macros` feature,故不作 doctest 编译)
///
/// 中文:词典抽象:词条容器。键 = [`DICTKEY`](Self::DICTKEY) 变体值(单条
/// [`get`](Self::get) / 批量 [`get_sub`](Self::get_sub))。存储结构、缺失回退
/// 策略由实现者自定——多语言一体(按当前语言切换数组)与 `Vec` 稀疏存储均
/// 可:实现者声明 `type DICTKEY: DictKey` 与 `type VALUE`,提供 `get` 与
/// `get_sub`(`keys` 通常来自 `subset::VARIANTS`,按 `keys` 顺序返回词条)。
pub trait Dict {
    /// Key type — the dict enum declared by the implementor.
    /// 键类型(实现者声明的 dict enum)。
    type DICTKEY: DictKey;
    /// Entry value type (e.g. `&'static str` statically embedded, `String`
    /// at runtime). 词条值类型(如 `&'static str` 静态嵌入、`String` 运行时)。
    type VALUE;
    /// Fetches the entry for `key` (`key` is a [`DICTKEY`](Self::DICTKEY)
    /// variant; missing-key/fallback handling is up to the implementor).
    /// 按键取词条(key 为 [`DICTKEY`](Self::DICTKEY) 变体值;缺失/回退的处理
    /// 由实现者决定)。
    fn get(&self, key: Self::DICTKEY) -> Self::VALUE;
    /// Batch fetch: returns entries in `keys` order (`keys` usually comes
    /// from `subset::VARIANTS`). 批量拉取:按 `keys` 顺序返回词条(`keys`
    /// 通常来自 `subset::VARIANTS`)。
    fn get_sub(&self, keys: &[Self::DICTKEY]) -> Vec<Self::VALUE> {
        let mut retval = Vec::with_capacity(keys.len());
        keys.iter().for_each(|&k| retval.push(self.get(k)));
        retval
    }
}

#[cfg(feature = "macros")]
pub use i18n_dict_macros::*;
