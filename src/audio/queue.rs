use crossbeam_channel::{Sender, TrySendError};

/// Drop a new chunk under backpressure, keeping the capture thread stoppable.
/// False means the consumer disconnected and capture should stop.
pub(super) fn send_or_drop<T>(tx: &Sender<T>, value: T) -> bool {
    match tx.try_send(value) {
        Ok(()) | Err(TrySendError::Full(_)) => true,
        Err(TrySendError::Disconnected(_)) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::send_or_drop;
    use crossbeam_channel::bounded;
    use std::sync::{atomic::{AtomicBool, Ordering}, Arc, mpsc};
    use std::thread;
    use std::time::Duration;

    #[test]
    fn available_queue_keeps_the_chunk() {
        let (tx, rx) = bounded(1);
        assert!(send_or_drop(&tx, 7_u8));
        assert_eq!(rx.try_recv().unwrap(), 7);
    }

    #[test]
    fn disconnected_consumer_stops_capture() {
        let (tx, rx) = bounded(1);
        drop(rx);
        assert!(!send_or_drop(&tx, 7_u8));
    }

    #[test]
    fn full_queue_does_not_prevent_producer_shutdown() {
        let (tx, rx) = bounded(1);
        tx.send(0_u8).unwrap();
        let running = Arc::new(AtomicBool::new(true));
        let producer_running = running.clone();
        let (done_tx, done_rx) = mpsc::channel();
        let worker = thread::spawn(move || {
            let mut connected = send_or_drop(&tx, 1_u8);
            while connected && producer_running.load(Ordering::Acquire) {
                connected = send_or_drop(&tx, 2_u8);
                thread::yield_now();
            }
            let _ = done_tx.send(connected);
        });
        running.store(false, Ordering::Release);
        let completion = done_rx.recv_timeout(Duration::from_secs(2));
        let original = rx.try_recv();
        drop(rx);
        let joined = worker.join();
        assert!(joined.is_ok(), "producer panicked during cleanup");
        assert!(matches!(completion, Ok(true)),
                "producer blocked with a full live consumer: {completion:?}");
        assert_eq!(original.unwrap(), 0);
    }
}
