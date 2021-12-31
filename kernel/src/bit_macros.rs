#[macro_export]
macro_rules! bit_setter {
    ($field:tt : $field_type:ty; $bit:literal, $vis:vis $name:ident) => {
        #[allow(dead_code)]
        $vis fn $name(&mut self, value: bool) {
            let b: $field_type = 1 << $bit;
            if (value) {
                self.$field |= b;
            } else {
                self.$field &= !b;
            }
        }
    }
}

#[macro_export]
macro_rules! bit_getter {
    ($field:tt : $field_type:ty; $bit:literal, $vis:vis $name:ident) => {
        #[allow(dead_code)]
        $vis fn $name(&self) -> bool {
            let b: $field_type = 1 << $bit;
            (self.$field & b) == b
        }
    }
}
