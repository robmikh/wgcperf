use std::{
    sync::{
        Arc, RwLock,
        mpsc::{Receiver, Sender, channel},
    },
    time::Duration,
};

use windows::{
    Foundation::TypedEventHandler,
    System::{DispatcherQueue, DispatcherQueueHandler, DispatcherQueueTimer},
    Win32::Foundation::LUID,
    core::Result,
};

use crate::perf::PerfTracker;

pub struct PerfSession {
    inner: Arc<RwLock<PerfSessionInner>>,
    receiver: Receiver<Vec<f64>>,
}

struct PerfSessionInner {
    target_length: Duration,
    current_length: Duration,
    tick_length: Duration,
    samples: Vec<f64>,
    tracker: PerfTracker,
    timer: DispatcherQueueTimer,
    timer_token: Option<i64>,
    sender: Sender<Vec<f64>>,
}
// SAFETY: This will only ever be accessed by the UI thread, but the DispatcherQueueTimer's
//         Tick event requires Send and Sync.
unsafe impl Send for PerfSessionInner {}
unsafe impl Sync for PerfSessionInner {}

impl PerfSession {
    pub fn run_on_thread(
        thread: &DispatcherQueue,
        duration: Duration,
        process_id: u32,
        luid: Option<LUID>,
    ) -> Result<Vec<f64>> {
        let (inner, receiver) = {
            let (sender, receiver) = channel();
            thread.TryEnqueue(&DispatcherQueueHandler::new(move || -> Result<()> {
                let result = PerfSessionInner::start(duration, process_id, luid);
                sender.send(result).unwrap();
                Ok(())
            }))?;
            receiver.recv().unwrap()?
        };

        let this = Self { inner, receiver };
        let samples = this.receiver.recv().unwrap();

        Ok(samples)
    }
}

impl PerfSessionInner {
    fn start(
        duration: Duration,
        process_id: u32,
        luid: Option<LUID>,
    ) -> Result<(Arc<RwLock<Self>>, Receiver<Vec<f64>>)> {
        let target_length = duration;
        let current_length = Duration::from_secs(0);
        let tracker = PerfTracker::new(process_id, luid)?;
        let samples = Vec::new();

        let dispatcher_queue = DispatcherQueue::GetForCurrentThread()?;
        let timer = dispatcher_queue.CreateTimer()?;
        let tick_length = Duration::from_millis(500);
        timer.SetInterval(tick_length.into())?;
        timer.SetIsRepeating(true)?;

        let (sender, receiver) = channel();
        let inner = Arc::new(RwLock::new(Self {
            target_length,
            current_length,
            tick_length,
            samples,
            tracker,
            timer: timer.clone(),
            timer_token: None,
            sender,
        }));

        let token = timer.Tick(&TypedEventHandler::<DispatcherQueueTimer, _>::new({
            let inner = Arc::downgrade(&inner);
            move |timer, _| -> Result<()> {
                if let Some(inner) = inner.upgrade() {
                    let mut inner = inner.write().unwrap();
                    inner.on_tick()?;
                } else {
                    let timer = timer.unwrap();
                    timer.Stop()?;
                }

                Ok(())
            }
        }))?;

        {
            let mut inner = inner.write().unwrap();
            inner.timer_token = Some(token);
            inner.tracker.start()?;
            inner.timer.Start()?;
        }
        Ok((inner, receiver))
    }

    fn on_tick(&mut self) -> Result<()> {
        self.samples.push(self.tracker.get_current_value()?);

        self.current_length += self.tick_length;
        if self.current_length >= self.target_length {
            self.timer.Stop()?;
            let samples: Vec<_> = self.samples.drain(..).collect();
            self.sender.send(samples).unwrap();
        }

        Ok(())
    }
}

impl Drop for PerfSessionInner {
    fn drop(&mut self) {
        if let Some(token) = self.timer_token.take() {
            let _ = self.timer.RemoveTick(token);
        }
    }
}
