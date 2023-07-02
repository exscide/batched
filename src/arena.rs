//! A typeless, lifetimeless arena allocator that owns its values.
//! 
//! To use it, create a new [Arena] and allocate values using [Arena::alloc].
//! The returned [Ref]s can be accessed using [Arena::get] and [Arena::get_mut].


/// A typeless, lifetimeless arena allocator that owns its values.
/// 
/// Currently does not drop its values.
pub struct Arena<const BLOCK_SIZE: usize = 1024> {
	arena_id: usize,
	blocks: Vec<(std::alloc::Layout, *mut u8)>,
	cur_block: usize,
	offset: usize,
}

impl<const BLOCK_SIZE: usize> Arena<BLOCK_SIZE> {
	pub fn new() -> Self {
		let mut arena = Self::_new();

		arena.alloc_block();

		arena
	}

	fn _new() -> Self {
		static ARENA_IDX: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

		let arena = Self {
			arena_id: ARENA_IDX.load(std::sync::atomic::Ordering::Relaxed),
			blocks: Vec::new(),
			cur_block: 0,
			offset: 0,
		};

		ARENA_IDX.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

		arena
	}

	/// Create an [Arena] and allocate enough blocks to hold `n` bytes.
	pub fn with_capacity(n: usize) -> Self {
		let blocks = n / BLOCK_SIZE + if n % BLOCK_SIZE == 0 { 0 } else { 1 };

		Self::with_blocks(blocks)
	}

	/// Create an [Arena] and allocate `n` blocks
	pub fn with_blocks(n: usize) -> Self {
		let mut arena = Self::new();

		for _ in 0..n {
			arena.alloc_block();
		}

		arena
	}

	/// Allocate a block and push it to the block list.
	fn alloc_block(&mut self) {
		let layout = std::alloc::Layout::array::<u8>(BLOCK_SIZE).unwrap();

		// TODO: ensure safety
		let block = unsafe { std::alloc::alloc(layout) };

		if block.is_null() {
			panic!("Out of memory");
		}

		self.blocks.push((layout, block));
	}

	/// Move to the next block, allocate a new block if needed.
	fn next_block(&mut self) {
		self.cur_block += 1;

		if self.blocks.get(self.cur_block).is_none() {
			self.alloc_block();
		}

		self.offset = 0;
	}

	/// Ensure there is enough space within the current block for the given layout.
	/// If not, allocate a new one.
	fn make_space_for_layout(&mut self, layout: std::alloc::Layout) -> *mut u8 {
		let (cur_block, align_offset) = {
			// ensure space within the current block or allocate a new one

			let (_, cur_block) = self.blocks[self.cur_block];

			// calculate the alignment offset that would need to be applied to the current block
			// TODO: ensure safety
			let align_offset = unsafe { cur_block.add(self.offset) }.align_offset(layout.align());

			if self.offset + align_offset + layout.size() > BLOCK_SIZE {
				// not enough space within the current block

				self.next_block();

				let (_, cur_block) = self.blocks[self.cur_block];

				// calculate the alignment offset that would need to be applied to the new block
				// TODO: ensure safety
				let align_offset = unsafe { cur_block.add(self.offset) }.align_offset(layout.align());

				(cur_block, align_offset)
			} else {
				
				(cur_block, align_offset)
			}
		};

		// TODO: ensure safety
		let ptr = unsafe { cur_block.add(self.offset).add(align_offset) };

		// increase the current blocks offset for the next call to this function
		self.offset += align_offset + layout.size();

		ptr
	}

	/// Unsafely allocate space and memcpy val into the arena.
	unsafe fn alloc_memcpy<T>(&mut self, val: &T) -> Ref<T> {
		let layout = std::alloc::Layout::for_value(val);

		if layout.size() >= BLOCK_SIZE {
			// allocate a personal block for val if its type needs more space than blocks can provide

			let block = unsafe { std::alloc::alloc(layout) };

			// SAFETY: we've just allocated space
			unsafe { std::ptr::copy_nonoverlapping(val, block as *mut T, 1) };

			// insert the new block so that it is the second to last one
			self.blocks.insert(self.blocks.len()-1, (layout, block));

			self.cur_block += 1;

			return Ref::new(self.arena_id, std::ptr::NonNull::new(block as *mut T).unwrap());
		}

		let ptr = self.make_space_for_layout(layout) as *mut T;

		// SAFETY:
		// - there's enough space in the block for the type
		// - the pointer is aligned
		unsafe { std::ptr::copy_nonoverlapping(val, ptr, 1) };

		// SAFETY: the pointer is ensured not to be null when allocating the block in [alloc_block]
		Ref::new(self.arena_id, unsafe { std::ptr::NonNull::new_unchecked(ptr) })
	}

	/// Allocate a [str] within the Arena and return a [Ref] to it.
	pub fn alloc_str(&mut self, val: &str) -> Ref<str> {
		let val = val.as_bytes();

		let layout = std::alloc::Layout::for_value(val);

		if layout.size() >= BLOCK_SIZE {
			// allocate a personal block for val if its type needs more space than blocks can provide

			let block = unsafe { std::alloc::alloc(layout) };

			// SAFETY: we've just allocated space
			unsafe { std::ptr::copy_nonoverlapping(val.as_ptr(), block, val.len()) };

			// insert the new block so that it is the second to last one
			self.blocks.insert(self.blocks.len()-1, (layout, block));

			self.cur_block += 1;

			// SAFETY: we're just casting the previously copied val from the arena back to its original form
			let s = unsafe { std::str::from_utf8_unchecked_mut(std::slice::from_raw_parts_mut(block, val.len())) };

			return Ref::new(self.arena_id, std::ptr::NonNull::new(s as *mut str).unwrap());
		}

		let ptr = self.make_space_for_layout(layout) as *mut u8;

		// SAFETY:
		// - there's enough space in the block for the type
		// - the pointer is aligned
		unsafe { std::ptr::copy_nonoverlapping(val.as_ptr(), ptr, val.len()) };

		// SAFETY: we're just casting the previously copied val from the arena back to its original form
		let s = unsafe { std::str::from_utf8_unchecked_mut(std::slice::from_raw_parts_mut(ptr, val.len())) };

		// SAFETY: the pointer is ensured not to be null when allocating the block in [alloc_block]
		Ref::new(self.arena_id, unsafe { std::ptr::NonNull::new_unchecked(s as *mut str) })
	}

	/// Allocate a new value within the Arena and return a [Ref] to it.
	/// 
	/// When the current block is full and there is no free block left,
	/// a new one will be allocated.
	pub fn alloc<T: 'static>(&mut self, val: T) -> Ref<T> {

		// SAFETY:
		// - we're owning val
		// - val cannot contain any non-static references
		// - we're not dropping val since it's been moved to the arena
		let r = unsafe { self.alloc_memcpy(&val) };

		std::mem::forget(val);

		r
	}

	/// Get a reference to a value within the Arena.
	/// 
	/// Returns [None] when the value is invalid (Arena has been cleared, does not belong to this Arena).
	pub fn get<T: ?Sized>(&self, r: Ref<T>) -> Option<&T> {
		if r.arena_id != self.arena_id {
			return None;
		}

		// SAFETY: as long as the arena_id is equal, the memory pointed to has not been deallocated
		Some(unsafe { r.ptr.as_ref() })
	}

	/// Get a mutable reference to a value within the Arena.
	/// 
	/// Returns [None] when the value is invalid (Arena has been cleared, does not belong to this Arena).
	pub fn get_mut<T: ?Sized>(&mut self, mut r: Ref<T>) -> Option<&mut T> {
		if r.arena_id != self.arena_id {
			return None;
		}

		// SAFETY: as long as the arena_id is equal, the memory pointed to has not been deallocated
		Some(unsafe { r.ptr.as_mut() })
	}

	/// Clear the arena, leaving the blocks allocated.
	pub fn clear(&mut self) {
		self.cur_block = 0;
		self.offset = 0;
	}
}

impl<const BLOCK_SIZE: usize> Drop for Arena<BLOCK_SIZE> {
	fn drop(&mut self) {
		// TODO: implement dropping of values?

		for block in &self.blocks {
			// SAFETY: as long as self is alive, the memory pointed to has not been deallocated
			unsafe { std::alloc::dealloc(block.1, block.0) };
		}
	}
}


/// A reference into an [Arena].
#[derive(Debug)]
pub struct Ref<T: ?Sized> {
	arena_id: usize,
	ptr: std::ptr::NonNull<T>,
}

impl<T: ?Sized> Ref<T> {
	pub fn new(arena_id: usize, ptr: std::ptr::NonNull<T>) -> Self {
		Self {
			arena_id, ptr
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
		self.arena_id
	}

	/// Get the [std::ptr::NonNull] pointer this points to.
	pub fn as_ptr(&self) -> std::ptr::NonNull<T> {
		self.ptr
	}
}


impl<T: ?Sized> Clone for Ref<T> {
	fn clone(&self) -> Self {
		Ref { arena_id: self.arena_id, ptr: self.ptr }
	}
}

impl<T: ?Sized> Copy for Ref<T> {}

impl<T: ?Sized> PartialEq for Ref<T> {
	fn eq(&self, other: &Self) -> bool {
		self.arena_id == other.arena_id && self.ptr == other.ptr
	}
}

impl<T: ?Sized> Eq for Ref<T> {}

impl<T: ?Sized> std::hash::Hash for Ref<T> {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		state.write_usize(self.arena_id);
		state.write_usize(self.ptr.as_ptr() as *mut () as usize);
	}
}

// TODO: make casting work like it does for pointers (*mut T as *mut dyn Trait) if possible
// impl<T: ?Sized, U: ?Sized> std::ops::CoerceUnsized<Ref<U>> for Ref<T> where T: std::marker::Unsize<U> {}


#[cfg(test)]
mod test {
	use super::*;

	fn mutate_arena(arena: &mut Arena<32>) {
		// some allocation padding and block overflow checks

		let r1 = arena.alloc(1234u64); // u64 = 8 bytes
		assert_eq!(arena.get(r1), Some(&1234));
		assert_eq!(arena.offset, 8);

		let r2 = arena.alloc(4321u32); // u32 = 4 bytes
		assert_eq!(arena.get(r2), Some(&4321));
		assert_eq!(arena.offset, 8 + 4);

		let r3 = arena.alloc(1010u64); // u64 = 8 bytes
		assert_eq!(arena.get(r3), Some(&1010));
		assert_eq!(arena.offset, 8 + 4 + /*padding*/ 4 + 8); // alignment adds padding of 4 bytes

		let r4 = arena.alloc(u64::MAX); // u64 = 8 bytes
		assert_eq!(arena.get(r4), Some(&u64::MAX));
		assert_eq!(arena.offset, 32);

		assert_eq!(arena.cur_block, 0); // still no new block allocated

		let r5 = arena.alloc(u64::MIN);
		assert_eq!(arena.get(r5), Some(&u64::MIN));
		// arena was full, new block was allocated
		assert_eq!(arena.offset, 8);
		assert_eq!(arena.cur_block, 1);
	}

	#[test]
	fn test_arena() {
		let mut arena = Arena::<32>::new();

		mutate_arena(&mut arena);
		arena.clear();
		// arena should behave equally clearing
		mutate_arena(&mut arena);


		let old = arena.alloc(555);
		assert_eq!(arena.get(old), Some(&555));

		drop(arena);

		let mut arena2 = Arena::<0>::new();

		// wrong arena
		assert_eq!(arena2.get(old), None);


		// a zero-sized arena would allocate a new block for every alloc call
		// (this should work, but it wouldn't make sense to use it that way)
		let x = arena2.alloc(123123);
		assert_eq!(arena2.get(x), Some(&123123));
		assert_eq!(arena2.offset, 0);


		test_auto_traits::<Arena>();
	}

	// TODO: make it Send + Sync ?
	fn test_auto_traits<T: Unpin>() {}


	#[test]
	fn test_arena_str() {
		let mut arena = Arena::<16>::new();

		let r = arena.alloc_str("yote");
		assert_eq!(arena.get(r), Some("yote"));
		assert_eq!(arena.offset, 4);

		let x = arena.alloc_str("yöte");
		assert_eq!(arena.get(x), Some("yöte"));
		assert_eq!(arena.offset, 9);


		let a = arena.alloc_str("123456");
		assert_eq!(arena.get(a), Some("123456"));
		assert_eq!(arena.offset, 15);

		let b = arena.alloc_str("1234");
		assert_eq!(arena.get(b), Some("1234"));
		assert_eq!(arena.offset, 4);

		let c = arena.alloc_str("3");
		let d = arena.alloc_str("4");
		assert_eq!(arena.get(c), Some("3"));
		assert_eq!(arena.get(d), Some("4"));
		assert_eq!(arena.offset, 6);


	}
}
