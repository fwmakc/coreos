//! Lock-free single-producer single-consumer ring buffer for real-time audio.
//!
//! The producer writes data and advances `tail`; the consumer reads data and
//! advances `head`. Atomic ordering guarantees that written data is visible to
//! the reader before `tail` is incremented.

use std::cell::UnsafeCell;
use std::ptr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use zeroize::Zeroize;

/// Minimum ring buffer capacity in samples (power of 2).
const MIN_CAPACITY: usize = 1024;

/// Shared ring buffer handle passed between the audio stream and the cpal callback.
pub type SharedRingBuffer = Arc<RingBufferInner>;

/// Lock-free SPSC ring buffer storing interleaved `f32` audio samples.
pub struct RingBufferInner {
    data: UnsafeCell<Vec<f32>>,
    capacity: usize,
    mask: usize,
    head: AtomicUsize,
    tail: AtomicUsize,
}

// SAFETY: RingBufferInner is a single-producer single-consumer data structure.
// Only the producer modifies `tail` and writes to `data[tail..]`.
// Only the consumer modifies `head` and reads from `data[head..tail]`.
// Proper atomic ordering (Acquire/Release) prevents data races.
unsafe impl Sync for RingBufferInner {}
unsafe impl Send for RingBufferInner {}

impl RingBufferInner {
    /// Create a new ring buffer with at least `min_samples` capacity.
    ///
    /// The actual capacity is rounded up to the next power of two for efficient
    /// modular arithmetic via bitwise masking.
    pub fn new(min_samples: usize) -> Self {
        let capacity = (min_samples.max(MIN_CAPACITY)).next_power_of_two();
        let mask = capacity - 1;
        Self {
            data: UnsafeCell::new(vec![0.0f32; capacity]),
            capacity,
            mask,
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
        }
    }

    /// Number of samples available to read.
    pub fn available_read(&self) -> usize {
        let tail = self.tail.load(Ordering::Acquire);
        let head = self.head.load(Ordering::Acquire);
        tail.wrapping_sub(head)
    }

    /// Number of samples that can be written.
    pub fn available_write(&self) -> usize {
        let tail = self.tail.load(Ordering::Acquire);
        let head = self.head.load(Ordering::Acquire);
        self.capacity - tail.wrapping_sub(head)
    }

    /// Write samples into the ring buffer (producer side).
    ///
    /// Returns the number of samples actually written (may be less than `data`
    /// if the buffer is nearly full).
    pub fn write(&self, data: &[f32]) -> usize {
        let tail = self.tail.load(Ordering::Acquire);
        let head = self.head.load(Ordering::Acquire);
        let free = self.capacity - tail.wrapping_sub(head);
        let to_write = data.len().min(free);

        // SAFETY: We are the only producer. Positions [tail, tail+to_write)
        // are not read by the consumer because they are beyond `tail`.
        let buf = unsafe { &mut *self.data.get() };
        for i in 0..to_write {
            buf[(tail + i) & self.mask] = data[i];
        }

        self.tail.store(tail + to_write, Ordering::Release);
        to_write
    }

    /// Read samples from the ring buffer (consumer side).
    ///
    /// Returns the number of samples actually read (may be less than `out`
    /// if the buffer has fewer samples available).
    pub fn read(&self, out: &mut [f32]) -> usize {
        let tail = self.tail.load(Ordering::Acquire);
        let head = self.head.load(Ordering::Acquire);
        let available = tail.wrapping_sub(head);
        let to_read = out.len().min(available);

        // SAFETY: We are the only consumer. Positions [head, head+to_read)
        // are not written by the producer because they are before `tail`.
        let buf = unsafe { &*self.data.get() };
        for i in 0..to_read {
            out[i] = buf[(head + i) & self.mask];
        }

        self.head.store(head + to_read, Ordering::Release);
        to_read
    }

    /// Force-write samples, overwriting oldest data if the buffer is full.
    ///
    /// Used by the input (capture) stream: new microphone data must not be
    /// dropped, so old data is sacrificed instead. Returns the number of
    /// samples that were overwritten.
    pub fn write_overwrite(&self, data: &[f32]) -> usize {
        let len = data.len();
        let tail = self.tail.load(Ordering::Acquire);
        let head = self.head.load(Ordering::Acquire);
        let used = tail.wrapping_sub(head);
        let mut overwritten = 0;

        if len > self.capacity - used {
            overwritten = len - (self.capacity - used);
            let new_head = head + overwritten;
            self.head.store(new_head, Ordering::Release);
        }

        // SAFETY: Same as write() — we are the sole producer.
        let buf = unsafe { &mut *self.data.get() };
        for i in 0..len {
            buf[(tail + i) & self.mask] = data[i];
        }

        self.tail.store(tail + len, Ordering::Release);
        overwritten
    }

    /// Fill `out` with samples from the ring buffer. If fewer samples are
    /// available than requested, the remainder is filled with zeros (silence).
    ///
    /// Returns `true` if an underrun occurred (not enough data).
    pub fn read_or_silence(&self, out: &mut [f32]) -> bool {
        let tail = self.tail.load(Ordering::Acquire);
        let head = self.head.load(Ordering::Acquire);
        let available = tail.wrapping_sub(head);
        let to_read = out.len().min(available);
        let underrun = to_read < out.len();

        // SAFETY: Same as read().
        let buf = unsafe { &*self.data.get() };
        for i in 0..to_read {
            out[i] = buf[(head + i) & self.mask];
        }

        self.head.store(head + to_read, Ordering::Release);

        if underrun {
            // SAFETY: we are writing to out[to_read..] which is a valid
            // mutable slice region. write_bytes fills with 0u8 which is
            // a valid bit pattern for f32 (positive zero).
            unsafe {
                ptr::write_bytes(out.as_mut_ptr().add(to_read), 0, out.len() - to_read);
            }
        }

        underrun
    }
}

impl Zeroize for RingBufferInner {
    fn zeroize(&mut self) {
        // SAFETY: zeroize() takes &mut self, so we have exclusive access.
        // No concurrent reads or writes are possible.
        let buf = unsafe { &mut *self.data.get() };
        unsafe { ptr::write_bytes(buf.as_mut_ptr(), 0, buf.len()) };
        self.head.store(0, Ordering::SeqCst);
        self.tail.store(0, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_then_read() {
        let rb = RingBufferInner::new(16);
        let written = rb.write(&[1.0, 2.0, 3.0]);
        assert_eq!(written, 3);

        let mut out = [0.0f32; 3];
        let read = rb.read(&mut out);
        assert_eq!(read, 3);
        assert_eq!(out, [1.0, 2.0, 3.0]);
    }

    #[test]
    fn read_empty_returns_zero() {
        let rb = RingBufferInner::new(16);
        let mut out = [0.0f32; 4];
        let read = rb.read(&mut out);
        assert_eq!(read, 0);
    }

    #[test]
    fn write_full() {
        let rb = RingBufferInner::new(16);
        let data = vec![1.0f32; rb.capacity + 10];
        let written = rb.write(&data);
        assert_eq!(written, rb.capacity);
        assert_eq!(rb.available_write(), 0);
    }

    #[test]
    fn write_overwrite_drops_oldest() {
        let rb = RingBufferInner::new(16);
        let cap = rb.capacity;
        let half = vec![1.0f32; cap / 2];
        rb.write(&half);
        rb.write(&half);
        assert_eq!(rb.available_write(), 0);

        let extra = vec![99.0f32; 4];
        let overwritten = rb.write_overwrite(&extra);
        assert!(overwritten > 0);

        let mut out = vec![0.0f32; cap];
        let read = rb.read(&mut out);
        assert_eq!(read, cap);
        assert_eq!(out[0], 1.0);
        assert_eq!(out[cap - 4], 99.0);
    }

    #[test]
    fn read_or_silence_fills_zeros() {
        let rb = RingBufferInner::new(16);
        rb.write(&[42.0]);

        let mut out = [0.0f32; 4];
        let underrun = rb.read_or_silence(&mut out);
        assert!(underrun);
        assert_eq!(out[0], 42.0);
        assert_eq!(out[1], 0.0);
        assert_eq!(out[2], 0.0);
        assert_eq!(out[3], 0.0);
    }

    #[test]
    fn read_or_silence_no_underrun() {
        let rb = RingBufferInner::new(16);
        rb.write(&[1.0, 2.0, 3.0, 4.0]);

        let mut out = [0.0f32; 4];
        let underrun = rb.read_or_silence(&mut out);
        assert!(!underrun);
        assert_eq!(out, [1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn zeroize_clears_buffer() {
        let rb = RingBufferInner::new(16);
        rb.write(&[1.0, 2.0, 3.0]);
        assert_eq!(rb.available_read(), 3);

        let mut rb = rb;
        use zeroize::Zeroize;
        rb.zeroize();
        assert_eq!(rb.available_read(), 0);
    }

    #[test]
    fn wrapping_works() {
        let rb = RingBufferInner::new(8);
        for round in 0..10 {
            let data: Vec<f32> = (0..5).map(|i| round as f32 * 10.0 + i as f32).collect();
            rb.write(&data);
            let mut out = [0.0f32; 5];
            let read = rb.read(&mut out);
            assert_eq!(read, 5);
            for (i, val) in out.iter().take(5).enumerate() {
                assert_eq!(*val, round as f32 * 10.0 + i as f32);
            }
        }
    }
}
