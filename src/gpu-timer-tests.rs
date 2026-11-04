use super::*;

#[test]
fn a_frame_is_timed_only_while_a_query_is_free() {
    let mut ring = Ring::new(vec![1, 2]);
    assert_eq!(ring.begin(), Some(2));
    assert_eq!(ring.end(), Some(2));
    assert_eq!(ring.begin(), Some(1));
    assert_eq!(ring.end(), Some(1));
    // Both still wait for the GPU: the third frame goes untimed rather than stalling.
    assert_eq!(ring.begin(), None);
    assert_eq!(ring.end(), None);
}

#[test]
fn finished_queries_are_read_in_order_and_reused() {
    let mut ring = Ring::new(vec![1, 2, 3]);
    for _ in 0..3 {
        ring.begin();
        ring.end();
    }
    // Query 3 started first. Only it and 2 are done; 1 keeps its place.
    let done = [3, 2];
    let times = ring.collect(
        |id| done.contains(&id),
        |id| u64::from(id) * 1_000_000,
        false,
    );
    assert_eq!(times, [3., 2.]);
    assert_eq!(ring.begin(), Some(2));
    let times = ring.collect(|_| true, |_| 4_500_000, false);
    assert_eq!(times, [4.5]);
}

#[test]
fn a_query_is_read_only_after_the_ones_before_it() {
    let mut ring = Ring::new(vec![1, 2]);
    for _ in 0..2 {
        ring.begin();
        ring.end();
    }
    let times = ring.collect(|id| id == 1, |_| 1_000_000, false);
    assert!(
        times.is_empty(),
        "query 1 finished, but query 2 began before it"
    );
}

#[test]
fn a_disjoint_gpu_clock_voids_what_was_read() {
    let mut ring = Ring::new(vec![1]);
    ring.begin();
    ring.end();
    assert!(ring.collect(|_| true, |_| 9_000_000, true).is_empty());
    assert_eq!(ring.begin(), Some(1), "the voided query is free again");
}

#[test]
fn an_unended_query_is_not_read() {
    let mut ring = Ring::new(vec![1]);
    ring.begin();
    assert!(ring.collect(|_| true, |_| 1, false).is_empty());
}
