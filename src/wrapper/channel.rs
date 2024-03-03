use crate::wrapper::WrapperHost;
use clack_host::prelude::*;
use crossbeam_channel::{unbounded, Receiver, Sender};

// TODO: those channels are not realtime-safe
pub struct AudioProcessorChannel {
    sender: Sender<StoppedPluginAudioProcessor<WrapperHost>>,
    receiver: Receiver<StoppedPluginAudioProcessor<WrapperHost>>,
}

impl AudioProcessorChannel {
    pub fn check_for_new_processor(&mut self) -> Option<StoppedPluginAudioProcessor<WrapperHost>> {
        self.receiver.try_recv().ok()
    }

    pub fn move_to_latest_new_processor(
        &mut self,
    ) -> Option<StoppedPluginAudioProcessor<WrapperHost>> {
        let mut latest = None;

        for processor in self.receiver.try_iter() {
            if let Some(previous) = latest.take() {
                let _ = self.sender.send(previous);
            }

            latest = Some(processor)
        }

        latest
    }

    pub fn send_for_disposal(&mut self, processor: StoppedPluginAudioProcessor<WrapperHost>) {
        // If the channel is somehow disconnected, we just drop the processor, which will leak it if
        // the corresponding instance is gone already.

        let _ = self.sender.send(processor);
    }
}

pub struct MainThreadChannel {
    sender: Sender<StoppedPluginAudioProcessor<WrapperHost>>,
    receiver: Receiver<StoppedPluginAudioProcessor<WrapperHost>>,
    instances_awaiting_destruction: Vec<PluginInstance<WrapperHost>>,
}

impl MainThreadChannel {
    pub fn new_pair() -> (MainThreadChannel, AudioProcessorChannel) {
        let (sender_main_thread, receiver_audio_processor) = unbounded();
        let (sender_audio_processor, receiver_main_thread) = unbounded();

        (
            MainThreadChannel {
                sender: sender_main_thread,
                receiver: receiver_main_thread,
                instances_awaiting_destruction: Vec::new(),
            },
            AudioProcessorChannel {
                sender: sender_audio_processor,
                receiver: receiver_audio_processor,
            },
        )
    }

    pub fn send_new_audio_processor(
        &mut self,
        processor: StoppedPluginAudioProcessor<WrapperHost>,
        previous_instance: PluginInstance<WrapperHost>,
    ) -> Result<(), StoppedPluginAudioProcessor<WrapperHost>> {
        self.instances_awaiting_destruction.push(previous_instance);

        self.sender.send(processor).map_err(|e| e.0)
    }

    pub fn destroy_awaiting(&mut self) {
        for audio_processor in self.receiver.try_iter() {
            let Some(matching_index) = self
                .instances_awaiting_destruction
                .iter()
                .position(|i| audio_processor.matches(i))
            else {
                return;
            };

            let mut instance = self.instances_awaiting_destruction.remove(matching_index);
            instance.deactivate(audio_processor);
        }
    }

    pub fn consume(mut self, audio_processor_channel: AudioProcessorChannel) {
        for audio_processor in audio_processor_channel.receiver.try_iter() {
            let Some(matching_index) = self
                .instances_awaiting_destruction
                .iter()
                .position(|i| audio_processor.matches(i))
            else {
                return;
            };

            let mut instance = self.instances_awaiting_destruction.remove(matching_index);
            instance.deactivate(audio_processor);
        }

        self.destroy_awaiting();
    }
}
