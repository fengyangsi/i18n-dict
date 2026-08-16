use i18n_dict_macros::dictkey;

#[dictkey]
enum E {
    #[cfg(feature = "x")]
    A,
}

fn main() {}
