//! 词典运行时 crate。
//!
//! 核心是 [`DictKey`] trait:dict enum 的键名抽象。
//! 实现方式二选一(需 `macros` feature):
//!
//! ```toml
//! i18n-dict = { features = ["macros"] }
//! ```
//!
//! - **形态 A(推荐)** `#[dictkey]`:属性宏,实现 trait。默认与形态 B 一致:
//!   保留声明顺序,判别值 = 声明顺序索引;参数 `sort` 时重排变体为字母序,
//!   判别值 = 字母序位置(只依赖键名集合,与声明顺序无关);
//!   参数 `deserialize` 同时生成 `Deserialize`(需直接依赖 serde)
//! - **形态 B** `#[derive(DictKey)]`:保留声明顺序实现 trait,
//!   判别值 = 声明顺序索引;反序列化搭配 `#[derive(DictKeyDeserialize)]`
//!   (走 `find`,自动继承查找策略)
//!
//! 两种形态的 trait 接口一致:五常量 + `find`(键名 → 变体值)。
//! `KEYS` / `VARIANTS` 的顺序 = 声明顺序(sort 模式下为字母序),**不保证排序**;
//! 需要有序访问时一律用 `SORTED_KEYS` / `SORTED_VARIANTS`(恒为字母序,
//! sort 模式下二者是 `KEYS` / `VARIANTS` 的别名)。
//! 变体数量大(> 16)时 `find` 自动切二分,小则线性——策略由宏按数量决定;
//! 反序列化走 `find`,二分反序列化仅在变体数 > 16 且启用反序列化时出现。
//! trait 要求类型 `Copy`(find 按值返回变体),使用宏的 enum 请同时
//! `#[derive(Clone, Copy)]`。
//!
//! `subset!(Dict, Name, Dict::a, Dict::b)` 声明页面子集(仅限已实现
//! `DictKey` 的 enum,未实现会编译报错),生成 `Name::VARIANTS`
//! (子集变体数组,`dict.get_sub` 按此拉取)与每 key 的位置常量
//! (枚举判别值 = 子集内位置)。全局下标无需生成:`Dict::key as usize`
//! 即判别值。key 参数也可写裸变体名 `subset!(Dict, Name, a, b)`
//! (需 `use Dict::*`)。
//!
//! [`Dict`] 是词条容器抽象:实现者声明键类型(`type DICTKEY: DictKey`)
//! 与值类型(`type VALUE`),提供 `get(&self, key) -> VALUE`
//! 与 `get_sub(&self, keys) -> Vec<VALUE>`
//! (key/keys 均为 `DICTKEY` 变体值,`keys` 通常来自 `subset::VARIANTS`)。
//! 存储结构(静态数组 / `Vec` / 多语言切换)、缺失回退策略、
//! 加载与反序列化均由实现者自定。
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
//! (示例需 `macros` feature 且依赖 serde,故不作 doctest 编译)

/// dict enum 的键名抽象。
///
/// 五个关联常量描述词条集合:
/// - [`COUNT`](Self::COUNT):词条总数
/// - [`VARIANTS`](Self::VARIANTS):变体值数组(下标 = 判别值 = full 词典下标)
/// - [`KEYS`](Self::KEYS):键名数组(与 `VARIANTS` 同序)
/// - [`SORTED_KEYS`](Self::SORTED_KEYS):字母序键名数组(二分查找专用表)
/// - [`SORTED_VARIANTS`](Self::SORTED_VARIANTS):与 `SORTED_KEYS` 同序的变体值数组
///
/// `KEYS` / `VARIANTS` 的顺序 = 声明顺序(sort 模式下为字母序),**不保证排序**;
/// 需要有序访问(如二分)时一律用 `SORTED_KEYS` / `SORTED_VARIANTS`
/// (恒为字母序,由宏无条件生成;sort 模式下二者是 `KEYS` / `VARIANTS` 的别名)。
///
/// [`find`](Self::find) 把键名映射为变体值,供反序列化与语言文件加载共用。
/// 默认线性查找;`#[dictkey]` / `#[derive(DictKey)]` 在变体数 > 16 时
/// 生成二分覆写(基于 `SORTED_KEYS` / `SORTED_VARIANTS`,数量少时线性更快)。
///
/// 类型必须 `Copy`:`find` 按值返回变体值,`VARIANTS[i]` 取值即拷贝。
pub trait DictKey: Sized + Copy + 'static {
    /// 词条总数(full 词典条目数校验、占位容量)
    const COUNT: usize;
    /// 变体值数组(下标 = 判别值 = full 词典下标)。
    /// 静态 slice(指向编译期生成的静态数组,trait 无法表达定长数组类型)
    const VARIANTS: &'static [Self];
    /// 键名数组(与 VARIANTS 同序)
    const KEYS: &'static [&'static str];
    /// 字母序键名数组(二分查找专用表;sort 模式下与 KEYS 同内容)
    const SORTED_KEYS: &'static [&'static str];
    /// 与 SORTED_KEYS 同序的变体值数组
    const SORTED_VARIANTS: &'static [Self];

    /// 键名 → 变体值;键名不存在返回 `None`。
    /// 默认线性查找,数量大时宏生成二分覆写。
    fn find(name: &str) -> Option<Self> {
        Self::KEYS
            .iter()
            .position(|k| *k == name)
            .map(|i| Self::VARIANTS[i])
    }
}

/// 词典抽象:词条容器。键 = [`DICTKEY`](Self::DICTKEY) 变体值
/// (单条 [`get`](Self::get) / 批量 [`get_sub`](Self::get_sub))。
/// 存储结构、缺失回退策略由实现者自定——多语言一体
/// (按当前语言切换数组)与 `Vec` 稀疏存储均可:
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
///             Lang::Key => Self::DICTKEY::KEYS[key as usize], // 回退:显示键名
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
/// assert_eq!(strs[UseKey::world as usize], "world"); // 位置寻址
/// ```
///
/// (示例需 `macros` feature,故不作 doctest 编译)
pub trait Dict {
    /// 键类型(实现者声明的 dict enum)
    type DICTKEY: DictKey;
    /// 词条值类型(如 `&'static str` 静态嵌入、`String` 运行时)
    type VALUE;
    /// 按键取词条(key 为 [`DICTKEY`](Self::DICTKEY) 变体值;
    /// 缺失/回退的处理由实现者决定)
    fn get(&self, key: Self::DICTKEY) -> Self::VALUE;
    /// 批量拉取:按 `keys` 顺序返回词条(`keys` 通常来自 `subset::VARIANTS`)
    fn get_sub(&self, keys: &[Self::DICTKEY]) -> Vec<Self::VALUE> {
        let mut retval = Vec::with_capacity(keys.len());
        keys.iter().for_each(|&k| retval.push(self.get(k)));
        retval
    }
}

#[cfg(feature = "macros")]
pub use i18n_dict_macros::*;
