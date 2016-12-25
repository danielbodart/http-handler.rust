use std::usize;

pub struct Buffer{
    value: Vec<u8>,
    read_position: usize,
    write_position: usize,
}

impl Buffer {
    pub fn new(capacity:usize) -> Buffer {
        let mut value = Vec::with_capacity(capacity);
        unsafe { value.set_len(capacity) }
        Buffer{
            value: value,
            read_position: 0,
            write_position: 0,
        }
    }

    pub fn capacity(&self) -> usize {
        self.value.capacity()
    }

    pub fn as_read(&self) -> &[u8] {
        &self.value[self.read_position..self.write_position]
    }

    pub fn as_write(&mut self) -> &mut [u8] {
        &mut self.value[self.write_position..]
    }

    pub fn read_position(&mut self, value:usize) -> &mut Self {
        self.read_position = value;
        self
    }

    pub fn write_position(&mut self, value:usize) -> &mut Self {
        self.write_position = value;
        self
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supports_capacity() {
        let buffer = Buffer::new(8);
        assert_eq!(buffer.capacity(), 8);
    }

    #[test]
    fn when_empty_there_will_be_nothing_to_read() {
        let buffer = Buffer::new(8);
        let slice:&[u8] = buffer.as_read();
        assert_eq!(slice.len(), 0);
    }

    #[test]
    fn when_empty_the_write_slice_will_have_the_full_capacity() {
        let mut buffer = Buffer::new(8);
        assert_eq!(buffer.as_write().len(), 8);
    }

    #[test]
    fn if_you_write_data_it_becomes_available_to_read() {
        let mut buffer = Buffer::new(8);
        {
            let mut write = buffer.as_write();
            write[0] = 1;
            write[1] = 2;
            write[2] = 3;
        }
        buffer.write_position(3);
        {
            let read: &[u8] = buffer.as_read();
            assert_eq!(read.len(), 3);
            assert_eq!(read[0], 1);
            assert_eq!(read[1], 2);
            assert_eq!(read[2], 3);
        }
    }
}