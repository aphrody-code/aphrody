// SPDX-License-Identifier: Apache-2.0
//
// Host-side hook state — minimal port of the `useState`/`useEffect` slice
// of the TS hook layer (`src/hooks/*.ts`). The full React-fiber hook
// machinery (queue dispatch, scheduling, dependency tracking) is implemented
// inside `react-reconciler` upstream; what lives here is the host-side
// snapshot that the caller threads through `Reconciler::commit()`.
//
// TODO Phase 4.2 follow-up: port `useInput`, `useFocus`, `useFocusManager`,
// `useStdout`, `useWindowSize`. Those are wrappers around runtime IO (raw
// stdin, sigwinch, focus events) that need a cross-platform event loop —
// shipping them belongs to `aphrody-terminal-backend` integration work.

//! Commit-time `use_state` slot + ordered `Effect` enum.

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
use core::cell::RefCell;

/// Generic mutable state cell shared by reconciler callers. Matches the
/// observable shape of React's `useState` returned tuple `[value, setter]`,
/// but exposed as a single struct because Rust closures with mutable capture
/// are awkward in trait objects.
#[derive(Debug)]
pub struct UseState<T> {
    inner: RefCell<T>,
}

impl<T: Clone> UseState<T> {
    /// Creates a fresh state slot with the given initial value.
    pub const fn new(initial: T) -> Self {
        Self {
            inner: RefCell::new(initial),
        }
    }

    /// Returns a clone of the current value.
    pub fn get(&self) -> T {
        self.inner.borrow().clone()
    }

    /// Replaces the value. Equivalent to `setState(next)`.
    pub fn set(&self, next: T) {
        *self.inner.borrow_mut() = next;
    }

    /// Functional setter — `setState(prev => f(prev))`.
    pub fn update<F: FnOnce(T) -> T>(&self, f: F) {
        let prev = self.get();
        self.set(f(prev));
    }
}

/// Commit-time effect. Mirrors the `Effect` shape React's reconciler hands
/// to its `commitMutationEffects`. We expose a typed enum so host code can
/// react without re-implementing React's tag bitmask.
pub enum Effect {
    /// `useEffect(create, deps)` mount effect.
    Mount(Box<dyn FnOnce()>),
    /// `useEffect` cleanup effect.
    Cleanup(Box<dyn FnOnce()>),
    /// Named custom effect (escape hatch).
    Named {
        /// Effect name (debug-only).
        name: String,
        /// Effect body.
        run: Box<dyn FnOnce()>,
    },
}

impl core::fmt::Debug for Effect {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Mount(_) => f.write_str("Effect::Mount(<fn>)"),
            Self::Cleanup(_) => f.write_str("Effect::Cleanup(<fn>)"),
            Self::Named { name, .. } => write!(f, "Effect::Named({name:?})"),
        }
    }
}

/// Ordered effect queue. Drained in FIFO order during commit — matches
/// React's effect-list traversal.
#[derive(Debug, Default)]
pub struct HookState {
    queue: RefCell<Vec<Effect>>,
}

impl HookState {
    /// Constructs a fresh empty queue.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            queue: RefCell::new(Vec::new()),
        }
    }

    /// Enqueues an effect for the next commit.
    pub fn enqueue(&self, effect: Effect) {
        self.queue.borrow_mut().push(effect);
    }

    /// Drains and runs all queued effects in FIFO order.
    pub fn drain_and_run(&self) {
        let drained: Vec<Effect> = core::mem::take(&mut *self.queue.borrow_mut());
        for effect in drained {
            match effect {
                Effect::Mount(f) | Effect::Cleanup(f) | Effect::Named { run: f, .. } => f(),
            }
        }
    }

    /// Returns the number of pending effects (without draining).
    #[must_use]
    pub fn pending(&self) -> usize {
        self.queue.borrow().len()
    }
}
