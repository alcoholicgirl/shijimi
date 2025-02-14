pub fn to_bytes<T: Sized>(src: &T) -> &[u8] {
    unsafe { std::slice::from_raw_parts((src as *const T).cast(), std::mem::size_of::<T>()) }
}

pub fn vec_to_bytes<T: Sized>(src: &Vec<T>) -> &[u8] {
    unsafe { std::slice::from_raw_parts(src.as_ptr().cast(), std::mem::size_of::<T>() * src.len()) }
}
