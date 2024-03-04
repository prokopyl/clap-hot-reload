use crate::wrapper::audio_processor::note_tracker::NoteTracker;
use crate::wrapper::*;
use clack_host::prelude::ProcessStatus;
use clack_plugin::host::HostAudioThreadHandle;
use clack_plugin::plugin::{AudioConfiguration, PluginAudioProcessor, PluginError};
use clack_plugin::prelude::{Audio, Events, Process};

mod cross_fader;
use cross_fader::*;
mod note_tracker;
mod output_buffers;

use output_buffers::*;

const CROSSFADE_TIME: f64 = 0.25;

pub struct WrapperPluginAudioProcessor<'a> {
    host: HostAudioThreadHandle<'a>,
    shared: &'a WrapperPluginShared<'a>,
    pub(crate) current_audio_processor: clack_host::process::PluginAudioProcessor<WrapperHost>,
    fade_out_audio_processor: Option<clack_host::process::PluginAudioProcessor<WrapperHost>>,
    channel: AudioProcessorChannel,
    input_event_buffer: EventBuffer,
    note_tracker: NoteTracker,
    cross_fader: CrossFader,
    output_buffers: OutputBuffers,
}

impl<'a> WrapperPluginAudioProcessor<'a> {
    fn swap_if_needed(&mut self, events: &Events) -> bool {
        // TODO: properly handle cookies
        if let Some(new_processor) = self.channel.check_for_new_processor() {
            println!("Audio processor received new update. Hot-swapping.");
            let old_processor =
                core::mem::replace(&mut self.current_audio_processor, new_processor.into());

            self.fade_out_audio_processor = Some(old_processor);

            // Recover notes
            // TODO: handle non-static
            self.input_event_buffer.clear();
            self.note_tracker
                .recover_all_current_notes(&mut self.input_event_buffer);

            // TODO: erf
            for e in events.input {
                let e: &UnknownEvent<'static> =
                    unsafe { &*(e as *const UnknownEvent<'_> as *const UnknownEvent<'static>) };
                self.input_event_buffer.push(e)
            }

            self.input_event_buffer.sort();

            println!("Note buffer : {:?}", &self.input_event_buffer);

            self.cross_fader.reset(); // Prepare for cross-fading
            true
        } else {
            false
        }
    }
}

impl<'a> PluginAudioProcessor<'a, WrapperPluginShared<'a>, WrapperPluginMainThread<'a>>
    for WrapperPluginAudioProcessor<'a>
{
    fn activate(
        host: HostAudioThreadHandle<'a>,
        main_thread: &mut WrapperPluginMainThread<'a>,
        shared: &'a WrapperPluginShared<'a>,
        audio_config: AudioConfiguration,
    ) -> Result<Self, PluginError> {
        main_thread.timers.init(&mut main_thread.host); // Do it now I guess... (TODO: fixme)

        // FIXME: Host should very much NOT be Copy or Clone
        let audio_processor =
            WrapperHost::activate_instance(&mut main_thread.plugin_instance, audio_config);

        // TODO: handle possible leftover channel
        let (main_thread_channel, audio_processor_channel) = MainThreadChannel::new_pair();
        main_thread.audio_processor_channel = Some(main_thread_channel);
        main_thread.current_audio_config = Some(audio_config);

        Ok(Self {
            host,
            shared,
            current_audio_processor: audio_processor.into(),
            fade_out_audio_processor: None,
            channel: audio_processor_channel,
            input_event_buffer: EventBuffer::with_capacity(64),
            note_tracker: NoteTracker::new(),
            cross_fader: CrossFader::new(audio_config.sample_rate, CROSSFADE_TIME),
            output_buffers: OutputBuffers::new_from_config(
                &main_thread.audio_ports_info,
                audio_config,
            ),
        })
    }

    fn process(
        &mut self,
        process: Process,
        mut audio: Audio,
        events: Events,
    ) -> Result<ProcessStatus, PluginError> {
        self.note_tracker.handle_note_events(events.input);

        // Hot swap! (but only if we're not already crossfading two instances)
        let swapped = if self.fade_out_audio_processor.is_some() {
            false
        } else {
            self.swap_if_needed(&events)
        };

        let status = if let Some(fade_out_audio_processor) = &mut self.fade_out_audio_processor {
            let audio_inputs = InputAudioBuffers::from_plugin_audio(&audio);

            let mut audio_outputs = self.output_buffers.output_buffers_for(true);

            // let in_events = buf.as_slice(); // TODO: add impl for Vec so it doesn't have to go through &slice.
            let in_events = self.input_event_buffer.as_input();
            let in_events = if swapped { &in_events } else { events.input };

            let main_status = self
                .current_audio_processor
                .ensure_processing_started()
                .map_err(|_| PluginError::Message("Not started"))?
                .process(
                    &audio_inputs,
                    &mut audio_outputs,
                    in_events,
                    events.output,
                    process.steady_time.map(|i| i as i64).unwrap_or(-1), // FIXME: i64 consistency stuff
                    None,
                    process.transport,
                )
                .map_err(|_| PluginError::OperationFailed)?;

            let mut audio_outputs = self.output_buffers.output_buffers_for(false);

            let fade_out_status = fade_out_audio_processor
                .ensure_processing_started()
                .map_err(|_| PluginError::Message("Not started"))?
                .process(
                    &audio_inputs,
                    &mut audio_outputs,
                    events.input,
                    &mut OutputEvents::void(), // Ignore all output events from the instance being faded out
                    process.steady_time.map(|i| i as i64).unwrap_or(-1), // FIXME: i64 consistency stuff
                    None,
                    process.transport,
                )
                .map_err(|_| PluginError::OperationFailed)?;

            self.output_buffers
                .output_crossfade(&mut self.cross_fader, &mut audio)?;

            if self.cross_fader.is_done() {
                // PANIC: we just checked above if the audio processor was there
                let old_processor = self.fade_out_audio_processor.take().unwrap();
                self.channel.send_for_disposal(old_processor.into_stopped()); // Byee

                // We don't care about if the older instance still wanted to process, we already
                // faded it away
                main_status
            } else {
                main_status.combined_with(fade_out_status)
            }
        } else {
            let (audio_inputs, mut audio_outputs) = AudioPorts::from_plugin_audio_mut(&mut audio);

            self.current_audio_processor
                .ensure_processing_started()
                .map_err(|_| PluginError::Message("Not started"))?
                .process(
                    &audio_inputs,
                    &mut audio_outputs,
                    events.input,
                    events.output,
                    process.steady_time.map(|i| i as i64).unwrap_or(-1), // FIXME: i64 consistency stuff
                    None,
                    process.transport,
                )
                .map_err(|_| PluginError::OperationFailed)?
        };

        Ok(status)
    }

    fn deactivate(self, main_thread: &mut WrapperPluginMainThread<'a>) {
        main_thread
            .plugin_instance
            .deactivate(self.current_audio_processor.into_stopped());

        if let Some(channel) = main_thread.audio_processor_channel.take() {
            channel.consume(self.channel)
        }
    }

    fn reset(&mut self) {
        self.note_tracker.reset();
        // FIXME: there's no reset on host
        todo!()
    }

    fn start_processing(&mut self) -> Result<(), PluginError> {
        if let Some(new_processor) = self.channel.move_to_latest_new_processor() {
            let new_processor = new_processor.start_processing().unwrap().into();
            let old_processor =
                core::mem::replace(&mut self.current_audio_processor, new_processor);

            self.channel.send_for_disposal(old_processor.into_stopped());
        }

        self.current_audio_processor.start_processing().unwrap(); // TODO: unwrap
        self.note_tracker.reset();
        Ok(())
    }

    fn stop_processing(&mut self) {
        self.current_audio_processor.stop_processing().unwrap(); // TODO: unwrap
        self.note_tracker.reset();

        if let Some(new_processor) = self.channel.move_to_latest_new_processor() {
            let new_processor = new_processor.into();
            let old_processor =
                core::mem::replace(&mut self.current_audio_processor, new_processor);

            self.channel.send_for_disposal(old_processor.into_stopped());
        }
    }
}
