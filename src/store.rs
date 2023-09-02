use crate::*;

use std::ptr::NonNull;


pub struct Store<T> {
	store_id: usize,
	values: Vec<T>,
}

impl<T> Store<T> {
	pub fn new() -> Self {
		Self { store_id: usize_counter(), values: Vec::new() }
	}

	pub fn with_capacity(capacity: usize) -> Self {
		Self { store_id: usize_counter(), values: Vec::with_capacity(capacity) }
	}

	pub fn alloc(&mut self, val: T) -> Ref<T> {
		self.values.push(val);

		Ref {
			id: self.store_id,
			/// Safety: we've just stored the value there, it is definitely valid
			ptr: unsafe { NonNull::new_unchecked(self.values.last_mut().unwrap()) }
		}
	}

	pub fn get(&self, r: Ref<T>) -> Option<&T> {
		if r.id != self.store_id {
			return None;
		}

		// SAFETY: as long as the id is equal, the memory pointed to has not been deallocated
		// and we're borrowing the store, so there cannot be any shared reference to T
		Some(unsafe { r.ptr.as_ref() })
	}

	pub fn get_mut(&mut self, mut r: Ref<T>) -> Option<&mut T> {
		if r.id != self.store_id {
			return None;
		}

		// SAFETY: as long as the id is equal, the memory pointed to has not been deallocated
		// and we're borrowing the store, so there cannot be any shared reference to T
		Some(unsafe { r.ptr.as_mut() })
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_store() {
		let mut store = Store::new();

		let a = store.alloc(12);
		let b = store.alloc(13);
		let c = store.alloc(14);

		assert_eq!(store.get(a), Some(&12));
		assert_eq!(store.get(b), Some(&13));
		assert_eq!(store.get(c), Some(&14));
	}
}
