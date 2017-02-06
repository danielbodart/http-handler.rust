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
    fn send(&mut self, receiver: Receiver<I, E>);
}

pub struct MapReceiver<M, R> {
    receiver: R,
    mapper: M,
}

impl<I, O, E, F, R> Receiver<O, E> for MapReceiver<F, R>
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

pub trait Transducee<I, O, E, OR, IR> where OR: Receiver<O, E>, IR: Receiver<I, E> {
    fn apply(self, receiver: OR) -> IR;
}

pub struct MapTransducee<M> {
    mapper: M,
}

impl<I, O, E, F, R> Transducee<I, O, E, R, MapReceiver<F, R>> for MapTransducee<F>
where F: Fn(Result<I, E>) -> Result<O, E>,
      R: Receiver<O, E> {
    fn apply(self, receiver: R) -> MapReceiver<F, R> {
        MapReceiver {
            receiver: receiver,
            mapper: self.mapper,
        }
    }
}


//impl<I, E, Iter> Sender<I, E> for Iter where Iter: Iterator<Item = I> {
//    fn send(&mut self, receiver: &mut Receiver<I, E>) {
//        if receiver.start() == State::Stop {
//            return;
//        }
//        for item in self {
//            if receiver.next(Ok(item)) == State::Stop {
//                break;
//            }
//        }
//        receiver.finish();
//    }
//}
//
//pub struct CapturingReceiver<I, E> {
//    items: Vec<Result<I, E>>,
//}
//
//impl<I, E> CapturingReceiver<I, E> {
//    pub fn new() -> CapturingReceiver<I, E> {
//        CapturingReceiver { items: vec![] }
//    }
//}
//
//impl<I, E> Receiver<I, E> for CapturingReceiver<I, E> {
//    fn start(&mut self) -> State {
//        State::Continue
//    }
//
//    fn next(&mut self, item: Result<I, E>) -> State {
//        self.items.push(item);
//        State::Continue
//    }
//
//    fn finish(&mut self) {}
//}

//pub fn assert_received<I, E, S>(sender: &mut S, items: Vec<I>)
//    where I: Debug, S: Sender<I, E> {
//    let mut receiver = CapturingReceiver::new();
//    sender.send(&mut receiver);
//    assert_eq!(receiver.items, items);
//}


#[cfg(test)]
mod tests {
    //    use super::*;

    #[test]
    fn can_map() {
        //        let items = vec![1, 2];
        //        let mut receiver = CapturingReceiver::new();
        //        Sender::<_, ()>::send(&mut items.iter().cloned(), &mut receiver);
        //        assert_eq!(receiver.items.into_iter().filter_map(|result| result.ok()).collect::<Vec<_>>(), items);
    }
}