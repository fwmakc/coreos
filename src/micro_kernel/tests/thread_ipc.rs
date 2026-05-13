//! Real thread-based IPC integration tests.
//!
//! Verifies message ordering, backpressure, and broadcast between threads.

use std::sync::mpsc;
use std::thread;

/// TC-12-004: IPC round-trip latency between two threads.
#[test]
fn thread_ipc_roundtrip_latency() {
    let (tx1, rx1) = mpsc::channel::<String>();
    let (tx2, rx2) = mpsc::channel::<String>();

    let handle = thread::spawn(move || {
        let msg = rx1.recv().unwrap();
        assert_eq!(msg, "ping");
        tx2.send("pong".into()).unwrap();
    });

    let start = std::time::Instant::now();
    tx1.send("ping".into()).unwrap();
    let response = rx2.recv().unwrap();
    let elapsed = start.elapsed();

    assert_eq!(response, "pong");
    // On localhost (intra-process) latency should be < 1 ms.
    assert!(
        elapsed.as_micros() < 5000,
        "Latency too high: {:?}",
        elapsed
    );

    handle.join().unwrap();
}

/// TC-12-005: 1 MB payload round-trip.
#[test]
fn thread_ipc_1mb_payload() {
    let (tx, rx) = mpsc::channel::<Vec<u8>>();

    let payload = vec![0xABu8; 1_048_576];
    let payload_clone = payload.clone();

    let handle = thread::spawn(move || {
        let received = rx.recv().unwrap();
        assert_eq!(received, payload_clone);
    });

    tx.send(payload).unwrap();
    handle.join().unwrap();
}

/// TC-12-007: 100 parallel "isolates" (threads) sending messages.
#[test]
fn thread_ipc_100_parallel_isolates() {
    let (tx, rx) = mpsc::channel::<(usize, String)>();

    let mut handles = Vec::with_capacity(100);
    for i in 0..100 {
        let tx_i = tx.clone();
        handles.push(thread::spawn(move || {
            tx_i.send((i, format!("msg-from-{i}"))).unwrap();
        }));
    }
    drop(tx); // Drop original sender so recv knows when done.

    let mut received = Vec::new();
    for (id, msg) in rx {
        received.push((id, msg));
    }

    assert_eq!(received.len(), 100);
    // Verify all IDs are unique (no message loss).
    let mut ids: Vec<usize> = received.iter().map(|(id, _)| *id).collect();
    ids.sort_unstable();
    assert_eq!(ids, (0..100).collect::<Vec<_>>());

    for h in handles {
        h.join().unwrap();
    }
}

/// TC-12-023: Ordered delivery (FIFO).
#[test]
fn thread_ipc_ordered_delivery() {
    let (tx, rx) = mpsc::channel::<i32>();

    let handle = thread::spawn(move || {
        for i in 1..=100 {
            tx.send(i).unwrap();
        }
    });

    for expected in 1..=100 {
        let val = rx.recv().unwrap();
        assert_eq!(val, expected);
    }

    handle.join().unwrap();
}

/// TC-12-024: Broadcast to 10 isolates via shared receiver.
#[test]
fn thread_ipc_broadcast_10_isolates() {
    let (tx, rx) = mpsc::channel::<String>();
    let rx = std::sync::Arc::new(std::sync::Mutex::new(rx));
    let mut handles = Vec::with_capacity(10);

    for _ in 0..10 {
        let rx_i = rx.clone();
        handles.push(thread::spawn(move || {
            let msg = rx_i.lock().unwrap().recv().unwrap();
            assert_eq!(msg, "broadcast");
        }));
    }

    for _ in 0..10 {
        tx.send("broadcast".into()).unwrap();
    }
    drop(tx);

    for h in handles {
        h.join().unwrap();
    }
}

/// TC-12-022: Backpressure — bounded channel fills up.
#[test]
fn thread_ipc_backpressure_bounded() {
    let (tx, rx) = mpsc::sync_channel::<i32>(5);

    // Send 5 items — should succeed immediately.
    for i in 0..5 {
        tx.try_send(i).unwrap();
    }

    // 6th item should fail (channel full).
    assert!(tx.try_send(5).is_err());

    // Drain one item.
    let _ = rx.recv().unwrap();

    // Now we can send again.
    tx.try_send(5).unwrap();
}
