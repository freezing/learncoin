pub struct FlipBuffer {
    buffer: Vec<u8>,
    start_index: usize,
    end_index: usize,
}

impl FlipBuffer {
    pub fn new(capacity: usize) -> Self {
        let mut buffer = Vec::<u8>::new();
        buffer.resize(capacity, 0);

        Self {
            buffer,
            start_index: 0,
            end_index: 0,
        }
    }

    /// Moves the data to the beginning of the buffer, which frees the consumed space.
    pub fn flip(&mut self) {
        self.buffer.copy_within(self.start_index..self.end_index, 0);
        self.end_index -= self.start_index;
        self.start_index = 0;
    }

    /// Returns the mutable slice from the underlying buffer that comes immediately after
    /// all unconsumed data.
    /// When the data is written to the slice, the buffer must be notified via `consume_free_space`
    /// method.
    pub fn free_space_slice_mut(&mut self) -> &mut [u8] {
        &mut self.buffer[self.end_index..]
    }

    /// Returns the number of bytes that can be written to the buffer.
    pub fn free_space_size(&self) -> usize {
        self.buffer.capacity() - self.data().len()
    }

    /// Returns the slice of the unconsumed data.
    pub fn data(&self) -> &[u8] {
        &self.buffer[self.start_index..self.end_index]
    }

    /// Consumes the given number of bytes from the data.
    /// Note that the consumed bytes are not freed and can't be used unless the flip() method is
    /// called.
    ///
    /// Preconditions:
    ///   - Number of consumed bytes must be <= size of the data().
    pub fn consume_data(&mut self, num_bytes: usize) {
        self.start_index += num_bytes;
        assert!(self.start_index <= self.end_index);
    }

    /// Moves part of the free space into the unconsumed data.
    pub fn consume_free_space(&mut self, num_bytes: usize) {
        self.end_index += num_bytes
    }
}
