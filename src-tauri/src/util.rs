// Small shared helpers for the Tauri command layer.

use std::sync::{Mutex, MutexGuard};

/// Lock a long-lived manager mutex, recovering if a previous holder panicked.
///
/// The managers behind these mutexes are plain registries (a `HashMap` of PTY,
/// agent or database sessions). If a panic happens in *our* code while the lock
/// is held, the map itself is still perfectly usable — but `lock().unwrap()`
/// would propagate the poison and make every subsequent call panic too, so a
/// single hiccup would brick all terminals and connections until the app is
/// restarted. Recovering the guard keeps the session registry usable instead.
pub(crate) fn lock_recover<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Drive a future to completion from synchronous code, safely.
///
/// `tauri::async_runtime::block_on` is tokio's `Handle::block_on`, which
/// **panics** when called from a thread that is already driving the runtime:
/// *"Cannot block the current thread from within a runtime."*
///
/// That matters because commands marked `#[tauri::command(async)]` have their
/// body executed inside a runtime task (the macro wraps a sync fn in an
/// `async move` block). Our ClickHouse and SQL Anywhere code paths are sync
/// functions that block on futures, so calling them from an async command
/// would crash at runtime — while compiling perfectly.
///
/// Running the future on a dedicated scoped thread keeps it correct from either
/// context. The thread costs microseconds next to a database round-trip.
pub(crate) fn block_on_future<F>(fut: F) -> F::Output
where
    F: std::future::Future + Send,
    F::Output: Send,
{
    std::thread::scope(|s| {
        s.spawn(|| tauri::async_runtime::block_on(fut))
            .join()
            .expect("blocking database task panicked")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // The regression this guards: the same call from inside a runtime task.
    #[test]
    fn block_on_future_works_inside_a_runtime() {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();
        let got = rt.block_on(async { block_on_future(async { 6 * 7 }) });
        assert_eq!(
            got, 42,
            "blocking on a future inside a runtime must not panic"
        );
    }

    #[test]
    fn block_on_future_works_outside_a_runtime() {
        assert_eq!(block_on_future(async { "ok" }), "ok");
    }

    #[test]
    fn lock_recover_survives_a_poisoned_mutex() {
        let m = std::sync::Arc::new(Mutex::new(vec![1, 2, 3]));
        let m2 = m.clone();
        // Poison it: panic while the lock is held.
        let _ = std::thread::spawn(move || {
            let _g = m2.lock().unwrap();
            panic!("boom");
        })
        .join();
        assert!(m.lock().is_err(), "mutex should be poisoned");
        assert_eq!(
            lock_recover(&m).len(),
            3,
            "recovered guard should still work"
        );
    }
}
