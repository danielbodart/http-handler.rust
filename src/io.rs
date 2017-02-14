use std::usize;
use std::io::{Read, BufRead, Write, Result, Error, ErrorKind};
use std::cmp::min;
use std::fmt::{Debug, Display};
use std::slice::from_raw_parts_mut;

pub trait ReadFrom {
    fn read_from<F>(&mut self, fun: F) -> Result<usize>
        where F: FnMut(&[u8]) -> Result<usize>;
}

pub trait WriteInto {
    fn write_into<F>(&mut self, fun: F) -> Result<usize>
        where F: FnMut(&mut [u8]) -> Result<usize>;
}

#[derive(Debug)]
pub struct Buffer<B> {
    value: B,
    pub read_position: usize,
    pub write_position: usize,
}

impl<B> Buffer<B> where B: AsRef<[u8]> {
    pub fn as_read(&self) -> &[u8] {
        &self.value.as_ref()[self.read_position..self.write_position]
    }

    pub fn increment_read(&mut self, value: usize) {
        self.read_position += value;
        if self.read_position == self.write_position {
            self.read_position = 0;
            self.write_position = 0;
        }
    }
}

impl<B> Buffer<B> where B: AsMut<[u8]> {
    pub fn as_write(&mut self) -> &mut [u8] {
        &mut self.value.as_mut()[self.write_position..]
    }

    pub fn increment_write(&mut self, value: usize) {
        self.write_position += value;
    }

    pub fn fill<R>(&mut self, read: &mut R) -> Result<usize>
        where R: Read + Sized {
        self.write_into(|slice| read.read(slice))
    }
}

impl<B> From<B> for Buffer<B> where B: AsRef<[u8]> {
    fn from(value: B) -> Self {
        Buffer {
            value: value,
            read_position: 0,
            write_position: 0,
        }
    }
}

impl Buffer<Vec<u8>> {
    pub fn with_capacity(capacity: usize) -> Buffer<Vec<u8>> {
        let mut value = Vec::with_capacity(capacity);
        unsafe { value.set_len(capacity) }
        Buffer::from(value)
    }
}

impl<B> Read for Buffer<B> where B: AsRef<[u8]> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        self.read_from(|slice| {
            let size = min(slice.len(), buf.len());
            buf[..size].copy_from_slice(&slice[..size]);
            Ok(size)
        })
    }
}

impl<B> Write for Buffer<B> where B: AsMut<[u8]> {
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

impl<B> ReadFrom for Buffer<B> where B: AsRef<[u8]> {
    fn read_from<F>(&mut self, mut fun: F) -> Result<usize>
        where F: FnMut(&[u8]) -> Result<usize> {
        let result = fun(self.as_read());
        if let Ok(count) = result {
            self.increment_read(count);
        }
        result
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

impl<B> WriteInto for Buffer<B> where B: AsMut<[u8]> {
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

/// Supports fragmented input unlike `std::io::BufReader` and is much simpler!
#[derive(Debug)]
pub struct BufferedRead<T, B> {
    pub inner: T,
    pub buffer: Buffer<B>,
}

impl<T> BufferedRead<T, Vec<u8>> where T: Read + Sized {
    pub fn new(inner: T) -> BufferedRead<T, Vec<u8>> {
        BufferedRead {
            inner: inner,
            buffer: Buffer::with_capacity(4096),
        }
    }
}

impl<T, B> BufferedRead<T, B> where T: Read + Sized, B: AsMut<[u8]> {
    pub fn fill(&mut self) -> Result<usize> {
        self.buffer.fill(&mut self.inner)
    }
}

impl<T, B> BufRead for BufferedRead<T, B> where T: Read + Sized, B: AsRef<[u8]> + AsMut<[u8]> {
    fn fill_buf(&mut self) -> Result<&[u8]> {
        self.fill()?;
        Ok(self.buffer.as_read())
    }

    fn consume(&mut self, amt: usize) {
        self.buffer.increment_read(amt);
    }
}

impl<T, B> Read for BufferedRead<T, B> where T: Read + Sized, B: AsRef<[u8]> + AsMut<[u8]> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        self.fill()?;
        self.buffer.read(buf)
    }
}

impl<T, B> ReadFrom for BufferedRead<T, B> where T: Read + Sized, B: AsRef<[u8]> + AsMut<[u8]> {
    fn read_from<F>(&mut self, fun: F) -> Result<usize>
        where F: FnMut(&[u8]) -> Result<usize> {
        self.fill()?;
        self.buffer.read_from(fun)
    }
}


pub trait SplitRead<'a> {
    type Output: SplitRead<'a>;

    fn split_read<F>(&'a mut self, fun: F) -> Result<usize>
        where F: FnMut(&[u8], Box<FnMut(usize) -> Self::Output + 'a>) -> Result<usize>;
}

impl<'a, B: 'a> SplitRead<'a> for Buffer<B> where B: AsRef<[u8]> + AsMut<[u8]> {
    type Output = Buffer<&'a mut [u8]>;

    fn split_read<F>(&'a mut self, mut fun: F) -> Result<usize>
        where F: FnMut(&[u8], Box<FnMut(usize) -> Self::Output + 'a>) -> Result<usize> {
        let result = {
            let data = self.value.as_mut();

            let ptr: *mut u8 = data.as_mut_ptr();
            let len = data.len();
            let read = self.read_position;
            let write = self.write_position;

            let slice = &data[read..write];

            fun(slice, Box::new(move |offset| {
                let remainder = unsafe { from_raw_parts_mut(ptr.offset((read + offset) as isize), len - offset) };
                Buffer { value: remainder, read_position: 0, write_position: write - offset }
            }))
        };
        if let Ok(count) = result {
            self.increment_read(count);
        }
        result
    }
}

// TODO: Work out a safe way to do this
impl<'a, T: 'a, B> SplitRead<'a> for BufferedRead<T, B> where T: Read + Sized, B: AsRef<[u8]> + AsMut<[u8]> {
    type Output = BufferedRead<&'a mut T, &'a mut [u8]>;

    fn split_read<F>(&'a mut self, mut fun: F) -> Result<usize>
        where F: FnMut(&[u8], Box<FnMut(usize) -> Self::Output + 'a>) -> Result<usize> {
        self.fill()?;
        let ptr: *mut T = &mut self.inner;

        self.buffer.split_read(|slice, mut splitter| {
            fun(slice, Box::new(move |offset| {
                BufferedRead {
                    inner: unsafe { &mut *ptr },
                    buffer: splitter(offset),
                }
            }))
        })
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

pub trait Streamer<'a> {
    type Item: 'a;

    fn next(&'a mut self) -> Option<Self::Item>;
}


#[cfg(test)]
mod tests {
    use super::*;

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
    fn split_read_with_buffer() {
        let mut buffer = Buffer::with_capacity(20);
        let mut data = &b"1234567890"[..];
        buffer.fill(&mut data).expect("peace");
        let read = buffer.split_read(|slice, mut splitter| {
            assert_eq!(slice, &b"1234567890"[..]);
            let mut read = 2;
            let mut remainder = splitter(read);

            read += remainder.split_read(|slice, _splitter| {
                assert_eq!(slice, &b"34567890"[..]);
                Ok(2)
            })?;

            read += remainder.split_read(|slice, _splitter| {
                assert_eq!(slice, &b"567890"[..]);
                Ok(2)
            })?;
            Ok(read)
        }).unwrap();
        assert_eq!(read, 6);
        buffer.split_read(|slice, mut _splitter| {
            assert_eq!(slice, &b"7890"[..]);
            Ok(2)
        }).unwrap();
    }

    #[test]
    fn split_read_with_buffered_read() {
        let data = &b"1234567890"[..];
        let mut reader = BufferedRead::new(Fragmented::new(data, 4));
        let read = reader.split_read(|slice, mut splitter| {
            assert_eq!(slice, &b"12"[..]);
            let mut read = 2;
            let mut remainder = splitter(read);

            read += remainder.split_read(|slice, _splitter| {
                assert_eq!(slice, &b"34"[..]);
                Ok(2)
            })?;

            read += remainder.split_read(|slice, _splitter| {
                assert_eq!(slice, &b"56"[..]);
                Ok(2)
            })?;
            Ok(read)
        }).unwrap();
        assert_eq!(read, 6);
        reader.split_read(|slice, mut _splitter| {
            assert_eq!(slice, &b"7890"[..]);
            Ok(2)
        }).unwrap();
    }
}