
/// A reference into an [crate::Arena](Arena) or a [crate::Store](Store).
#[derive(Debug)]
pub struct Ref<T: ?Sized> {
	pub(crate) id: usize,
	pub(crate) ptr: std::ptr::NonNull<T>,
}

impl<T: ?Sized> Ref<T> {
	pub fn new(id: usize, ptr: std::ptr::NonNull<T>) -> Self {
		Self {
			id, ptr
		}
	}

	/// Get a reference to the value, bypassing all checks.
	/// 
	/// Use with caution. For a safe variation, see: [Arena::get]
	/// 
	/// Safety:
	/// - the [Arena] this Ref belongs to has to be alive
	/// - there must not be any mutable reference to the same value
	pub unsafe fn get_unchecked(&self) -> &T {
		self.ptr.as_ref()
	}

	/// Get a mutable reference to the value, bypassing all checks.
	/// 
	/// Use with caution. For a safe variation, see: [Arena::get_mut]
	/// 
	/// Safety:
	/// - the [Arena] this Ref belongs to has to be alive
	/// - there must not be any mutable OR shared reference to the same value
	pub unsafe fn get_mut_unchecked(&mut self) -> &mut T {
		self.ptr.as_mut()
	}

	/// Get the id of the [Arena] this refers to.
	pub fn arena_id(&self) -> usize {
		self.id
	}

	/// Get the [std::ptr::NonNull] pointer this points to.
	pub fn as_ptr(&self) -> std::ptr::NonNull<T> {
		self.ptr
	}
}


impl<T: ?Sized> Clone for Ref<T> {
	fn clone(&self) -> Self {
		Ref { id: self.id, ptr: self.ptr }
	}
}

impl<T: ?Sized> Copy for Ref<T> {}

impl<T: ?Sized> PartialEq for Ref<T> {
	fn eq(&self, other: &Self) -> bool {
		self.id == other.id && self.ptr == other.ptr
	}
}

impl<T: ?Sized> Eq for Ref<T> {}

impl<T: ?Sized> std::hash::Hash for Ref<T> {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		state.write_usize(self.id);
		state.write_usize(self.ptr.as_ptr() as *mut () as usize);
	}
}

// TODO: make casting work like it does for pointers (*mut T as *mut dyn Trait) if possible
// impl<T: ?Sized, U: ?Sized> std::ops::CoerceUnsized<Ref<U>> for Ref<T> where T: std::marker::Unsize<U> {}
