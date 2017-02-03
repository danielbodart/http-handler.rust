#[derive(PartialEq, Debug)]
pub enum State {
    Continue,
    Stop
}

pub trait Receiver<I, E> {
    fn start(&mut self) -> State;
    fn next(&mut self, item: Result<I, E>) -> State;
    fn finish(&mut self);
}

pub trait Sender<I, E> {
    fn send(&mut self, receiver: &mut Receiver<I, E>);
}

pub trait Transducer<I, O, E> {
    type Output: Receiver<I, E>;

    fn apply(&mut self, receiver: &mut Receiver<O, E>) -> Self::Output;
}

pub struct MapTransducer<I, O, E> {
    mapper: Fn(Result<I, E>) -> O,
}

impl<I, O, E> Transducer<I, O, E> for MapTransducer<I, O, E> {
    type Output = MapReceiver<I, O, E>;

    fn apply(&mut self, receiver: &mut Receiver<O, E>) -> Self::Output {
        MapReceiver {
            mapper: self.mapper,
            receiver: receiver,
        }
    }

}

pub struct MapReceiver<I, O, E> {
    mapper: Box<Fn(Result<I, E>) -> O>,
    receiver: Receiver<O, E>,
}

impl<I,O,E> Receiver<I, E>  for MapReceiver<I, O, E>{
    fn start(&mut self) -> State {
        unimplemented!()
    }

    fn next(&mut self, item: Result<I, E>) -> State {
        unimplemented!()
    }

    fn finish(&mut self) {
        unimplemented!()
    }
}

impl<I, E, Iter> Sender<I, E> for Iter where Iter: Iterator<Item = I> {
    fn send(&mut self, receiver: &mut Receiver<I, E>) {
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
        Sender::<_, ()>::send(&mut items.iter().cloned(), &mut receiver);
        assert_eq!(receiver.items.into_iter().filter_map(|result| result.ok()).collect::<Vec<_>>(), items);
    }
}