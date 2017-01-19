use std::usize;
use std::io::{Read, BufRead, Write, Result, Error, ErrorKind};
use std::cmp::min;
use std::fmt::{Debug, Display};

pub trait ReadFrom {
    fn read_from<F>(&mut self, fun: F) -> Result<usize>
        where F: FnMut(&[u8]) -> Result<usize>;
}

pub trait WriteInto {
    fn write_into<F>(&mut self, fun: F) -> Result<usize>
        where F: FnMut(&mut [u8]) -> Result<usize>;
}

#[derive(Debug)]
pub struct Buffer {
    value: Vec<u8>,
    pub read_position: usize,
    pub write_position: usize,
}

impl Buffer {
    pub fn with_capacity(capacity: usize) -> Buffer {
        let mut value = Vec::with_capacity(capacity);
        unsafe { value.set_len(capacity) }
        Buffer {
            value: value,
            read_position: 0,
            write_position: 0,
        }
    }

    pub fn capacity(&self) -> usize {
        self.value.capacity()
    }

    pub fn space(&self) -> usize {
        self.value.capacity() - self.write_position
    }

    pub fn reserve(&mut self, additional: usize) {
        println!("reserve space: {} additional: {} capacity: {}", self.space(), additional, self.capacity());
        if additional > self.space() {
            let actual = additional - self.space();
            self.value.reserve(actual);
        }
        let cap = { self.value.capacity() };
        unsafe { self.value.set_len(cap) }
    }

    pub fn len(&self) -> usize {
        self.write_position - self.read_position
    }

    pub fn as_read(&self) -> &[u8] {
        &self.value[self.read_position..self.write_position]
    }

    pub fn increment_read(&mut self, value: usize) {
        self.read_position += value;
        if self.read_position == self.write_position {
            self.read_position = 0;
            self.write_position = 0;
        }
    }

    pub fn as_write(&mut self) -> &mut [u8] {
        &mut self.value[self.write_position..]
    }

    pub fn increment_write(&mut self, value: usize) {
        self.write_position += value;
    }

    pub fn from<R>(&mut self, read: &mut R) -> Result<usize>
        where R: Read + Sized {
        self.write_into(|slice| read.read(slice))
    }
}

impl Read for Buffer {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        self.read_from(|slice| {
            let size = min(slice.len(), buf.len());
            buf[..size].copy_from_slice(&slice[..size]);
            Ok(size)
        })
    }
}

impl Write for Buffer {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.write_into(|slice| {
            let size = min(slice.len(), buf.len());
            slice[..size].copy_from_slice(&buf[..size]);
            Ok(size)
        })
    }

    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}

pub struct SimpleError;

impl SimpleError {
    pub fn debug<V>(value: V) -> Error where V: Debug {
        SimpleError::error(format!("{:?}", value))
    }

    pub fn display<V>(value: V) -> Error where V: Display {
        SimpleError::error(format!("{}", value))
    }

    pub fn error<V>(value: V) -> Error where V: AsRef<str> {
        Error::new(ErrorKind::Other, value.as_ref())
    }
}

#[allow(unused_variables)]
pub fn unit(result: Result<usize>) -> Result<()> {
    result.map(|ignore| ())
}

pub fn consume(result: Result<usize>) -> Result<()> {
    match result {
        Ok(value) if value > 0 => Ok(()),
        Ok(_) => Err(SimpleError::error("No data consumed")),
        Err(e) => Err(e),
    }
}

impl ReadFrom for Buffer {
    fn read_from<F>(&mut self, mut fun: F) -> Result<usize>
        where F: FnMut(&[u8]) -> Result<usize> {
        let result = fun(self.as_read());
        if let Ok(count) = result {
            self.increment_read(count);
        }
        result
    }
}

impl WriteInto for Buffer {
    fn write_into<F>(&mut self, mut fun: F) -> Result<usize>
        where F: FnMut(&mut [u8]) -> Result<usize> {
        let result = fun(self.as_write());
        if let Ok(count) = result {
            self.increment_write(count);
        }
        result
    }
}

impl ReadFrom for BufRead {
    fn read_from<F>(&mut self, mut fun: F) -> Result<usize>
        where F: FnMut(&[u8]) -> Result<usize> {
        let result = fun(self.fill_buf()?);
        if let Ok(count) = result {
            self.consume(count);
        }
        result
    }
}

/// Supports fragmented input unlike std::io::BufReader and is much simpler!
#[derive(Debug)]
pub struct BufferedRead<T> where T: Read + Sized {
    inner: T,
    buffer: Buffer,
}

impl<T> From<T> for BufferedRead<T> where T: Read + Sized {
    fn from(inner: T) -> Self {
        BufferedRead::with_capacity(4096, inner)
    }
}

impl<T> BufferedRead<T> where T: Read + Sized {
    pub fn new(inner: T, buffer: Buffer) -> BufferedRead<T> {
        BufferedRead {
            inner: inner,
            buffer: buffer,
        }
    }

    pub fn with_capacity(capacity: usize, inner: T) -> BufferedRead<T> {
        BufferedRead::new(inner, Buffer::with_capacity(capacity))
    }

    pub fn fill(&mut self) -> Result<usize> {
        self.buffer.from(&mut self.inner)
    }

    pub fn read_segment<F>(&mut self, mut fun: F) -> Result<usize>
        where F: FnMut(&[u8], &mut BufferedRead<&mut T>) -> Result<usize> {
        self.fill()?;
        match *self {
            BufferedRead { ref mut inner, ref mut buffer } => {
                println!("outer buffer before read: {:?}", buffer);
                let mut buffered = BufferedRead::with_capacity(buffer.capacity(), inner);
                let slice_read = buffer.read_from(|slice| {
                    fun(slice, &mut buffered)
                });
                println!("outer buffer after read: {:?}", buffer);
                println!("slice_read: {:?}", slice_read);
                let b = buffer;
                let buffered_read = match buffered {
                    BufferedRead { ref mut buffer, .. } => {
                        println!("inner buffer before from: {:?}", buffer);
                        b.reserve(buffer.len());
                        let x = b.from(buffer);
                        println!("inner buffer after from: {:?}", buffer);
                        x
                    }
                };
                println!("outer buffer after from: {:?}", b);
                println!("buffered_read: {:?}", buffered_read);
                Ok(slice_read? + buffered_read?)
            }
        }
    }
}

impl<T> BufRead for BufferedRead<T> where T: Read + Sized {
    fn fill_buf(&mut self) -> Result<&[u8]> {
        self.fill()?;
        Ok(self.buffer.as_read())
    }

    fn consume(&mut self, amt: usize) {
        self.buffer.increment_read(amt);
    }
}

impl<T> Read for BufferedRead<T> where T: Read + Sized {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        self.fill()?;
        self.buffer.read(buf)
    }
}

impl<T> ReadFrom for BufferedRead<T> where T: Read + Sized {
    fn read_from<F>(&mut self, fun: F) -> Result<usize>
        where F: FnMut(&[u8]) -> Result<usize> {
        self.fill()?;
        self.buffer.read_from(fun)
    }
}

#[derive(Debug)]
pub struct Fragmented<'a> {
    data: Vec<&'a [u8]>,
    count: usize,
}

impl<'a> Fragmented<'a> {
    pub fn new(data: &'a [u8], fragments: usize) -> Fragmented<'a> {
        let size = data.len() / fragments;
        let mut vec = Vec::with_capacity(size);
        for i in 0..fragments {
            let start = i * size;
            let end = (i + 1) * size;
            vec.push(&data[start..end]);
        }
        let last = fragments * size;
        vec.push(&data[last..]);
        Fragmented {
            data: vec,
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
    use std::error::Error;

    #[test]
    fn supports_capacity() {
        let buffer = Buffer::with_capacity(8);
        assert_eq!(buffer.capacity(), 8);
    }

    #[test]
    fn supports_length() {
        let mut buffer = Buffer::with_capacity(8);
        assert_eq!(buffer.len(), 0);
        { buffer.increment_write(5) };
        { buffer.increment_read(2) };
        assert_eq!(buffer.len(), 3);
    }

    #[test]
    fn supports_space() {
        let mut buffer = Buffer::with_capacity(8);
        assert_eq!(buffer.space(), 8);
        { buffer.increment_write(5) };
        assert_eq!(buffer.space(), 3);
    }

    #[test]
    fn supports_reserve() {
        let mut buffer = Buffer::with_capacity(8);
        { buffer.increment_write(5) };
        { buffer.reserve(11) };
        assert_eq!(buffer.capacity(), 16);
    }

    #[test]
    fn when_empty_there_will_be_nothing_to_read() {
        let buffer = Buffer::with_capacity(8);
        let slice: &[u8] = buffer.as_read();
        assert_eq!(slice.len(), 0);
    }

    #[test]
    fn when_empty_the_write_slice_will_have_the_full_capacity() {
        let mut buffer = Buffer::with_capacity(8);
        assert_eq!(buffer.as_write().len(), 8);
    }

    #[test]
    fn if_you_write_data_it_becomes_available_to_read() {
        let mut buffer = Buffer::with_capacity(8);
        buffer.write_into(|slice| {
            slice[0] = 1;
            slice[1] = 2;
            Ok(2)
        }).expect("Could not write_into");
        assert_eq!(buffer.read_position, 0);
        assert_eq!(buffer.write_position, 2);

        buffer.write_into(|slice| {
            slice[0] = 3;
            slice[1] = 4;
            Ok(2)
        }).expect("Could not write_into");
        assert_eq!(buffer.read_position, 0);
        assert_eq!(buffer.write_position, 4);

        buffer.read_from(|slice| {
            assert_eq!(slice.len(), 4);
            assert_eq!(slice[0], 1);
            assert_eq!(slice[1], 2);
            Ok(2)
        }).expect("Could not read_from");
        assert_eq!(buffer.read_position, 2);
        assert_eq!(buffer.write_position, 4);

        buffer.read_from(|slice| {
            assert_eq!(slice.len(), 2);
            assert_eq!(slice[0], 3);
            assert_eq!(slice[1], 4);
            Ok(2)
        }).expect("Could not read_from");
        assert_eq!(buffer.read_position, 0);
        assert_eq!(buffer.write_position, 0);
    }

    #[test]
    #[allow(unused_variables)]
    #[allow(unused_must_use)]
    fn buffered_read_can_nest() {
        let data = &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26][..];
        let mut buffered = BufferedRead::with_capacity(2, data);
        buffered.read_segment(|head, tail| {
            assert_eq!(head, &[1, 2][..]);
            tail.read_segment(|head, tail| {
                assert_eq!(head, &[3, 4][..]);
                Ok(1)
            })?;
            Ok(2)
        }).expect("No errors");
        buffered.read_segment(|head, tail| {
            assert_eq!(head, &[4, 5][..]);
            let failed = tail.read_segment(|head, tail| {
                assert_eq!(head, &[6, 7][..]);
                Err(SimpleError::error(""))
            });
            assert!(failed.is_err());
            tail.read_segment(|head, tail| {
                assert_eq!(head, &[6, 7][..]);
                Ok(1)
            })?;
            Ok(2)
        }).expect("No errors");
        buffered.read_segment(|head, tail| {
            assert_eq!(head, &[7, 8, 9, 10][..]);
            tail.read_segment(|head, tail| {
                assert_eq!(head, &[8, 9, 10, 11][..]);
                Ok(1)
            })?;
            tail.read_segment(|head, tail| {
                assert_eq!(head, &[9, 10, 11][..]);
                Ok(1)
            })?;
            tail.read_segment(|head, tail| {
                assert_eq!(head, &[10, 11][..]);
                Err(SimpleError::error(""))
            })?;
            panic!("Should never get here")
        }).expect("No errors");
        buffered.read_segment(|head, tail| {
            assert_eq!(head, &[4, 5, 6, 7][..]);
            Ok(0)
        }).expect("No errors");
    }
}