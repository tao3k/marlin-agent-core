// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Executable design prototype: not a production event bus or durable log.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct EventId(u64);
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ResourceId(u64);
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Lifecycle {
    Started,
    CleanupRequested,
    Reclaimed,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Event {
    id: EventId,
    resource: ResourceId,
    state: Lifecycle,
    bytes: usize,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Admission {
    Accepted,
    Backpressured,
    NoSubscribers,
    TooLarge,
}
#[derive(Debug)]
struct Subscriber {
    open: bool,
    item_limit: usize,
    byte_limit: usize,
    queue: Vec<Event>,
    bytes: usize,
}
impl Subscriber {
    fn new(item_limit: usize, byte_limit: usize) -> Self {
        Self {
            open: true,
            item_limit,
            byte_limit,
            queue: vec![],
            bytes: 0,
        }
    }
    fn take(&mut self) -> Option<Event> {
        if self.queue.is_empty() {
            return None;
        }
        let event = self.queue.remove(0);
        self.bytes -= event.bytes;
        Some(event)
    }
    fn close(&mut self) {
        self.open = false;
        self.queue.clear();
        self.bytes = 0;
    }
}
fn publish(subscribers: &mut [Subscriber], event: Event) -> Admission {
    let active = subscribers.iter().filter(|s| s.open);
    if active.clone().count() == 0 {
        return Admission::NoSubscribers;
    }
    if active.clone().any(|s| event.bytes > s.byte_limit) {
        return Admission::TooLarge;
    }
    if active
        .clone()
        .any(|s| s.queue.len() >= s.item_limit || s.bytes > s.byte_limit - event.bytes)
    {
        return Admission::Backpressured;
    }
    for subscriber in subscribers.iter_mut().filter(|s| s.open) {
        subscriber.bytes += event.bytes;
        subscriber.queue.push(event);
    }
    Admission::Accepted
}
#[test]
fn reliable_admission_is_atomic_and_capacity_follows_each_consumer() {
    let mut s = [Subscriber::new(1, 2), Subscriber::new(1, 2)];
    let a = Event {
        id: EventId(1),
        resource: ResourceId(7),
        state: Lifecycle::Started,
        bytes: 2,
    };
    let b = Event {
        id: EventId(2),
        state: Lifecycle::CleanupRequested,
        ..a
    };
    assert_eq!(publish(&mut s, a), Admission::Accepted);
    assert_eq!(s[0].take(), Some(a));
    assert_eq!(publish(&mut s, b), Admission::Backpressured);
    assert_eq!(s[0].queue.len(), 0);
    s[1].close();
    assert_eq!(publish(&mut s, b), Admission::Accepted);
    assert_eq!(s[0].take(), Some(b));
    assert_eq!(s[0].bytes, 0);
    let big = Event {
        state: Lifecycle::Reclaimed,
        bytes: 3,
        ..a
    };
    assert_eq!(publish(&mut s, big), Admission::TooLarge);
    s[0].close();
    assert_eq!(publish(&mut s, a), Admission::NoSubscribers);
}
