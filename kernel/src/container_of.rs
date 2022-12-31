#[macro_export]
#[allow(deref_nullptr)]
macro_rules! container_of{
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