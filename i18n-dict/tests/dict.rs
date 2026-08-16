//! 端到端测试:通过主 crate 的 `macros` feature 使用宏,
//! 与真实用户 `i18n-dict = { features = ["macros"] }` 的用法一致。

use i18n_dict::DictKey as _; // trait 匿名导入(find 方法解析用)
use i18n_dict::{Dict, dictkey, subset};

#[dictkey(deserialize)]
#[repr(usize)]
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DictKey {
    welcome_title,
    welcome_body,
    footer,
}

subset!(
    DictKey,
    settings_subset,
    DictKey::welcome_body,
    DictKey::footer
);

#[test]
fn end_to_end() {
    // 默认不排序:判别值 = 声明顺序索引(welcome_title=0, welcome_body=1, footer=2)
    assert_eq!(DictKey::COUNT, 3);
    assert_eq!(DictKey::KEYS, &["welcome_title", "welcome_body", "footer"]);
    assert_eq!(DictKey::VARIANTS[0], DictKey::welcome_title);
    // SORTED_* = 字母序表(与 KEYS 不同)
    assert_eq!(
        DictKey::SORTED_KEYS,
        &["footer", "welcome_body", "welcome_title"]
    );
    assert_eq!(DictKey::SORTED_VARIANTS[0], DictKey::footer);

    // find:键名 → 变体值
    assert_eq!(DictKey::find("welcome_body"), Some(DictKey::welcome_body));
    assert_eq!(DictKey::find("unknown"), None);

    // 反序列化:trait find 链(键名 → 变体值)
    let key: DictKey = serde_json::from_str(r#""welcome_body""#).unwrap();
    assert_eq!(key as usize, 1);

    // subset:子集变体数组 + 位置(判别值);全局下标 = 判别值 as usize
    assert_eq!(
        settings_subset::VARIANTS,
        [DictKey::welcome_body, DictKey::footer]
    );
    assert_eq!(settings_subset::welcome_body as usize, 0);
    assert_eq!(settings_subset::footer as usize, 1);
    assert_eq!(DictKey::welcome_body as usize, 1);
    assert_eq!(DictKey::footer as usize, 2);

    // 伪代码链路:按子集变体拉取 → 位置直接寻址(下标 = 判别值 as usize)
    let full = ["欢迎", "你好", "页脚"]; // full 词典,下标 = DictKey 判别值
    let x: Vec<String> = settings_subset::VARIANTS
        .iter()
        .map(|&k| full[k as usize].to_string())
        .collect();
    assert_eq!(x[settings_subset::welcome_body as usize], "你好");
    assert_eq!(x[settings_subset::footer as usize], "页脚");
}

// sort 模式:完整保留旧「字母序判别值」语义的回归覆盖

#[dictkey(sort, deserialize)]
#[repr(usize)]
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SortedKey {
    welcome_title,
    welcome_body,
    footer,
}

subset!(
    SortedKey,
    sorted_settings_subset,
    SortedKey::welcome_body,
    SortedKey::footer
);

#[test]
fn end_to_end_sort_mode() {
    // sort 模式:判别值 = 字母序位置(footer=0, welcome_body=1, welcome_title=2)
    assert_eq!(SortedKey::COUNT, 3);
    assert_eq!(
        SortedKey::KEYS,
        &["footer", "welcome_body", "welcome_title"]
    );
    assert_eq!(SortedKey::footer as usize, 0);
    assert_eq!(SortedKey::welcome_body as usize, 1);
    assert_eq!(SortedKey::welcome_title as usize, 2);
    // SORTED_* 与 KEYS / VARIANTS 同内容(别名)
    assert_eq!(SortedKey::SORTED_KEYS, SortedKey::KEYS);
    assert_eq!(SortedKey::SORTED_VARIANTS, SortedKey::VARIANTS);

    // find + 反序列化
    assert_eq!(
        SortedKey::find("welcome_body"),
        Some(SortedKey::welcome_body)
    );
    let key: SortedKey = serde_json::from_str(r#""footer""#).unwrap();
    assert_eq!(key as usize, 0);

    // subset:子集变体数组 + 位置(判别值);全局下标 = 判别值 as usize
    assert_eq!(
        sorted_settings_subset::VARIANTS,
        [SortedKey::welcome_body, SortedKey::footer]
    );
    assert_eq!(sorted_settings_subset::welcome_body as usize, 0);
    assert_eq!(sorted_settings_subset::footer as usize, 1);
    assert_eq!(SortedKey::welcome_body as usize, 1);
    assert_eq!(SortedKey::footer as usize, 0);
}

// ---------------------------------------------------------------------------
// Dict trait:词条容器抽象(用户例子的可编译版)
// ---------------------------------------------------------------------------

#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum lang {
    key,
    en,
    cn,
}

#[dictkey]
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mydictkey {
    hello,
    world,
}

// Mydict1:多语言一体结构,lang 切换语言;key 模式回退显示键名
struct Mydict1 {
    lang: lang,
    en: [&'static str; Mydictkey::COUNT],
    cn: [&'static str; Mydictkey::COUNT],
}

impl Dict for Mydict1 {
    type DICTKEY = Mydictkey;
    type VALUE = &'static str;
    fn get(&self, key: Mydictkey) -> &'static str {
        match self.lang {
            lang::key => Self::DICTKEY::KEYS[key as usize], // 回退:显示键名
            lang::en => self.en[key as usize],
            lang::cn => self.cn[key as usize],
        }
    }
}

subset!(Mydictkey, usekey, Mydictkey::world);

#[test]
fn dict_multi_lang_and_key_fallback() {
    let mut dict = Mydict1 {
        lang: lang::en,
        en: ["hello", "world"],
        cn: ["你好", "世界"],
    };
    // 默认不排序:hello=0, world=1
    assert_eq!(usekey::VARIANTS, [Mydictkey::world]); // 子集变体数组
    assert_eq!(usekey::world as usize, 0); // 子集内位置(参数顺序)
    assert_eq!(Mydictkey::world as usize, 1); // 全局下标 = 判别值
    let strs = dict.get_sub(&usekey::VARIANTS);
    assert_eq!(strs[0], "world"); // 位置寻址:strs[usekey::world as usize]
    // 直接按键取词条(新接口:get 接收键而非下标)
    assert_eq!(dict.get(Mydictkey::hello), "hello");
    assert_eq!(dict.get(Mydictkey::world), "world");
    dict.lang = lang::key;
    assert_eq!(strs[0], "world"); // 已借出(&'static str 拷贝),不随 lang 变
    let strs = dict.get_sub(&usekey::VARIANTS);
    assert_eq!(strs[0], "world"); // key 回退 = 键名 KEYS[1] = "world"
    dict.lang = lang::cn;
    let strs = dict.get_sub(&usekey::VARIANTS);
    assert_eq!(strs[0], "世界");
}

// Mydict2:Vec 稀疏存储,缺词条回退 "NULL"
struct Mydict2 {
    v: Vec<String>,
}

impl Dict for Mydict2 {
    type DICTKEY = Mydictkey;
    type VALUE = String;
    fn get(&self, key: Mydictkey) -> String {
        match self.v.get(key as usize) {
            Some(s) => s.clone(),
            None => "NULL".to_string(),
        }
    }
}

#[test]
fn dict_sparse_vec_fallback() {
    let dict2en = Mydict2 {
        v: vec!["hello".into(), "world".into()],
    };
    let strs = dict2en.get_sub(&usekey::VARIANTS);
    assert_eq!(strs[0], "world");
    // 稀疏:缺 world(下标 1)时回退 "NULL"
    let dict2cn = Mydict2 {
        v: vec!["你好".into()],
    };
    let strs = dict2cn.get_sub(&usekey::VARIANTS);
    assert_eq!(strs[0], "NULL");
}
