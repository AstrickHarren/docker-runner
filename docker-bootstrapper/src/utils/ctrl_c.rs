use std::{
    sync::{
        atomic::{AtomicBool, Ordering::*},
        Arc, Mutex,
    },
    task::{Poll, Waker},
};

use futures::Future;

pub async fn ctrl_c() -> Result<(), ctrlc::Error> {
    CtrlC::new()?.await
}

struct CtrlC {
    active: Arc<AtomicBool>,
    waker: Arc<Mutex<Option<Waker>>>,
}

impl CtrlC {
    fn new() -> Result<Self, ctrlc::Error> {
        let waker = Arc::new(Mutex::new(None));
        let active: Arc<AtomicBool> = Arc::new(false.into());
        let ret = Ok(Self {
            active: active.clone(),
            waker: waker.clone(),
        });
        ctrlc::set_handler(move || {
            waker.lock().unwrap().as_ref().unwrap().wake_by_ref();
            active.swap(true, Relaxed);
        })?;
        ret
    }
}

impl Future for CtrlC {
    type Output = Result<(), ctrlc::Error>;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        if self.active.load(Relaxed) {
            Poll::Ready(Ok(()))
        } else {
            *self.waker.lock().unwrap() = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}

#[cfg(test)]
mod tests {
    use super::CtrlC;

    #[ignore = "needs ctrl c"]
    #[tokio::test]
    async fn ctrl_c() -> Result<(), ctrlc::Error> {
        CtrlC::new()?.await?;
        println!("ctrl c awaited");
        Ok(())
    }
}
