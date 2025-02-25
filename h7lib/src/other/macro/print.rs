use core::any::type_name;

fn print_type_of<T>(_: &T) {
    rprintln!("{}", type_name::<T>());
}

fn stripped_type_name<T>() -> &'static str {
    let s = core::any::type_name::<T>();
    let p = s.split("::");
    p.last().unwrap()
}
