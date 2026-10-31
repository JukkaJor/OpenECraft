//! Work spread over the threads of the machine (one in the browser): two tasks at once,
//! a function over many items. Generated texts keep the language of the calling thread.

/// Number of threads work may be spread over: 1 where the platform has none (browser).
pub fn threads() -> usize {
    std::thread::available_parallelism().map_or(1, |n| n.get())
}

/// Runs `a` and `b`, on two threads where the platform has them. The language of
/// generated texts carries over to the other thread.
pub fn join<A: Send, B: Send>(a: impl FnOnce() -> A + Send, b: impl FnOnce() -> B + Send) -> (A, B) {
    if threads() < 2 {
        return (a(), b());
    }
    let lang = openecraft_i18n::lang();
    std::thread::scope(|s| {
        let a = s.spawn(move || openecraft_i18n::with_lang(lang, a));
        let b = b();
        (a.join().unwrap_or_else(|e| std::panic::resume_unwind(e)), b)
    })
}

/// `f` of every item, in the order of `items`, spread over the available threads
/// (e.g. the checks of a project, which do not depend on each other).
pub fn par_map<T: Sync, R: Send>(items: &[T], f: impl Fn(&T) -> R + Sync) -> Vec<R> {
    par_map_n(items, threads(), f)
}

/// Like [`par_map`], on at most `threads` threads.
pub fn par_map_n<T: Sync, R: Send>(items: &[T], threads: usize, f: impl Fn(&T) -> R + Sync) -> Vec<R> {
    let n = threads.min(items.len());
    if n < 2 {
        return items.iter().map(f).collect();
    }
    let lang = openecraft_i18n::lang();
    let next = std::sync::atomic::AtomicUsize::new(0);
    let mut done: Vec<(usize, R)> = std::thread::scope(|s| {
        let workers: Vec<_> = (0..n)
            .map(|_| {
                s.spawn(|| {
                    openecraft_i18n::with_lang(lang, || {
                        let mut out = Vec::new();
                        loop {
                            let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                            let Some(item) = items.get(i) else { break };
                            out.push((i, f(item)));
                        }
                        out
                    })
                })
            })
            .collect();
        workers.into_iter().flat_map(|w| w.join().unwrap_or_else(|e| std::panic::resume_unwind(e))).collect()
    });
    done.sort_by_key(|(i, _)| *i);
    done.into_iter().map(|(_, r)| r).collect()
}
