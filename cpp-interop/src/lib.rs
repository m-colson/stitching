pub mod container;

#[cfg(target_os = "windows")]
#[repr(C)]
#[derive(PartialEq, Eq)]
pub struct DestructorVEntry<T> {
    vec_delete: unsafe extern "C" fn(this: *mut T, n: core::ffi::c_uint),
}

#[cfg(target_os = "linux")]
#[repr(C)]
#[derive(PartialEq, Eq)]
pub struct DestructorVEntry<T> {
    complete_destroy: unsafe extern "C" fn(this: *mut T),
    delete_destroy: unsafe extern "C" fn(this: *mut T),
}

impl<T> Clone for DestructorVEntry<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> Copy for DestructorVEntry<T> {}

#[cfg(target_os = "windows")]
impl<T> DestructorVEntry<T> {
    pub const fn new() -> Self {
        extern "C" fn vec_delete<T>(_this: *mut T, n: core::ffi::c_uint) {
            todo!(
                "vector delete destructor ({n}) for type {}",
                std::any::type_name::<T>()
            )
        }
        Self { vec_delete }
    }

    /// # Safety
    /// `this` must be a valid pointer created with C++ new
    pub unsafe fn delete(self, this: *mut T) {
        (self.vec_delete)(this, 1);
    }
}

#[cfg(target_os = "linux")]
impl<T> DestructorVEntry<T> {
    pub const fn new() -> Self {
        extern "C" fn complete_destroy<T>(_this: *mut T) {
            println!(
                "complete destructor for type {}",
                std::any::type_name::<T>()
            );
            std::process::exit(2);
        }
        extern "C" fn delete_destroy<T>(_this: *mut T) {
            println!("delete destructor for type {}", std::any::type_name::<T>());
            std::process::exit(2);
        }
        Self {
            complete_destroy,
            delete_destroy,
        }
    }

    /// # Safety
    /// `this` must be a valid pointer created with C++ new
    pub unsafe fn delete(self, this: *mut T) {
        (self.delete_destroy)(this);
    }
}

impl<T> Default for DestructorVEntry<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[macro_export]
macro_rules! make_vtable {
    ($t:ident for $dt:ty { $($fields:tt)* }) => {
        $crate::make_vtable!(& $t for $dt => () () $($fields)*);
    };

    (& $t:ident for $dt:ty => ($($acc:tt)*) ($($impl_acc:tt)*) fn $fname:ident ($($bname:ident : $bty:ty),*) $(-> $ret:ty)?;$($rest:tt)*) => {
        $crate::make_vtable!(& $t for $dt =>
            ($($acc)*
                pub $fname: unsafe extern "C" fn(this: *mut $dt, $($bname : $bty),*) $(-> $ret)?,
            )
            ($($impl_acc)*
                pub unsafe fn $fname(&mut self, $($bname: $bty),*) $(-> $ret)? {
                    ((*(self.vtable_ as *const $t)).$fname)(self, $($bname),*)
                }
            )
            $($rest)*
        );
    };

    (& $t:ident for $dt:ty => ($($acc:tt)*) ($($impl_acc:tt)*) const fn $fname:ident ($($bname:ident : $bty:ty),*) $(-> $ret:ty)?;$($rest:tt)*) => {
        $crate::make_vtable!(& $t for $dt =>
            ($($acc)*
                pub $fname: unsafe extern "C" fn(this: *const $dt, $($bname : $bty),*) $(-> $ret)?,
            )
            ($($impl_acc)*
                pub unsafe fn $fname(&self, $($bname: $bty),*) $(-> $ret)? {
                    ((*(self.vtable_ as *const $t)).$fname)(self, $($bname),*)
                }
            )
            $($rest)*
        );
    };

    (& $t:ident for $dt:ty => ($($acc:tt)*) ($($impl_acc:tt)*) destructor;$($rest:tt)*) => {
        $crate::make_vtable!(& $t for $dt =>
            ($($acc)*
                pub destruct: $crate::DestructorVEntry<$dt>,)
            ($($impl_acc)*)
            $($rest)*
        );
    };

    (& $t:ident for $dt:ty => ($($acc:tt)*) ($($impl_acc:tt)*)) => {
        #[repr(C)]
        pub struct $t {
            $($acc)*
        }

        impl $dt {
            $($impl_acc)*
        }
    };
}
