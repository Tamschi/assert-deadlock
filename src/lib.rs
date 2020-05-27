//! Assert that a deadlock happens. Mildly cursed.

#![doc(html_root_url = "https://docs.rs/assert-deadlock/1.0.0-alpha.1")]
#![doc(test(no_crate_inject))]
#![warn(
    clippy::as_conversions,
    clippy::cargo,
    clippy::clone_on_ref_ptr,
    clippy::missing_docs_in_private_items,
    clippy::pedantic
)]
// Debug cleanup. Uncomment before committing.
#![forbid(
    clippy::dbg_macro,
    clippy::print_stdout,
    clippy::todo,
    clippy::unimplemented
)]

/// Asserts that `$stmt` deadlocks.
///
/// # Panics
///
/// Iff `$stmt` doesn't lock up for at least `$duration`.
///
/// # Example
///
/// ```rust
/// # use {
/// #     assert_panic::assert_panic,
/// #     std::{sync::Mutex, time::Duration},
/// # };
/// use assert_deadlock::assert_deadlock;
///
/// let mutex = Mutex::new(());
///
/// assert_panic!(
///     assert_deadlock!(
///         { },
///         Duration::from_secs(1),
///     ),
///     &str,
///     "assert_deadlock! expression returned.",
/// );
/// 
/// let guard = mutex.lock();
/// assert_deadlock!(
///     { Box::leak(Box::new(mutex.lock())); },
///     Duration::from_secs(1),
/// );
/// ```
///
/// # Details
///
/// If this macro panics from `$stmt` completing, effects of `$stmt` are reliably observable.
///
/// If `$stmt` panics, that panic is propagated:
///
/// ```rust
/// # use {
/// #     assert_panic::assert_panic,
/// #     std::{sync::Mutex, time::Duration},
/// # };
/// use assert_deadlock::assert_deadlock;
///
/// assert_panic!(
///     assert_deadlock!(
///         panic!("Inner panic!"),
///         Duration::from_secs(1),
///     ),
///     &str,
///     "Inner panic!",
/// );
/// ```
#[macro_export]
macro_rules! assert_deadlock {
    ($stmt:stmt, $duration:expr$(,)?) => {{
        use std::{
            any::Any,
            mem::transmute,
            panic::{catch_unwind, resume_unwind, UnwindSafe},
            sync::{Arc, Mutex},
            thread,
        };

        let stmt: Box<dyn FnOnce() + UnwindSafe + '_> = Box::new(|| {
            {}
            $stmt
        });
        let stmt: Box<dyn FnOnce() + UnwindSafe + Send + 'static> = unsafe {
            //SAFETY: Essentially the same type, externally synchronised.
            transmute(stmt)
        };
        let panic_slot: Arc<Mutex<Option<Box<dyn Any + Send + 'static>>>> = Arc::default();
        let _ = thread::spawn({
            let panic_slot = panic_slot.clone();
            move || {
                panic_slot
                    .lock()
                    .unwrap()
                    .replace(catch_unwind(stmt).map_or_else(
                        |error| error,
                        |()| Box::new("assert_deadlock! expression returned."),
                    ));
            }
        });
        thread::sleep($duration);
        if let Ok(mut guard) = panic_slot.try_lock() {
            let panic = guard.take();
            drop(guard);
            if let Some(panic) = panic {
                resume_unwind(panic); // Hm, doesn't seem to quite work yet with inner panics.
            } else {
                panic!("assert_deadlock!: Could not start `$stmt` during `$duration`");
            }
        } else {
            // Still locked, all good.
        };
    }};
}
