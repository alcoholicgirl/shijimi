pub fn cast_bytes<T: Sized>(src: &T) -> &[u8] {
    unsafe { std::slice::from_raw_parts((src as *const T).cast(), std::mem::size_of::<T>()) }
}

pub fn cast_bytes_vec<T: Sized>(src: &Vec<T>) -> &[u8] {
    unsafe { std::slice::from_raw_parts(src.as_ptr().cast(), std::mem::size_of::<T>() * src.len()) }
}
