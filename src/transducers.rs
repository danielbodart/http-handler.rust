#[derive(PartialEq, Debug)]
pub enum State {
    Continue,
    Stop
}

pub trait Receiver<I, E> {
    fn start(&mut self) -> State;
    fn next(&mut self, item: Result<I, E>) -> State;
    fn finish(&mut self);

    fn map<O, F>(self, mapper: F) -> MapReceiver<Self, F>
        where Self: Sized,
              F: Fn(Result<I, E>) -> Result<O, E> {
        MapReceiver::new(self, mapper)
    }
}

pub trait Sender<I, E, R> where R: Receiver<I, E> {
    fn send(&mut self, receiver: R);
}

pub struct MapReceiver<R, M> {
    receiver: R,
    mapper: M,
}

impl<R, M> MapReceiver<R, M> {
    pub fn new(receiver: R, mapper: M) -> MapReceiver<R, M> {
        MapReceiver {
            receiver: receiver,
            mapper: mapper,
        }
    }
}

impl<I, O, E, F, R> Receiver<O, E> for MapReceiver<R, F>
where F: Fn(Result<O, E>) -> Result<I, E>,
      R: Receiver<I, E> {
    fn start(&mut self) -> State {
        self.receiver.start()
    }

    fn next(&mut self, item: Result<O, E>) -> State {
        self.receiver.next((self.mapper)(item))
    }

    fn finish(&mut self) {
        self.receiver.finish()
    }
}

pub trait Transducee<I, O, E, R> where R: Receiver<O, E> {
    type Result: Receiver<I, E>;

    fn apply(self, receiver: R) -> Self::Result;

    fn identity() -> () {
        ()
    }

    fn map<F>(self, mapper: F) -> MapTransducee<F>
        where Self: Sized,
              F: Fn(Result<I, E>) -> Result<O, E> {
        MapTransducee { mapper: mapper }
    }
}

impl<I, E, R> Transducee<I, I, E, R> for ()
where R: Receiver<I, E> {
    type Result = R;

    fn apply(self, receiver: R) -> Self::Result {
        receiver
    }
}

pub struct MapTransducee<M> {
    mapper: M,
}

impl<I, O, E, F, R> Transducee<I, O, E, R> for MapTransducee<F>
where F: Fn(Result<I, E>) -> Result<O, E>,
      R: Receiver<O, E> {
    type Result = MapReceiver<R, F>;

    fn apply(self, receiver: R) -> Self::Result {
        MapReceiver::new(receiver, self.mapper)
    }
}


impl<I, E, R, Iter> Sender<I, E, R> for Iter
where Iter: Iterator<Item = I>, R: Receiver<I, E> {
    fn send(&mut self, mut receiver: R) {
        if receiver.start() == State::Stop {
            return;
        }
        for item in self {
            if receiver.next(Ok(item)) == State::Stop {
                break;
            }
        }
        receiver.finish();
    }
}

pub struct CapturingReceiver<I, E> {
    items: Vec<Result<I, E>>,
}

impl<I, E> CapturingReceiver<I, E> {
    pub fn new() -> CapturingReceiver<I, E> {
        CapturingReceiver { items: vec![] }
    }
}

impl<I, E> Receiver<I, E> for CapturingReceiver<I, E> {
    fn start(&mut self) -> State {
        State::Continue
    }

    fn next(&mut self, item: Result<I, E>) -> State {
        self.items.push(item);
        State::Continue
    }

    fn finish(&mut self) {}
}

impl<'a, I, E, R> Receiver<I, E> for &'a mut R where R: Receiver<I, E>{
    fn start(&mut self) -> State {
        (*self).start()
    }

    fn next(&mut self, item: Result<I, E>) -> State {
        (*self).next(item)
    }

    fn finish(&mut self) {
        (*self).finish()
    }
}

//pub fn assert_received<I, E, S>(sender: &mut S, items: Vec<I>)
//    where I: Debug, S: Sender<I, E> {
//    let mut receiver = CapturingReceiver::new();
//    sender.send(&mut receiver);
//    assert_eq!(receiver.items, items);
//}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_map() {
        let items = vec![1, 2];
        let mut receiver = CapturingReceiver::new();
        Sender::<_, (), _>::send(&mut items.iter().cloned(), &mut receiver);
        assert_eq!(receiver.items.into_iter().filter_map(|result| result.ok()).collect::<Vec<_>>(), items);
    }
}