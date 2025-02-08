use std::{
    cell::Cell, future::Future, pin::Pin, sync::{atomic::{AtomicUsize,Ordering}, Arc, Once, OnceLock},
    task::{Context, Poll, Waker}, time::Duration
};

//public API to create a channel
pub fn oneshot<T>() -> (Sender<T>, Receiver<T>) {
    let chan = Arc::new(Chan::new());
    (Sender{chan: chan.clone()}, Receiver{chan})
}

struct Chan<T> {
    tx: Once,
    sender_rc: AtomicUsize,
    data: Cell<Option<T>>,
    waker: OnceLock<Waker>,
}

impl<T> Chan<T> {
    const fn new() -> Self {
        Self {
            tx: Once::new(),
            sender_rc: AtomicUsize::new(1),
            data: Cell::new(None),
            waker: OnceLock::new(),
        }
    }

    fn set(&self, data: T) -> Result<(), T> {
        let mut data = Some(data);

        self.tx.call_once(||{
            self.data.set(data.take());
        });

        match data {
            None => {
                if let Some(waker) = self.waker.get() {
                    waker.wake_by_ref();
                }
                Ok(())
            },
            Some(data) => Err(data),
        }
    }

    fn take(&self) -> Option<T> {
        if !self.tx.is_completed() {
            return None;
        }
        self.data.take()
    }

    fn is_set(&self) -> bool {
        self.tx.is_completed()
    }

    fn is_dropped(&self) -> bool {
        self.sender_rc.load(Ordering::Acquire) == 0
    }
}

//marker trait
unsafe impl<T:Send + Sync> Sync for Chan<T> {}

// sending half
pub struct Sender<T> {
    chan: Arc<Chan<T>>,
}

impl<T> Sender<T> {
    pub fn send(&self, data: T) -> Result<(), T> {
        self.chan.set(data)
    }
}

impl<T> Clone for Sender<T> {
    fn clone(&self) -> Self {
        self.chan.sender_rc.fetch_add(1, Ordering::Release);
        Self {
            chan: self.chan.clone(),
        }
    }
}

impl<T> Drop for Sender<T> {
    fn drop(&mut self) {
        if self.chan.sender_rc.fetch_sub(1, Ordering::AcqRel) == 1 {
            // last sender dropped, return None to receive end
            if let Some(waker) = self.chan.waker.get() {
                waker.wake_by_ref();
            }
        }
    }
}

// receiving half
pub struct Receiver<T> {
    chan: Arc<Chan<T>>,
}

pub struct Recv<T> {
    chan: Arc<Chan<T>>,
}

impl<T> Future for Recv<T> {
    type Output = Option<T>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // fast path
        if self.chan.is_set() || self.chan.is_dropped() {
            return Poll::Ready(self.chan.take());
        }

        // check & set the waker
        self.chan.waker.get_or_init(|| {
            cx.waker().clone()
        });

        if self.chan.is_set() || self.chan.is_dropped() {
            Poll::Ready(self.chan.take())
        } else {
            Poll::Pending
        }
    }
}

impl<T> Receiver<T> {
    pub fn recv(&self) -> Recv<T> {
        Recv {
            chan: self.chan.clone(),
        }
    }
}

#[tokio::main]
async fn main() {
    let (tx, rx) = oneshot::<usize>();

    // send a value
    tx.send(65).unwrap();

    // receive the value async
    let res = tokio::join!(rx.recv()).0;
    println!("{:?}", res);
    // assert_eq!(res, Some(65));
}

#[tokio::test]
async fn channel_test() {
    let (tx, rx) = oneshot::<i32>();
    assert_eq!(Ok(()), tx.send(6555));

    assert_eq!(Some(6555), rx.recv().await);
}

#[tokio::test]
async fn channel_test_multiple_sender() {
    let (tx, rx) = oneshot::<i32>();

    // send from different task
    let tx1 = tx.clone();
    tokio::spawn(async move {
        assert_eq!(Ok(()), tx1.send(12999));
    });

    // send from another task with some delay
    let tx2 = tx.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(2)).await;
        assert_eq!(Err(-1122), tx2.send(-1122));
    });

    tokio::time::sleep(Duration::from_secs(2)).await;
    assert_eq!(Err(6555), tx.send(6555));

    // we should see value only from first task
    assert_eq!(Some(12999), rx.recv().await);

    // let's try to receive again
    assert_eq!(rx.recv().await, None);
}