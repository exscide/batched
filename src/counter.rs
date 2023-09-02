
/// Return a value higher than the previous one
pub fn usize_counter() -> usize {
	static COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
	COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}
