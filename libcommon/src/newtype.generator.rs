const SIGNED: bool = true;
const UNSIGNED: bool = false;
const PTR_SIZES: [Option<u8>; 4] = [ None, Some(16), Some(32), Some(64) ];

#[derive(Clone, Copy)]
struct Ty {
    name: &'static str,
    signed: bool,
    size: u8,
    ptr_size: Option<u8>
}

fn main() {
    let types = [
        Ty { name: "i8", signed: SIGNED, size: 8, ptr_size: None },
        Ty { name: "u8", signed: UNSIGNED, size: 8, ptr_size: None },
        Ty { name: "i16", signed: SIGNED, size: 16, ptr_size: None },
        Ty { name: "u16", signed: UNSIGNED, size: 16, ptr_size: None },
        Ty { name: "i32", signed: SIGNED, size: 32, ptr_size: None },
        Ty { name: "u32", signed: UNSIGNED, size: 32, ptr_size: None },
        Ty { name: "i64", signed: SIGNED, size: 64, ptr_size: None },
        Ty { name: "u64", signed: UNSIGNED, size: 64, ptr_size: None },
        Ty { name: "i128", signed: SIGNED, size: 128, ptr_size: None },
        Ty { name: "u128", signed: UNSIGNED, size: 128, ptr_size: None },
        Ty { name: "isize", signed: SIGNED, size: 16, ptr_size: Some(16) },
        Ty { name: "usize", signed: UNSIGNED, size: 16, ptr_size: Some(16) },
        Ty { name: "isize", signed: SIGNED, size: 32, ptr_size: Some(32) },
        Ty { name: "usize", signed: UNSIGNED, size: 32, ptr_size: Some(32) },
        Ty { name: "isize", signed: SIGNED, size: 64, ptr_size: Some(64) },
        Ty { name: "usize", signed: UNSIGNED, size: 64, ptr_size: Some(64) },
    ];

    for ty in types.iter().filter(|ty| ty.ptr_size.is_none()) {
        let mut infallible_from = Vec::new();
        let mut infallible_to = Vec::new();
        let mut fallible_from = Vec::new();
        let mut fallible_to = Vec::new();

        for other in types.iter() {
            match (ty.signed, other.signed) {
                (SIGNED, SIGNED) | (UNSIGNED, UNSIGNED) => {
                    if ty.size == other.size {
                        infallible_from.push(other);
                        infallible_to.push(other);
                    } else if ty.size > other.size {
                        infallible_from.push(other);
                        fallible_to.push(other);
                    } else if ty.size < other.size {
                        fallible_from.push(other);
                        infallible_to.push(other);
                    } else {
                        panic!("the laws of mathematics no longer apply");
                    }
                },
                (SIGNED, UNSIGNED) => {
                    if ty.size > other.size {
                        infallible_from.push(other);
                    } else {
                        fallible_from.push(other);
                    }

                    fallible_to.push(other);
                },
                (UNSIGNED, SIGNED) => {
                    fallible_from.push(other);

                    if ty.size < other.size {
                        infallible_to.push(other);
                    } else {
                        fallible_to.push(other);
                    }
                },
            }
        }

        println!("    (@impl $ident:ident, {}) => {{", ty.name);

        for ptr_size in &PTR_SIZES {
            if has_tys(&infallible_from, ptr_size) {
                print_cfg(ptr_size);
                println!(
                    "        $crate::newtype_num!(@from $ident, {});",
                    get_tys(&infallible_from, ptr_size)
                );
            }
            if has_tys(&infallible_to, ptr_size) {
                print_cfg(ptr_size);
                println!(
                    "        $crate::newtype_num!(@into $ident, {});",
                    get_tys(&infallible_to, ptr_size)
                );
            }
            if has_tys(&fallible_from, ptr_size) {
                print_cfg(ptr_size);
                println!(
                    "        $crate::newtype_num!(@try_from $ident, {}, {});",
                    ty.name,
                    get_tys(&fallible_from, ptr_size)
                );
            }
            if has_tys(&fallible_to, ptr_size) {
                print_cfg(ptr_size);
                println!(
                    "        $crate::newtype_num!(@try_into $ident, {}, {});",
                    ty.name,
                    get_tys(&fallible_to, ptr_size)
                );
            }
        }

        println!("    }};");
    }
}

fn print_cfg(ptr_size: &Option<u8>) {
    match ptr_size {
        None => {},
        Some(size) => println!("        #[cfg(target_pointer_width = \"{}\")]", size)
    }
}

fn has_tys(types: &Vec<&Ty>, ptr_size: &Option<u8>) -> bool {
    types.iter().any(|ty| ty.ptr_size == *ptr_size)
}

fn get_tys(types: &Vec<&Ty>, ptr_size: &Option<u8>) -> String {
    let mut tys = types.iter()
        .filter(|ty| ty.ptr_size == *ptr_size)
        .map(|ty| ty.name)
        .collect::<Vec<_>>();
    if ptr_size.is_some() {
        tys.sort();
        tys.dedup();
    }
    tys.join(" ")
}
