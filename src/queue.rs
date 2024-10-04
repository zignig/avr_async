
/// A generic data queue.
/// If you have incoming stuff that can happen quickly , use this.
/// majorly simplified version https://github.com/embassy-rs/embassy/blob/main/embassy-sync/src/channel.rs
/// Refcell only , not for use with interrupts.

use core::{
    cell::RefCell,
    future::poll_fn,
    task::{Poll, Waker},
};

use heapless::Deque;

// The queue itself
pub struct Queue<T, const N: usize> {
    queue: RefCell<Deque<T, N>>,
    waker: RefCell<Option<Waker>>,
}

impl<T, const N: usize> Queue<T, N> {
    pub const fn new() -> Self {
        Self {
            queue: RefCell::new(Deque::new()),
            waker: RefCell::new(None),
        }
    }

    pub fn get_sender(&self) -> Sender<T, N> {
        Sender { queue: &self }
    }

    pub fn get_receiver(&self) -> Receiver<T, N> {
        Receiver {
            queue: &self,
            state: ReceiverState::Init,
        }
    }

    pub fn send(&self, item: T) {
        if self.queue.borrow_mut().push_back(item).is_ok() {
            if let Some(waker) = &mut *self.waker.borrow_mut() {
                waker.wake_by_ref();
            }
        };
    }

    fn receive(&self) -> Option<T> {
        self.queue.borrow_mut().pop_front()
    }

    fn register(&self, waker: Waker) {
        self.waker.replace(Some(waker));
    }

    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.queue.borrow_mut().len()
    }
}


// The sender half
pub struct Sender<'a, T, const N: usize> {
    queue: &'a Queue<T, N>,
}

impl<T, const N: usize> Sender<'_, T, N> {
    pub fn send(&mut self, item: T) {
        self.queue.send(item);
    }

    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.queue.len()
    }
}

// The reciever half , this does futures.
enum ReceiverState {
    Init,
    Wait,
}

pub struct Receiver<'a, T, const N: usize> {
    queue: &'a Queue<T, N>,
    state: ReceiverState,
}

impl<T, const N: usize> Receiver<'_, T, N> {
    pub async fn receive(&mut self) -> T {
        poll_fn(|cx| match self.state {
            ReceiverState::Init => {
                self.queue.register(cx.waker().clone());
                self.state = ReceiverState::Wait;
                Poll::Pending
            }
            ReceiverState::Wait => match self.queue.receive() {
                Some(item) => Poll::Ready(item),
                None => Poll::Pending,
            },
        })
        .await
    }
}
