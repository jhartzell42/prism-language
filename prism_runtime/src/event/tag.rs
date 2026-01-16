use std::sync::{Arc, Mutex, Weak};

use crate::{
    behavior::Behavior,
    event::{Event, EventCallback, EventImpl, subscriber_list::SubscriberList},
    runtime::{Action, Runtime},
};

struct Tag<A, B> {
    subscriber_list: SubscriberList<(Arc<A>, Arc<B>)>,
    event: Event<A>,
    behavior: Behavior<B>,
    height: Mutex<Option<usize>>,
    weak_self: Weak<Self>,
    inner_sub: Mutex<Option<Arc<TagCallback<A, B>>>>,
}

struct TagCallback<A, B> {
    tag: Weak<Tag<A, B>>,
}

struct TagAction<A, B> {
    tag: Arc<Tag<A, B>>,
    value: Arc<A>,
}

impl<A, B> Action for TagAction<A, B> {
    fn act(self: Box<Self>, runtime: &Runtime) {
        let this = self.tag;
        let value = self.value;
        let subs = this.subscriber_list.consolidate();

        if subs.is_empty() {
            let mut inner_sub = this.inner_sub.lock().unwrap();
            *inner_sub = None;
            return;
        }

        let behavior_value = this.behavior.0.query_for_tag();
        let combined_value = Arc::new((value, behavior_value));

        for sub in subs {
            sub.event_fired(runtime, combined_value.clone());
        }
    }
}

impl<A: 'static, B: 'static> EventCallback<A> for TagCallback<A, B> {
    fn event_fired(&self, runtime: &Runtime, value: Arc<A>) {
        let Some(this) = self.tag.upgrade() else {
            return;
        };
        let height = this.height();
        runtime.schedule(height, TagAction { tag: this, value });
    }
}

impl<A: 'static, B: 'static> Tag<A, B> {
    fn inner_subscription(&self) -> Arc<TagCallback<A, B>> {
        let sub = Arc::new(TagCallback {
            tag: self.weak_self.clone(),
        });
        let sub_dyn: Arc<dyn EventCallback<A>> = sub.clone();
        self.event.0.subscribe(Arc::downgrade(&sub_dyn));
        sub
    }
}

impl<A: 'static, B: 'static> EventImpl<(Arc<A>, Arc<B>)> for Tag<A, B> {
    fn subscribe(&self, cb: Weak<dyn super::EventCallback<(Arc<A>, Arc<B>)>>) {
        self.subscriber_list.add(cb);
        let mut inner_sub = self.inner_sub.lock().unwrap();
        if inner_sub.is_none() {
            *inner_sub = Some(self.inner_subscription());
        }
    }

    fn height(&self) -> usize {
        let mut height = self.height.lock().unwrap();
        if let Some(height) = *height {
            return height;
        }
        let new_height = self.event.0.height() + 1;
        *height = Some(new_height);
        new_height
    }
}

impl<A: 'static> Event<A> {
    /// This creates a new event that is triggered whenever `self` is triggered.
    /// It augments the value of `self` with the value of `behavior` that it had
    /// before this occurrence.
    ///
    /// It is **not** *prompt*, that is, it doesn't reflect updates to the
    /// behavior that happen during this occurrence. Promptness is basically
    /// never what you want anyway.
    pub fn tag<B: 'static>(&self, behavior: Behavior<B>) -> Event<(Arc<A>, Arc<B>)> {
        Event(Arc::new_cyclic(|weak| Tag {
            event: self.clone(),
            behavior,
            subscriber_list: SubscriberList::new(),
            height: Mutex::new(None),
            inner_sub: Mutex::new(None),
            weak_self: weak.clone(),
        }))
    }
}
