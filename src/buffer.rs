use std::collections::VecDeque;

#[derive(Debug)]
pub(crate) struct ReorganizingBuffer<Value> {
	pub(crate) depth: usize,
	queue: VecDeque<(u64, Vec<Value>)>,
}

#[derive(Debug)]
pub(crate) enum ReorganizingBufferError {
	MissingOffset(u64),
	DepthExceeded(u64),
}

impl<Value> ReorganizingBuffer<Value> {
	pub(crate) fn new(depth: usize) -> ReorganizingBuffer<Value> {
		ReorganizingBuffer { depth, queue: VecDeque::with_capacity(depth + 1) }
	}

	pub(crate) fn push(
		&mut self,
		(new_offset, new_value): (u64, Vec<Value>),
	) -> Result<Option<(u64, Vec<Value>)>, ReorganizingBufferError> {
		if let Some((last_offset, _)) = self.queue.back() {
			// Ensure new item does not exceed reorganization depth limit
			let expected_offset = last_offset + 1;
			if new_offset > expected_offset {
				return Err(ReorganizingBufferError::MissingOffset(expected_offset));
			}

			// Perform reorganization, if necessary
			let reorg_depth = expected_offset - new_offset;
			if reorg_depth > self.depth.try_into().unwrap() {
				return Err(ReorganizingBufferError::DepthExceeded(reorg_depth));
			}
			for _ in 0..reorg_depth {
				self.queue.pop_back();
			}
		}

		// Update queue with new item
		self.queue.push_back((new_offset, new_value));

		// Return item that passed confirmation requirement
		if self.queue.len() > self.depth {
			Ok(self.queue.pop_front())
		} else {
			Ok(None)
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	mod push {
		use super::*;

		mod depth_0 {
			use super::*;

			#[test]
			fn passthrough() {
				let mut buffer = ReorganizingBuffer::<&str>::new(0);
				assert!(buffer.queue.is_empty());
				assert_eq!(
					buffer.push((123, vec!["abc", "def"])).unwrap(),
					Some((123, vec!["abc", "def"]))
				);
				assert!(buffer.queue.is_empty());
			}
		}

		mod depth_3 {
			use super::*;

			const DEPTH: usize = 3;

			mod ok {

				use super::*;

				#[test]
				fn reorg_none() {
					let item_1 = || (1, vec!["a"]);
					let item_2 = || (2, vec!["b"]);
					let item_3 = || (3, vec!["c"]);
					let item_4 = || (4, vec!["d"]);

					let mut buffer = ReorganizingBuffer::<&str>::new(DEPTH);
					assert_eq!(buffer.queue, vec![]);

					assert_eq!(buffer.push(item_1()).unwrap(), None);
					assert_eq!(buffer.queue, vec![item_1()]);

					assert_eq!(buffer.push(item_2()).unwrap(), None);
					assert_eq!(buffer.queue, vec![item_1(), item_2()]);

					assert_eq!(buffer.push(item_3()).unwrap(), None);
					assert_eq!(buffer.queue, vec![item_1(), item_2(), item_3()]);

					assert_eq!(buffer.push(item_4()).unwrap(), Some(item_1()));
					assert_eq!(buffer.queue, vec![item_2(), item_3(), item_4()]);
				}

				#[test]
				fn reorg_one() {
					let item_1 = || (1, vec!["a"]);
					let item_2 = || (2, vec!["b"]);
					let item_3 = || (3, vec!["c"]);
					let item_4 = || (4, vec!["d"]);

					let mut buffer = ReorganizingBuffer::<&str>::new(DEPTH);
					assert_eq!(buffer.queue, vec![]);

					assert_eq!(buffer.push(item_1()).unwrap(), None);
					assert_eq!(buffer.queue, vec![item_1()]);

					assert_eq!(buffer.push(item_2()).unwrap(), None);
					assert_eq!(buffer.queue, vec![item_1(), item_2()]);

					assert_eq!(buffer.push(item_3()).unwrap(), None);
					assert_eq!(buffer.queue, vec![item_1(), item_2(), item_3()]);

					assert_eq!(buffer.push(item_4()).unwrap(), Some(item_1()));
					assert_eq!(buffer.queue, vec![item_2(), item_3(), item_4()]);

					let item_4 = || (4, vec!["x"]);
					assert_eq!(buffer.push(item_4()).unwrap(), None);
					assert_eq!(buffer.queue, vec![item_2(), item_3(), item_4()]);
				}

				#[test]
				fn reorg_many() {
					let item_1 = || (1, vec!["a"]);
					let item_2 = || (2, vec!["b"]);
					let item_3 = || (3, vec!["c"]);
					let item_4 = || (4, vec!["d"]);

					let mut buffer = ReorganizingBuffer::<&str>::new(DEPTH);
					assert_eq!(buffer.queue, vec![]);

					assert_eq!(buffer.push(item_1()).unwrap(), None);
					assert_eq!(buffer.queue, vec![item_1()]);

					assert_eq!(buffer.push(item_2()).unwrap(), None);
					assert_eq!(buffer.queue, vec![item_1(), item_2()]);

					assert_eq!(buffer.push(item_3()).unwrap(), None);
					assert_eq!(buffer.queue, vec![item_1(), item_2(), item_3()]);

					assert_eq!(buffer.push(item_4()).unwrap(), Some(item_1()));
					assert_eq!(buffer.queue, vec![item_2(), item_3(), item_4()]);

					let item_3 = || (3, vec!["x"]);
					assert_eq!(buffer.push(item_3()).unwrap(), None);
					assert_eq!(buffer.queue, vec![item_2(), item_3()]);
				}

				#[test]
				fn reorg_max() {
					let item_2 = || (2, vec!["b"]);
					let item_3 = || (3, vec!["c"]);
					let item_4 = || (4, vec!["d"]);

					let buffer_new = || ReorganizingBuffer {
						depth: DEPTH,
						queue: VecDeque::from([item_2(), item_3(), item_4()]),
					};

					let mut buffer = buffer_new();
					let item_2 = || (2, vec!["x"]);
					assert_eq!(buffer.push(item_2()).unwrap(), None);
					assert_eq!(buffer.queue, vec![item_2()]);

					let mut buffer = buffer_new();
					let item_1 = || (1, vec!["x"]);
					assert!(buffer.push(item_1()).is_err());
				}
			}

			mod err {
				use super::*;

				mod depth_exceeded {
					use super::*;

					#[test]
					fn full_buffer() {
						let item_1 = || (1, vec!["a"]);
						let item_2 = || (2, vec!["b"]);
						let item_3 = || (3, vec!["c"]);
						let item_4 = || (4, vec!["d"]);

						let mut buffer = ReorganizingBuffer::<&str>::new(DEPTH);
						assert_eq!(buffer.queue, vec![]);

						assert_eq!(buffer.push(item_1()).unwrap(), None);
						assert_eq!(buffer.queue, vec![item_1()]);

						assert_eq!(buffer.push(item_2()).unwrap(), None);
						assert_eq!(buffer.queue, vec![item_1(), item_2()]);

						assert_eq!(buffer.push(item_3()).unwrap(), None);
						assert_eq!(buffer.queue, vec![item_1(), item_2(), item_3()]);

						assert_eq!(buffer.push(item_4()).unwrap(), Some(item_1()));
						assert_eq!(buffer.queue, vec![item_2(), item_3(), item_4()]);

						let item_1 = || (1, vec!["x"]);
						let result = buffer.push(item_1());

						match result {
							Err(ReorganizingBufferError::DepthExceeded(4)) => assert!(true),
							_ => assert!(false, "Unexpected result {:?}", result),
						}
						assert_eq!(buffer.queue, vec![item_2(), item_3(), item_4()]);
					}
				}
			}
		}
	}
}
