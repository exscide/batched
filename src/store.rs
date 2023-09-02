use crate::*;


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

	pub fn alloc(&mut self, val: T) -> Handle<T> {
		self.values.push(val);

		Handle {
			store_id: self.store_id,
			idx: self.values.len() - 1,
			_marker: std::marker::PhantomData,
		}
	}

	pub fn get(&self, r: Handle<T>) -> Option<&T> {
		if r.store_id != self.store_id {
			return None;
		}

		Some(&self.values[r.idx])
	}

	pub fn get_mut(&mut self, r: Handle<T>) -> Option<&mut T> {
		if r.store_id != self.store_id {
			return None;
		}

		Some(&mut self.values[r.idx])
	}
}


pub struct Handle<T> {
	store_id: usize,
	idx: usize,
	_marker: std::marker::PhantomData<T>,
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
