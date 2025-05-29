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

use crate::{adapter::Adapter, perf::PerfTracker};

pub struct PerfSession {
    inner: Arc<RwLock<PerfSessionInner>>,
    receiver: Receiver<Vec<Vec<f64>>>,
}

struct PerfSessionInner {
    target_length: Duration,
    current_length: Duration,
    tick_length: Duration,
    trackers: PerfTrackerBundle,
    timer: DispatcherQueueTimer,
    timer_token: Option<i64>,
    sender: Sender<Vec<Vec<f64>>>,
}
// SAFETY: This will only ever be accessed by the UI thread, but the DispatcherQueueTimer's
//         Tick event requires Send and Sync.
unsafe impl Send for PerfSessionInner {}
unsafe impl Sync for PerfSessionInner {}

struct PerfTrackerBundle {
    trackers: Vec<Option<PerfTracker>>,
    samples: Vec<Vec<f64>>,
}

impl PerfSession {
    pub fn run_on_thread(
        thread: &DispatcherQueue,
        duration: Duration,
        process_id: u32,
        adapters: &[Adapter],
        verbose: bool,
    ) -> Result<Vec<Vec<f64>>> {
        let (inner, receiver) = {
            let (sender, receiver) = channel();
            let adapter_luids: Vec<_> = adapters.iter().map(|x| x.luid).collect();
            thread.TryEnqueue(&DispatcherQueueHandler::new(move || -> Result<()> {
                let result = PerfSessionInner::start(duration, process_id, &adapter_luids, verbose);
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
        adapter_luids: &[LUID],
        verbose: bool,
    ) -> Result<(Arc<RwLock<Self>>, Receiver<Vec<Vec<f64>>>)> {
        let target_length = duration;
        let current_length = Duration::from_secs(0);
        let trackers = PerfTrackerBundle::new(adapter_luids, process_id, verbose)?;

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
            trackers,
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
            inner.trackers.start()?;
            inner.timer.Start()?;
        }
        Ok((inner, receiver))
    }

    fn on_tick(&mut self) -> Result<()> {
        self.trackers.update_samples()?;

        self.current_length += self.tick_length;
        if self.current_length >= self.target_length {
            self.timer.Stop()?;
            let samples: Vec<_> = self.trackers.samples.drain(..).collect();
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

impl PerfTrackerBundle {
    fn new(adapter_luids: &[LUID], pid: u32, verbose: bool) -> Result<Self> {
        let trackers: Vec<_> = adapter_luids.iter().map(|x| PerfTracker::new(pid, Some(*x), verbose).ok()).collect();
        let samples = vec![Vec::new(); trackers.len()];
        Ok(Self {
            trackers,
            samples,
        })
    }

    fn start(&self) -> Result<()> {
        for tracker in &self.trackers {
            if let Some(tracker) = tracker {
                tracker.start()?;
            }
        }
        Ok(())
    }

    fn update_samples(&mut self) -> Result<()> {
        for (tracker, samples) in self.trackers.iter().zip(self.samples.iter_mut()) {
            if let Some(tracker) = tracker.as_ref() {
                samples.push(tracker.get_current_value()?);
            }
        }
        Ok(())
    }
}
