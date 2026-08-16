use i18n_dict_macros::DictKeyDeserialize;

#[derive(DictKeyDeserialize)]
enum E {
    A(i32),
}

fn main() {}
