#[macro_export]
macro_rules! bit_getter {
    ($base:tt $([$idx:literal])? : $base_ty:ty ; $mask:expr ; $ty:ty, $vis:vis $getter_name:ident) => {
        #[allow(dead_code)]
        $vis fn $getter_name(&self) -> $ty {
            (((self.$base $([$idx])?) & $mask) >> <$base_ty>::trailing_zeros($mask)) as $ty
        }
    };
}

#[macro_export]
macro_rules! bit_setter {
    ($base:tt $([$idx:literal])? : $base_ty:ty ; $mask:expr ; $ty:ty, $vis:vis $setter_name:ident) => {
        #[allow(dead_code)]
        $vis fn $setter_name(&mut self, val: $ty) {
            self.$base $([$idx])? = self.$base $([$idx])? & !$mask | ((val as $base_ty) << <$base_ty>::trailing_zeros($mask));
        }
    };
}

#[macro_export]
macro_rules! container_of {
    ($ptr: expr, $container: path, $field: ident) => {
        unsafe {
            let inner = $ptr as *const _;
            let outer = &(*(0 as *const $container)).$field as *const _;
            &*((inner as usize - outer as usize) as *const $container)
        }
    };
    ($ptr: expr, mutable $container: path, $field: ident) => {
        unsafe {
            let inner = $ptr as *const _;
            let outer = &(*(0 as *const $container)).$field as *const _;
            &mut *((inner as usize - outer as usize) as *mut $container)
        }
    };
}
