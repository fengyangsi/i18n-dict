//! `i18n-dict-macros`: procedural macros for i18n dictionaries.
//!
//! Generates implementations of the
//! [`DictKey`](https://docs.rs/i18n-dict) trait (plus optional
//! `serde::Deserialize`), providing four macros:
//!
//! - `#[dictkey]` — attribute macro: implements the trait, keeping
//!   declaration order by default like the derive (discriminant =
//!   declaration index); with `sort` variants are reordered
//!   **alphabetically** (discriminant = alphabetical position); with
//!   `deserialize` a deserialization impl is generated too
//! - `#[derive(DictKey)]` — derive macro: implements the trait keeping
//!   declaration order, discriminant = declaration index
//! - `#[derive(DictKeyDeserialize)]` — deserialization impl (use with
//!   `DictKey`; goes through the trait `find`, inheriting its lookup
//!   strategy)
//! - `subset!(dict, name, dict::key1, dict::key2, ...)` — page subset
//!   declaration
//!
//! Both forms unconditionally generate `SORTED_KEYS` / `SORTED_VARIANTS`
//! (alphabetical tables); beyond 16 variants a binary-search `find`
//! override is generated (linear is faster for small tables, which use the
//! trait default).
//!
//! Typical usage (pick one of the two forms):
//!
//! ```ignore
//! use i18n_dict::{dictkey, DictKey, DictKeyDeserialize, subset};
//!
//! // Form A (recommended): #[dictkey] attribute, declaration order by
//! // default; sort reorders alphabetically
//! #[dictkey(sort, deserialize)]  // sort: alphabetical; deserialize: also generate Deserialize
//! enum mydict { a, b, c }
//!
//! // Form B: #[derive(DictKey)] keeps declaration order, discriminant =
//! // declaration index
//! #[derive(DictKey, DictKeyDeserialize)]
//! enum other { a, b, c }
//!
//! subset!(mydict, mysub, mydict::b, mydict::c);
//! ```
//!
//! All macro expansions are pure std code and do not depend on the runtime
//! crate (serde-related macros require the user to depend on serde
//! directly).
//!
//! ---
//!
//! 中文说明:`i18n-dict-macros` 是词典宏 crate,生成
//! [`DictKey`](https://docs.rs/i18n-dict) trait 的实现(以及可选的
//! `serde::Deserialize`),提供四个宏:
//!
//! - `#[dictkey]` — 属性宏:实现 trait,默认与 derive 一致保留声明顺序
//!   (判别值 = 声明顺序索引);参数 `sort` 时重排为**字母序**(判别值 =
//!   字母序位置);参数 `deserialize` 同时生成反序列化
//! - `#[derive(DictKey)]` — derive 宏:保留声明顺序实现 trait,
//!   判别值 = 声明顺序索引
//! - `#[derive(DictKeyDeserialize)]` — 反序列化实现(配 `DictKey` 用,
//!   走 trait `find`,自动继承其查找策略)
//! - `subset!(dict, name, dict::key1, dict::key2, ...)` — 页面子集声明
//!
//! 两种形态都无条件生成 `SORTED_KEYS` / `SORTED_VARIANTS`(字母序表);
//! 变体数 > 16 时生成 `find` 二分覆写(小表线性更快,走 trait 默认实现)。
//! 宏展开均为纯 std 代码,不依赖运行时 crate(serde 相关宏需用户自行依赖
//! serde)。

use proc_macro::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{
    Data, DeriveInput, Fields, Ident, Path, Token, parse_macro_input, punctuated::Punctuated,
};

/// 二分查找阈值:变体数量超过此值时线性查找变慢,
/// 生成排序表并二分;小表直接线性(trait 默认 `find` 实现)
const BINARY_SEARCH_THRESHOLD: usize = 16;

// ===========================================================================
// 反序列化 impl(#[dictkey(deserialize)] 与 #[derive(DictKeyDeserialize)] 共用)
// ===========================================================================

/// 反序列化 impl:`#[dictkey(deserialize)]` 与 `#[derive(DictKeyDeserialize)]` 共用。
///
/// 统一走 trait `find`——查找策略由 `DictKey` 实现决定:
/// 小表线性(默认),大表宏已覆写为二分。不生成任何附加表。
fn deserialize_impl(name: &Ident) -> proc_macro2::TokenStream {
    quote! {
        #[automatically_derived]
        impl<'de> ::serde::Deserialize<'de> for #name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: ::serde::Deserializer<'de>,
            {
                struct __Visitor;

                impl<'de> ::serde::de::Visitor<'de> for __Visitor {
                    type Value = #name;

                    fn expecting(
                        &self,
                        f: &mut ::core::fmt::Formatter<'_>,
                    ) -> ::core::fmt::Result {
                        f.write_str("DictKey 键名")
                    }

                    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
                    where
                        E: ::serde::de::Error,
                    {
                        match <#name as ::i18n_dict::DictKey>::find(v) {
                            ::core::option::Option::Some(key) => {
                                ::core::result::Result::Ok(key)
                            }
                            ::core::option::Option::None => Err(E::unknown_variant(
                                v,
                                <#name as ::i18n_dict::DictKey>::KEYS,
                            )),
                        }
                    }

                    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
                    where
                        E: ::serde::de::Error,
                    {
                        match ::core::str::from_utf8(v) {
                            ::core::result::Result::Ok(s) => self.visit_str(s),
                            ::core::result::Result::Err(_) => Err(E::invalid_value(
                                ::serde::de::Unexpected::Bytes(v),
                                &self,
                            )),
                        }
                    }
                }

                deserializer.deserialize_str(__Visitor)
            }
        }
    }
}

// ===========================================================================
// #[dictkey] 属性宏
// ===========================================================================

/// Attribute macro: implements the
/// [`DictKey`](https://docs.rs/i18n-dict) trait.
///
/// **Keeps declaration order by default** (same as `#[derive(DictKey)]`),
/// discriminant = declaration index; with `sort` variants are reordered
/// **alphabetically**, discriminant = alphabetical position (depends only
/// on the key-name set, not on declaration order).
///
/// Both modes unconditionally generate `SORTED_KEYS` / `SORTED_VARIANTS`
/// (alphabetical tables; identical to `KEYS` / `VARIANTS` in sort mode).
/// Beyond 16 variants a binary-search `find` override is generated (based
/// on `SORTED_KEYS`; linear is faster for small tables, which use the
/// trait default).
///
/// Arguments (optional, comma-separated, any order):
/// `#[dictkey(sort, deserialize)]`
/// - `sort`: reorder variants alphabetically (default: declaration order)
/// - `deserialize`: also generate a deserialization impl (requires the
///   user to depend on serde directly)
///
/// Combinatorics: without `deserialize` you may additionally
/// `#[derive(DictKeyDeserialize)]` (functionally equivalent to the
/// built-in `deserialize`); with `deserialize` do **not** derive it again
/// (duplicate serde impl, E0119). Mutually exclusive with
/// `#[derive(DictKey)]` (duplicate trait impl).
///
/// Restrictions:
/// - variants must be unit (field-less)
/// - explicit discriminants (`= n`) are not allowed — the macro owns them
/// - variants cannot carry `#[cfg]` / `#[cfg_attr]` (expansion precedes
///   cfg stripping; positions would shift)
/// - the enum must be `Copy` (trait requirement); derive `Clone, Copy`
///
/// ```ignore
/// #[dictkey(sort, deserialize)]
/// enum mydict { welcome_title, welcome_body, footer }  // sorted alphabetically
/// ```
///
/// 中文:属性宏,实现 [`DictKey`](https://docs.rs/i18n-dict) trait。**默认保留
/// 声明顺序**(与 `#[derive(DictKey)]` 一致),判别值 = 声明顺序索引;参数
/// `sort` 时重排变体为**字母序**,判别值 = 字母序位置(只依赖键名集合,与
/// 声明顺序无关)。两种模式都无条件生成 `SORTED_KEYS` / `SORTED_VARIANTS`
/// (字母序表;sort 模式下与 `KEYS` / `VARIANTS` 同内容)。变体数 > 16 时
/// 自动生成 `find` 二分覆写(基于 `SORTED_KEYS`,小表线性更快,用 trait
/// 默认实现)。参数(可选,逗号分隔、顺序任意):`sort`(重排为字母序)、
/// `deserialize`(同时生成反序列化实现,需用户直接依赖 serde)。组合说明:
/// 本宏**不带** `deserialize` 时,可另行 `#[derive(DictKeyDeserialize)]`
/// (功能等价);带 `deserialize` 时**不要**再 derive 它(会重复实现 serde,
/// E0119);与 `#[derive(DictKey)]` 二选一,不可混用。限制:变体必须是无
/// 字段(unit)变体;不能写显式判别值(`= n`);变体不能带 `#[cfg]` /
/// `#[cfg_attr]`(宏展开先于 cfg 剔除,会错位);enum 需 `Copy`,请同时
/// `#[derive(Clone, Copy)]`。
#[proc_macro_attribute]
pub fn dictkey(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attr = parse_macro_input!(attr as DictKeyAttr);
    let input = parse_macro_input!(item as DeriveInput);
    let name = &input.ident;

    // 只接受 enum;变体必须是无字段的 unit 变体(与语言文件键一一对应)
    let Data::Enum(data) = &input.data else {
        return syn::Error::new_spanned(name, "dictkey 只能用于 enum")
            .to_compile_error()
            .into();
    };

    // 生成的关联项同名会触发 E0592,提前给出友好报错
    const RESERVED: &[&str] = &[
        "COUNT",
        "VARIANTS",
        "KEYS",
        "SORTED_KEYS",
        "SORTED_VARIANTS",
    ];

    // (键名, 原位置):判别值 = 字母序位置,排序后输出变体
    let mut entries = Vec::new();
    for (i, variant) in data.variants.iter().enumerate() {
        if !matches!(variant.fields, Fields::Unit) {
            return syn::Error::new_spanned(variant, "dictkey 变体必须是无字段(unit)变体")
                .to_compile_error()
                .into();
        }
        if variant.discriminant.is_some() {
            return syn::Error::new_spanned(
                variant,
                "dictkey 变体不能写显式判别值;判别值 = 字母序位置,由宏决定",
            )
            .to_compile_error()
            .into();
        }
        if variant
            .attrs
            .iter()
            .any(|a| a.path().is_ident("cfg") || a.path().is_ident("cfg_attr"))
        {
            return syn::Error::new_spanned(
                variant,
                "dictkey 变体不能带 #[cfg]:排序发生在 cfg 剔除之前,会错位",
            )
            .to_compile_error()
            .into();
        }
        let key = variant.ident.to_string();
        if RESERVED.contains(&key.as_str()) {
            return syn::Error::new_spanned(
                &variant.ident,
                format!(
                    "变体名 `{key}` 与生成的关联项同名,请改名; \
                     如需保留该键名,可加 #[serde(rename = \"{key}\")] 解耦"
                ),
            )
            .to_compile_error()
            .into();
        }
        entries.push((key, i));
    }

    // sort 模式:编译期按字母序重排(键名唯一,无重复);输出变体保留原属性(serde rename 等)
    if attr.sort {
        entries.sort();
    }
    let ordered_variants = entries.iter().map(|(_, i)| &data.variants[*i]);
    let ordered_idents = entries.iter().map(|(_, i)| &data.variants[*i].ident);
    let keys = entries.iter().map(|(k, _)| proc_macro2::Literal::string(k));
    let len = entries.len();
    let attrs = &input.attrs;
    let vis = &input.vis;
    let generics = &input.generics;

    // SORTED_* 无条件生成:sort 模式下与 KEYS / VARIANTS 同内容(别名);
    // 非 sort 模式下是独立的字母序表
    let sorted_consts = if attr.sort {
        quote! {
            /// 键名数组(字母序;与 KEYS 同内容)
            #[doc(hidden)]
            const SORTED_KEYS: &'static [&'static str] = Self::KEYS;
            /// 变体值数组(字母序;与 VARIANTS 同内容)
            #[doc(hidden)]
            const SORTED_VARIANTS: &'static [Self] = Self::VARIANTS;
        }
    } else {
        let mut sorted = entries.clone();
        sorted.sort();
        let sorted_keys = sorted.iter().map(|(k, _)| proc_macro2::Literal::string(k));
        let sorted_idents = sorted.iter().map(|(_, i)| &data.variants[*i].ident);
        quote! {
            /// 键名数组(字母序,二分查找专用表)
            #[doc(hidden)]
            const SORTED_KEYS: &'static [&'static str] = &[#(#sorted_keys),*];
            /// 与 SORTED_KEYS 同序的变体值数组
            #[doc(hidden)]
            const SORTED_VARIANTS: &'static [Self] = &[#(Self::#sorted_idents),*];
        }
    };

    // find 二分覆写:统一基于 SORTED_KEYS / SORTED_VARIANTS(数量大时;小表线性更快)
    let find_impl = if len > BINARY_SEARCH_THRESHOLD {
        quote! {
            fn find(name: &str) -> Option<Self> {
                Self::SORTED_KEYS
                    .binary_search(&name)
                    .ok()
                    .map(|i| Self::SORTED_VARIANTS[i])
            }
        }
    } else {
        proc_macro2::TokenStream::new()
    };

    let deser = if attr.deserialize {
        deserialize_impl(name)
    } else {
        proc_macro2::TokenStream::new()
    };

    quote! {
        #(#attrs)*
        #[allow(non_camel_case_types)]
        #vis enum #name #generics {
            #(#ordered_variants),*
        }
        #[automatically_derived]
        impl #generics ::i18n_dict::DictKey for #name #generics {
            const COUNT: usize = #len;
            const VARIANTS: &'static [Self] = &[#(Self::#ordered_idents),*];
            const KEYS: &'static [&'static str] = &[#(#keys),*];
            #sorted_consts
            #find_impl
        }
        #deser
    }
    .into()
}

/// `#[dictkey(...)]` 参数:无参标志 `sort` / `deserialize`
/// (顺序任意、可重复;不支持 `= true` / `= on` / `= off` 等形式)
struct DictKeyAttr {
    sort: bool,
    deserialize: bool,
}

impl Parse for DictKeyAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut attr = DictKeyAttr {
            sort: false,
            deserialize: false,
        };
        if input.is_empty() {
            return Ok(attr);
        }
        for flag in Punctuated::<Ident, Token![,]>::parse_terminated(input)? {
            if flag == "sort" {
                attr.sort = true;
            } else if flag == "deserialize" {
                attr.deserialize = true;
            } else {
                return Err(syn::Error::new_spanned(
                    flag,
                    "未知参数,仅支持无参标志 `sort` 与 `deserialize`",
                ));
            }
        }
        Ok(attr)
    }
}

// ===========================================================================
// #[derive(DictKey)]
// ===========================================================================

/// Derive macro: implements the
/// [`DictKey`](https://docs.rs/i18n-dict) trait keeping declaration order,
/// discriminant = declaration index (same as the full-dictionary array
/// index).
///
/// Unconditionally generates `SORTED_KEYS` / `SORTED_VARIANTS`
/// (alphabetical tables for binary search); beyond 16 variants a
/// binary-search `find` override is generated (O(log n) instead of O(n));
/// small tables (≤ 16) use the trait's default linear implementation
/// (linear is faster at small sizes, due to CPU caches/pipelining).
///
/// Pair with `#[derive(DictKeyDeserialize)]` for deserialization — it goes
/// through the trait `find` (inheriting the binary-search strategy), no
/// extra tables.
///
/// Mutually exclusive with `#[dictkey]`: this macro never reorders
/// variants; use `#[dictkey(sort)]` for alphabetical discriminants.
///
/// The enum must be `Copy` (trait requirement); derive `Clone, Copy`.
///
/// Variant names are used **verbatim** as key-name strings, no case
/// forcing; consider `#[serde(rename_all = "snake_case")]` alongside —
/// consistency between key names and language files is the caller's
/// responsibility.
///
/// 中文:derive 宏,保留声明顺序实现 [`DictKey`](https://docs.rs/i18n-dict)
/// trait,判别值 = 声明顺序索引(与 full 词典数组下标一致)。无条件生成
/// `SORTED_KEYS` / `SORTED_VARIANTS`(字母序表,二分查找专用);变体数 > 16
/// 时自动生成 `find` 二分覆写(查找从 O(n) 降为 O(log n));小表(≤ 16)走
/// trait 默认线性实现(数量少时线性更快,与 CPU 缓存/流水线有关)。反序列化
/// 搭配 `#[derive(DictKeyDeserialize)]`,走 trait `find`(自动继承这里的二分
/// 策略),零额外表。与 `#[dictkey]` 二选一:本宏不改动变体顺序;如需字母序
/// 判别值请用 `#[dictkey(sort)]`。enum 需 `Copy`(trait 要求),请同时
/// `#[derive(Clone, Copy)]`。变体名**原样**作为键名字符串,不强制大小写;
/// 建议搭配 `#[serde(rename_all = "snake_case")]`,键名与语言文件的一致性
/// 由调用方保证。
#[proc_macro_derive(DictKey)]
pub fn derive_dict_key(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    // 只接受 enum;变体必须是无字段的 unit 变体(与语言文件键一一对应)
    let Data::Enum(data) = &input.data else {
        return syn::Error::new_spanned(name, "DictKey 只能用于 enum")
            .to_compile_error()
            .into();
    };

    // 生成的关联项同名会触发 E0592,提前给出友好报错
    const RESERVED: &[&str] = &[
        "COUNT",
        "VARIANTS",
        "KEYS",
        "SORTED_KEYS",
        "SORTED_VARIANTS",
    ];

    // (键名, 判别值):判别值 = 声明顺序索引
    let mut entries = Vec::new();
    // 变体 ident(声明顺序,值数组元素用)
    let mut variant_idents: Vec<&Ident> = Vec::new();
    for (i, variant) in data.variants.iter().enumerate() {
        if !matches!(variant.fields, Fields::Unit) {
            return syn::Error::new_spanned(variant, "DictKey 变体必须是无字段(unit)变体")
                .to_compile_error()
                .into();
        }
        let key = variant.ident.to_string();
        if RESERVED.contains(&key.as_str()) {
            return syn::Error::new_spanned(
                &variant.ident,
                format!(
                    "变体名 `{key}` 与 derive 生成的关联项同名,请改名; \
                     如需保留该键名,可加 #[serde(rename = \"{key}\")] 解耦"
                ),
            )
            .to_compile_error()
            .into();
        }
        entries.push((key, i));
        variant_idents.push(&variant.ident);
    }
    let len = entries.len();

    // SORTED_* 无条件生成(独立字母序表;find 二分覆写仅变体数大时,
    // 小表线性更快,直接用 trait 默认实现)
    let mut sorted = entries.clone();
    sorted.sort();
    let sorted_keys = sorted.iter().map(|(k, _)| proc_macro2::Literal::string(k));
    let sorted_variants = sorted.iter().map(|(_, i)| &data.variants[*i].ident);
    let sorted_consts = quote! {
        /// 键名数组(字母序,二分查找专用表)
        #[doc(hidden)]
        const SORTED_KEYS: &'static [&'static str] = &[#(#sorted_keys),*];
        /// 与 SORTED_KEYS 同序的变体值数组(字母序位置 → 变体,判别值 = 声明顺序索引)
        #[doc(hidden)]
        const SORTED_VARIANTS: &'static [Self] = &[#(Self::#sorted_variants),*];
    };

    let find_impl = if len > BINARY_SEARCH_THRESHOLD {
        quote! {
            fn find(name: &str) -> Option<Self> {
                Self::SORTED_KEYS
                    .binary_search(&name)
                    .ok()
                    .map(|i| Self::SORTED_VARIANTS[i])
            }
        }
    } else {
        proc_macro2::TokenStream::new()
    };

    // 键名数组(声明顺序)
    let keys = entries.iter().map(|(k, _)| proc_macro2::Literal::string(k));

    quote! {
        #[automatically_derived]
        impl ::i18n_dict::DictKey for #name {
            const COUNT: usize = #len;
            const VARIANTS: &'static [Self] = &[#(Self::#variant_idents),*];
            const KEYS: &'static [&'static str] = &[#(#keys),*];
            #sorted_consts
            #find_impl
        }
    }
    .into()
}

// ===========================================================================
// #[derive(DictKeyDeserialize)]
// ===========================================================================

/// Deserialization derive: use with `#[derive(DictKey)]` to generate a
/// `serde::Deserialize` impl for the enum.
///
/// Goes through the trait `find` (key name → variant), automatically
/// inheriting the `DictKey` lookup strategy: binary search for large
/// tables, linear for small ones — zero extra tables. Requires the user
/// to depend on serde directly; without this macro (or `deserialize` on
/// `#[dictkey]`) no serde code is generated and the crate compiles without
/// serde.
///
/// ```ignore
/// #[derive(DictKey, DictKeyDeserialize)]
/// enum Key { ... }
/// ```
///
/// Combinatorics: the standard pairing is with `#[derive(DictKey)]`; under
/// the `#[dictkey]` form it can only be added when the attribute does
/// **not** carry `deserialize` (functionally equivalent to the built-in
/// one); mixing `#[dictkey(deserialize)]` with this macro duplicates the
/// serde impl (compile error). Do not also `#[derive(serde::Deserialize)]`.
/// This macro does not check that `DictKey` is implemented — if it is not,
/// the expanded code referencing `find` fails to compile.
///
/// 中文:反序列化派生宏,与 `#[derive(DictKey)]` 搭配使用,为 enum 生成
/// `serde::Deserialize` 实现。统一走 trait `find`(键名 → 变体值),自动继承
/// `DictKey` 的查找策略:大表二分、小表线性,零附加表。需用户直接依赖
/// serde;不写本宏(或 `#[dictkey]` 不带 `deserialize` 参数)则不生成任何
/// serde 代码,不依赖 serde 也能编译。组合说明:与 `#[derive(DictKey)]`
/// 搭配是标准用法;`#[dictkey]` 形态下,仅当本宏**不带** `deserialize`
/// 参数时才可搭配本宏(功能等价于自带 `deserialize`);
/// `#[dictkey(deserialize)]` 与本宏混用会重复实现 serde(编译错误)。也不要
/// 再同时 `#[derive(serde::Deserialize)]`。本宏不检查 DictKey 是否已实现
/// ——未实现时展开代码引用 `find` 会编译报错。
#[proc_macro_derive(DictKeyDeserialize)]
pub fn derive_dict_key_deserialize(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    // 只接受 enum;变体必须是无字段的 unit 变体(与语言文件键一一对应)
    let Data::Enum(data) = &input.data else {
        return syn::Error::new_spanned(name, "DictKeyDeserialize 只能用于 enum")
            .to_compile_error()
            .into();
    };
    for variant in &data.variants {
        if !matches!(variant.fields, Fields::Unit) {
            return syn::Error::new_spanned(
                variant,
                "DictKeyDeserialize 变体必须是无字段(unit)变体",
            )
            .to_compile_error()
            .into();
        }
    }

    deserialize_impl(name).into()
}

// ===========================================================================
// subset!
// ===========================================================================

/// `subset!` 的输入:`dictEnum, SubsetName, key1, key2, ...`
/// (逗号分隔:dict enum 名、子集类型名、子集 key)。
/// key 可写路径 `dictEnum::key`(推荐,明确归属),也可写裸变体名
/// `key`(需 `use dictEnum::*` 后路径解析仍正确)。
struct SubsetInput {
    dict: Ident,
    name: Ident,
    keys: Vec<Path>,
}

impl Parse for SubsetInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let list = Punctuated::<Path, Token![,]>::parse_terminated(input)?;
        let mut it = list.into_iter();
        let dict = it.next().ok_or_else(|| input.error("缺少 dict enum 名"))?;
        let name = it.next().ok_or_else(|| input.error("缺少子集类型名"))?;
        let keys: Vec<Path> = it.collect();
        let dict = dict
            .get_ident()
            .cloned()
            .ok_or_else(|| syn::Error::new_spanned(&dict, "dict enum 名应为简单标识符"))?;
        let name = name
            .get_ident()
            .cloned()
            .ok_or_else(|| syn::Error::new_spanned(&name, "子集类型名应为简单标识符"))?;
        Ok(SubsetInput { dict, name, keys })
    }
}

/// Page-subset declaration macro: declares the keys a page uses, in one go:
/// - `{Name}`: an **enum** with one unit variant per key; discriminant =
///   position within the subset (`{Name}::{key} as usize` gives the
///   position for direct render addressing; reordering the declaration
///   remaps automatically — addressing code keeps its names, zero
///   misalignment risk)
/// - `{Name}::VARIANTS`: the subset variant array (fed to `dict.get_sub`;
///   order = declaration order)
///
/// Global indices need no generation: `{Dict}::{key} as usize` is the
/// discriminant — take it directly when indexing a custom container.
///
/// Only for enums that **implement the `DictKey` trait** (define the dict
/// enum with `#[dictkey]` or `#[derive(DictKey)]`); otherwise the
/// trait-bound assertion in the generated code makes rustc reject the
/// build.
///
/// Usage:
/// ```ignore
/// use i18n_dict::{dictkey, subset};
///
/// #[dictkey]
/// enum mydict { a, b, c }
///
/// subset!(mydict, mysub, mydict::c, mydict::b);
///
/// // position within the subset (discriminant = subset index, unrelated
/// // to the global index):
/// let pos = mysub::c as usize;  // 0
/// // subset variant array (dict.get_sub fetches by this; order =
/// // declaration order):
/// let keys = mysub::VARIANTS;   // [mydict::c, mydict::b]
/// // global index = dict discriminant, take it directly when needed:
/// let idx = mydict::c as usize; // 2
/// ```
///
/// 中文:页面子集声明宏,模块级声明页面用到的 key,一次生成 `{Name}` 枚举
/// (每 key 一个 unit 变体,判别值 = 子集内位置,`{Name}::{key} as usize`
/// 取位置,渲染直接寻址;调整声明顺序自动重映射,寻址处名字不变,零错位
/// 风险)与 `{Name}::VARIANTS` 子集变体数组(`dict.get_sub` 拉取用;顺序 =
/// 声明顺序)。全局下标无需生成:`{Dict}::{key} as usize` 即判别值,按下标
/// 访问自定义容器时直接取。仅限**已实现 `DictKey` trait** 的 enum(dict
/// enum 必须用 `#[dictkey]` 或 `#[derive(DictKey)]` 定义);未实现时生成
/// 代码中的 trait bound 断言会让 rustc 直接编译报错。
#[proc_macro]
pub fn subset(input: TokenStream) -> TokenStream {
    let SubsetInput { dict, name, keys } = parse_macro_input!(input as SubsetInput);
    let len = keys.len();

    // 变体名(路径最后一段;裸 ident 时即其本身)
    let key_idents: Vec<&Ident> = keys
        .iter()
        .map(|p| &p.segments.last().unwrap().ident)
        .collect();

    // 变体与生成的常量同名会触发 E0592,提前给出友好报错
    if let Some(key) = key_idents.iter().find(|k| k.to_string() == "VARIANTS") {
        return syn::Error::new_spanned(
            key,
            "子集 key `VARIANTS` 与生成的 VARIANTS 常量同名,请改名; \
             如需保留该键名,可加 #[serde(rename = \"VARIANTS\")] 解耦",
        )
        .to_compile_error()
        .into();
    }

    // 子集变体数组元素:dict enum 完整路径(dict::key)
    let key_paths = keys.iter().map(|path| quote! { #path });

    // 变体声明顺序 = 子集内位置 = 判别值,一次生成(无需递归辅助宏)
    quote! {
        #[allow(non_camel_case_types)]
        pub enum #name {
            #(#key_idents),*
        }
        impl #name {
            /// 子集变体数组(顺序 = 声明顺序,dict.get_sub 按此拉取)
            pub const VARIANTS: [#dict; #len] = [#(#key_paths),*];
            /// 编译期断言:#dict 已实现 DictKey trait(未实现则编译报错)
            #[doc(hidden)]
            const __DICTKEY_ASSERT: usize = <#dict as ::i18n_dict::DictKey>::COUNT;
        }
    }
    .into()
}
