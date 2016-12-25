use std::usize;
use std::io::{Read, Result};

#[derive(Debug)]
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

    pub fn increment_read(&mut self, value:usize) -> &mut Self {
        self.read_position += value;
        self
    }

    pub fn as_write(&mut self) -> &mut [u8] {
        &mut self.value[self.write_position..]
    }

    pub fn increment_write(&mut self, value:usize) -> &mut Self {
        self.write_position += value;
        self
    }

    pub fn from<R>(&mut self, read:&mut R) -> Result<usize> where R: Read + Sized {
        let result = read.read(self.as_write());
        if let Ok(count) = result {
            self.increment_write(count);
        }
        result
    }
}

#[derive(Debug)]
pub struct Fragmented<'a> {
    data: Vec<&'a [u8]>,
    count: usize,
}

impl <'a> Fragmented<'a>{
    pub fn new(data:&'a [u8]) -> Fragmented<'a> {
        let halfway = data.len() / 2;
        Fragmented {
            data: vec!(&data[..halfway], &data[halfway..]),
            count: 0,
        }
    }

    pub fn count(&self) -> usize {
        self.count
    }
}

impl<'a> Read for Fragmented<'a> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        if self.data.len() == self.count {
            return Ok(0)
        }
        let fragment = self.data[self.count];
        let length = fragment.len();
        buf[..length].copy_from_slice(fragment);

        self.count += 1;
        Ok(length)
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
        }
        {
            buffer.increment_write(2);
            println!("{:?}", buffer);
        }
        {
            let mut write = buffer.as_write();
            write[0] = 3;
            write[1] = 4;
        }
        {
            buffer.increment_write(2);
            println!("{:?}", buffer);
        }
        {
            let read: &[u8] = buffer.as_read();
            assert_eq!(read.len(), 4);
            assert_eq!(read[0], 1);
            assert_eq!(read[1], 2);
            assert_eq!(read[2], 3);
            assert_eq!(read[3], 4);
        }
    }
}