//! `i18n-dict-macros` 宏 + `DictKey` trait 的集成测试。
//!
//! 按最终使用形态组织:
//! - `#[derive(DictKey)]`(声明顺序)+ `#[derive(DictKeyDeserialize)]`
//! - `#[dictkey]` 属性宏(默认声明顺序)+ 参数 `sort` / `deserialize`
//! - `subset!` 路径语法 / 裸 ident / trait 断言
//! - 大 enum(> 16)的二分路径

use i18n_dict::DictKey as _; // trait 匿名导入(方法解析用;trait 名与 derive 宏同名,故不用具名导入)
use i18n_dict_macros::{DictKey, DictKeyDeserialize, dictkey, subset};

// ---------------------------------------------------------------------------
// 形态 B:#[derive(DictKey)] 保留声明顺序
// ---------------------------------------------------------------------------

#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, DictKey, DictKeyDeserialize)]
enum dict_key {
    welcome_title,
    welcome_body,
    footer,
}

#[test]
fn derive_keeps_declaration_order() {
    // 判别值 = 声明顺序索引,键名数组 = 声明顺序
    assert_eq!(dict_key::COUNT, 3);
    assert_eq!(dict_key::KEYS, &["welcome_title", "welcome_body", "footer"]);
    assert_eq!(dict_key::VARIANTS[0], dict_key::welcome_title);
    assert_eq!(dict_key::welcome_body as usize, 1);
    assert_eq!(dict_key::footer as usize, 2);
}

#[test]
fn derive_small_table_still_has_sorted_tables() {
    // 小表也无条件生成 SORTED_*(字母序表);find 走 trait 默认线性
    assert_eq!(
        dict_key::SORTED_KEYS,
        &["footer", "welcome_body", "welcome_title"]
    );
    assert_eq!(dict_key::SORTED_VARIANTS[0], dict_key::footer);
    assert_eq!(dict_key::SORTED_VARIANTS[2], dict_key::welcome_title);
}

#[test]
fn derive_find_uses_trait_interface() {
    // derive 形态小表:find = trait 默认线性实现,返回变体值
    assert_eq!(dict_key::find("welcome_body"), Some(dict_key::welcome_body));
    assert_eq!(dict_key::find("footer"), Some(dict_key::footer));
    assert_eq!(dict_key::find("unknown"), None);
}

#[test]
fn derive_deserialize_via_find() {
    // 小表(≤ 16)反序列化走 find 线性
    let key: dict_key = serde_json::from_str(r#""welcome_body""#).unwrap();
    assert_eq!(key, dict_key::welcome_body);
    let key: dict_key = serde_json::from_str(r#""footer""#).unwrap();
    assert_eq!(key as usize, 2);
    assert!(serde_json::from_str::<dict_key>(r#""unknown""#).is_err());
}

// ---------------------------------------------------------------------------
// 形态 A:#[dictkey] 属性宏,参数 sort 时变体重排为字母序
// ---------------------------------------------------------------------------

#[dictkey(sort, deserialize)]
#[repr(usize)]
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum sorted_key {
    welcome_title,
    welcome_body,
    footer,
}

#[test]
fn dictkey_sorts_variants() {
    // 声明顺序 welcome_title, welcome_body, footer
    // 展开后    footer, welcome_body, welcome_title(判别值 = 字母序位置)
    assert_eq!(sorted_key::COUNT, 3);
    assert_eq!(
        sorted_key::KEYS,
        &["footer", "welcome_body", "welcome_title"]
    );
    assert_eq!(sorted_key::VARIANTS[0], sorted_key::footer);
    assert_eq!(sorted_key::footer as usize, 0);
    assert_eq!(sorted_key::welcome_body as usize, 1);
    assert_eq!(sorted_key::welcome_title as usize, 2);
}

#[test]
fn dictkey_find_after_sort() {
    assert_eq!(
        sorted_key::find("welcome_title"),
        Some(sorted_key::welcome_title)
    );
    assert_eq!(sorted_key::find("footer"), Some(sorted_key::footer));
    assert_eq!(sorted_key::find("unknown"), None);
}

#[test]
fn dictkey_sorted_tables_are_aliases_small() {
    // sort 模式:小表也生成 SORTED_*,与 KEYS/VARIANTS 同内容(别名)
    assert_eq!(sorted_key::SORTED_KEYS, sorted_key::KEYS);
    assert_eq!(sorted_key::SORTED_VARIANTS, sorted_key::VARIANTS);
}

#[test]
fn dictkey_deserialize_via_find() {
    let key: sorted_key = serde_json::from_str(r#""welcome_title""#).unwrap();
    assert_eq!(key as usize, 2);
    let key: sorted_key = serde_json::from_str(r#""footer""#).unwrap();
    assert_eq!(key, sorted_key::footer);
    assert!(serde_json::from_str::<sorted_key>(r#""unknown""#).is_err());
}

#[test]
fn dictkey_sort_discriminants_depend_only_on_key_set() {
    // sort 模式:声明顺序颠倒、键名集合相同的另一个 enum:判别值完全一致
    #[dictkey(sort)]
    #[allow(non_camel_case_types)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum shuffled {
        footer,
        welcome_title,
        welcome_body,
    }
    assert_eq!(shuffled::footer as usize, sorted_key::footer as usize);
    assert_eq!(
        shuffled::welcome_body as usize,
        sorted_key::welcome_body as usize
    );
    assert_eq!(
        shuffled::welcome_title as usize,
        sorted_key::welcome_title as usize
    );
    assert_eq!(shuffled::KEYS, sorted_key::KEYS);
}

// ---------------------------------------------------------------------------
// 大 enum(> 16):二分路径
// ---------------------------------------------------------------------------

#[dictkey(deserialize)]
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum big_key {
    // 声明顺序故意打乱
    k19,
    k15,
    k00,
    k04,
    k11,
    k07,
    k03,
    k16,
    k12,
    k08,
    k01,
    k17,
    k05,
    k13,
    k09,
    k02,
    k18,
    k06,
    k14,
    k10,
}

#[test]
fn big_enum_binary_table() {
    assert_eq!(big_key::COUNT, 20);
    // 默认不排序:KEYS = 声明顺序(与字母序无关)
    assert_eq!(big_key::KEYS[0], "k19");
    assert_eq!(big_key::KEYS[19], "k10");
    // 判别值 = 声明顺序索引
    assert_eq!(big_key::k00 as usize, 2);
    assert_eq!(big_key::k10 as usize, 19);
    assert_eq!(big_key::k19 as usize, 0);
    // SORTED_* = 独立的字母序表(二分专用,与 KEYS 不同)
    assert_eq!(big_key::SORTED_KEYS[0], "k00");
    assert_eq!(big_key::SORTED_KEYS[19], "k19");
    assert_eq!(big_key::SORTED_VARIANTS[0], big_key::k00);
    assert_eq!(big_key::SORTED_VARIANTS[19], big_key::k19);
}

#[test]
fn big_enum_find_binary_search() {
    // 默认不排序 + 大表:find 二分覆写基于 SORTED_KEYS,返回变体值
    assert_eq!(big_key::find("k00"), Some(big_key::k00));
    assert_eq!(big_key::find("k10"), Some(big_key::k10));
    assert_eq!(big_key::find("k19"), Some(big_key::k19));
    assert_eq!(big_key::find("k99"), None);
}

#[test]
fn big_enum_deserialize() {
    // dictkey(deserialize):大表走 find(已被覆写为二分)
    let key: big_key = serde_json::from_str(r#""k07""#).unwrap();
    assert_eq!(key as usize, 5); // k07 的声明顺序索引
    assert_eq!(key, big_key::k07);
}

// ---------------------------------------------------------------------------
// 形态 B 的大 enum(> 16):DictKeyDeserialize 生成排序表 + 二分
// ---------------------------------------------------------------------------

#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, DictKey, DictKeyDeserialize)]
enum big_derive {
    k19,
    k15,
    k00,
    k04,
    k11,
    k07,
    k03,
    k16,
    k12,
    k08,
    k01,
    k17,
    k05,
    k13,
    k09,
    k02,
    k18,
    k06,
    k14,
    k10,
}

#[test]
fn big_derive_find_binary_search() {
    // DictKey 大表:SORTED_KEYS / SORTED_VARIANTS + find 二分覆写,
    // find 返回变体值(判别值 = 声明顺序索引)
    assert_eq!(big_derive::SORTED_KEYS[0], "k00");
    assert_eq!(big_derive::SORTED_KEYS[19], "k19");
    assert_eq!(big_derive::SORTED_VARIANTS[0], big_derive::k00); // 字母序首位对应的变体
    assert_eq!(big_derive::find("k00"), Some(big_derive::k00));
    assert_eq!(big_derive::find("k10"), Some(big_derive::k10));
    assert_eq!(big_derive::find("k19"), Some(big_derive::k19));
    assert_eq!(big_derive::find("k99"), None);
    assert_eq!(big_derive::k00 as usize, 2);
}

#[test]
fn big_derive_deserialize_via_find() {
    // 反序列化走 trait find(此处为二分覆写)
    let key: big_derive = serde_json::from_str(r#""k07""#).unwrap();
    assert_eq!(key as usize, 5);
    let key: big_derive = serde_json::from_str(r#""k00""#).unwrap();
    assert_eq!(key, big_derive::k00);
}

// ---------------------------------------------------------------------------
// subset!:路径语法 / 裸 ident / trait 断言
// ---------------------------------------------------------------------------

subset!(
    sorted_key,
    settings_subset,
    sorted_key::footer,
    sorted_key::welcome_title
);

#[test]
fn subset_indexes_are_dictkey_discriminants() {
    // VARIANTS:子集变体数组,按声明顺序(dict.get_sub 按此拉取)
    assert_eq!(
        settings_subset::VARIANTS,
        [sorted_key::footer, sorted_key::welcome_title]
    );
    // 全局下标无需生成:dict 判别值即下标
    assert_eq!(sorted_key::footer as usize, 0);
    assert_eq!(sorted_key::welcome_title as usize, 2);
}

#[test]
fn subset_positions_follow_declaration_order() {
    // 位置 = enum 判别值(子集声明顺序),与全局索引无关
    assert_eq!(settings_subset::footer as usize, 0);
    assert_eq!(settings_subset::welcome_title as usize, 1);
}

// 裸 ident 形态(用户 use Dict::* 后 key 可直接写变体名)
use sorted_key::*;
subset!(sorted_key, bare_sub, footer, welcome_title);

#[test]
fn subset_accepts_bare_idents() {
    assert_eq!(
        bare_sub::VARIANTS,
        [sorted_key::footer, sorted_key::welcome_title]
    );
    assert_eq!(bare_sub::footer as usize, 0);
    assert_eq!(bare_sub::welcome_title as usize, 1);
}

#[test]
fn usage_matches_pseudocode() {
    // 伪代码:
    //   subset!(mydict, mysub, mydict::b, mydict::c);
    //   let x: Vec<String> = dict.get_sub(mysub::VARIANTS);
    //   x[mysub::b] == "b 的译文"
    //
    // get_sub 默认实现:按 keys 顺序逐个 get;此处以 map 模拟其"按序拷出"语义。
    let full = ["页脚", "你好", "欢迎"]; // full 词典,下标 = sorted_key 判别值

    let x: Vec<String> = settings_subset::VARIANTS
        .iter()
        .map(|&k| full[k as usize].to_string())
        .collect();
    // 子集数据顺序 = 声明顺序(footer, welcome_title)

    // 位置 = 判别值,as usize 直接寻址,零查找
    assert_eq!(x[settings_subset::footer as usize], "页脚");
    assert_eq!(x[settings_subset::welcome_title as usize], "欢迎");
}

// ---------------------------------------------------------------------------
// 默认不排序(小表)/ sort 大表别名 / 空 enum
// ---------------------------------------------------------------------------

#[dictkey(deserialize)]
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum plain_key {
    zeta,
    alpha,
    mid,
}

#[test]
fn dictkey_keeps_declaration_order_by_default() {
    // 默认与 derive 一致:保留声明顺序,判别值 = 声明顺序索引
    assert_eq!(plain_key::COUNT, 3);
    assert_eq!(plain_key::KEYS, &["zeta", "alpha", "mid"]);
    assert_eq!(plain_key::zeta as usize, 0);
    assert_eq!(plain_key::alpha as usize, 1);
    assert_eq!(plain_key::mid as usize, 2);
    // SORTED_* = 字母序表(与 KEYS 不同)
    assert_eq!(plain_key::SORTED_KEYS, &["alpha", "mid", "zeta"]);
    assert_eq!(plain_key::SORTED_VARIANTS[0], plain_key::alpha);
    // find 返回变体值
    assert_eq!(plain_key::find("alpha"), Some(plain_key::alpha));
    assert_eq!(plain_key::find("unknown"), None);
    // 反序列化 roundtrip
    let key: plain_key = serde_json::from_str(r#""alpha""#).unwrap();
    assert_eq!(key, plain_key::alpha);
    assert_eq!(key as usize, 1);
}

#[dictkey(sort, deserialize)]
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum big_sorted {
    // 声明顺序故意打乱
    k19,
    k15,
    k00,
    k04,
    k11,
    k07,
    k03,
    k16,
    k12,
    k08,
    k01,
    k17,
    k05,
    k13,
    k09,
    k02,
    k18,
    k06,
    k14,
    k10,
}

#[test]
fn big_sorted_tables_are_aliases() {
    // sort 大表:SORTED_* 与 KEYS / VARIANTS 同内容(别名)
    assert_eq!(big_sorted::SORTED_KEYS, big_sorted::KEYS);
    assert_eq!(big_sorted::SORTED_VARIANTS, big_sorted::VARIANTS);
    // 判别值 = 字母序位置
    assert_eq!(big_sorted::KEYS[0], "k00");
    assert_eq!(big_sorted::k00 as usize, 0);
    assert_eq!(big_sorted::k10 as usize, 10);
    assert_eq!(big_sorted::k19 as usize, 19);
    // find 二分 + 反序列化
    assert_eq!(big_sorted::find("k00"), Some(big_sorted::k00));
    assert_eq!(big_sorted::find("k99"), None);
    let key: big_sorted = serde_json::from_str(r#""k07""#).unwrap();
    assert_eq!(key, big_sorted::k07);
}

#[dictkey]
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum empty_key {}

#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, DictKey)]
enum empty_derive {}

#[test]
fn empty_enum() {
    // 空 enum 两形态均正常:SORTED_* 为空数组,find 返回 None
    assert_eq!(empty_key::COUNT, 0);
    assert!(empty_key::KEYS.is_empty());
    assert!(empty_key::SORTED_KEYS.is_empty());
    assert!(empty_key::SORTED_VARIANTS.is_empty());
    assert_eq!(empty_key::find("x"), None);

    assert_eq!(empty_derive::COUNT, 0);
    assert!(empty_derive::KEYS.is_empty());
    assert!(empty_derive::SORTED_KEYS.is_empty());
    assert!(empty_derive::SORTED_VARIANTS.is_empty());
    assert_eq!(empty_derive::find("x"), None);
}
