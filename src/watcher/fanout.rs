use clack_host::prelude::PluginBundle;
use crossbeam_channel::{Receiver, Sender};
use std::sync::{Arc, Mutex};

// TODO: bikeshed
struct BundleFanoutInner {
    current_bundle: PluginBundle,
    senders: Vec<Sender<PluginBundle>>,
}

pub struct BundleProducer {
    inner: Arc<Mutex<BundleFanoutInner>>,
}

impl BundleProducer {
    pub fn produce(&mut self, new_bundle: &PluginBundle) {
        let mut inner = self.inner.lock().unwrap();
        inner.current_bundle = new_bundle.clone();

        // Remove disconnected senders
        inner
            .senders
            .retain_mut(|sender| sender.send(new_bundle.clone()).is_ok());
    }
}

pub struct BundleReceiverFactory {
    inner: Arc<Mutex<BundleFanoutInner>>,
}

impl BundleReceiverFactory {
    pub fn new_receiver(&self) -> BundleReceiver {
        let mut inner = self.inner.lock().unwrap();

        let current_bundle = inner.current_bundle.clone();
        let (sender, receiver) = crossbeam_channel::unbounded();
        inner.senders.push(sender);

        BundleReceiver {
            current_bundle,
            receiver,
        }
    }
}

pub struct BundleReceiver {
    current_bundle: PluginBundle,
    receiver: Receiver<PluginBundle>,
}

impl BundleReceiver {
    pub fn current_bundle(&self) -> &PluginBundle {
        &self.current_bundle
    }

    pub fn receive_new_bundle(&mut self) -> bool {
        let mut has_received = false;

        while let Ok(bundle) = self.receiver.try_recv() {
            self.current_bundle = bundle;
            has_received = true;
        }

        has_received
    }
}

pub fn new_bundle_fanout(initial_bundle: PluginBundle) -> (BundleProducer, BundleReceiverFactory) {
    let inner = Arc::new(Mutex::new(BundleFanoutInner {
        current_bundle: initial_bundle,
        senders: Vec::new(),
    }));

    (
        BundleProducer {
            inner: inner.clone(),
        },
        BundleReceiverFactory { inner },
    )
}
