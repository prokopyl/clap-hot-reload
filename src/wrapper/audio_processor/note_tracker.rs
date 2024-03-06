use clack_host::events::event_types::{NoteEvent, NoteOnEvent};
use clack_host::events::spaces::CoreEventSpace;
use clack_host::events::EventFlags;
use clack_host::prelude::{EventBuffer, EventHeader, InputEvents};

#[derive(Debug)]
struct ActiveNote {
    note_id: i32,
    port_index: i16,
    channel: i16,
    key: i16,
    velocity: f64, // TODO: all the other note stuff
}

impl ActiveNote {
    fn to_note_event(&self) -> NoteOnEvent {
        // TODO: are no flags always okay?
        NoteOnEvent(NoteEvent::new(
            EventHeader::new_core(0, EventFlags::empty()),
            self.note_id,
            self.port_index,
            self.key, // TODO: fix ordering
            self.channel,
            self.velocity,
        ))
    }

    fn from_note_on_event(event: &NoteOnEvent) -> Self {
        let event = &event.0;
        Self {
            note_id: event.note_id(),
            port_index: event.port_index(),
            channel: event.channel(),
            key: event.key(),
            velocity: event.velocity(),
        }
    }
}

impl<E> PartialEq<NoteEvent<E>> for ActiveNote {
    fn eq(&self, other: &NoteEvent<E>) -> bool {
        if other.note_id() >= 0 {
            return self.note_id == other.note_id();
        }

        self.port_index == other.port_index()
            && self.channel == other.channel()
            && self.key == other.key()
    }
}

pub struct NoteTracker {
    active_notes: Vec<ActiveNote>,
}

impl NoteTracker {
    pub fn new() -> Self {
        NoteTracker {
            active_notes: Vec::with_capacity(128),
        }
    }

    pub fn handle_note_events(&mut self, events: &InputEvents) {
        for event in events {
            match event.as_core_event() {
                // TODO: check duplicates?
                Some(CoreEventSpace::NoteOn(e)) => {
                    self.active_notes.push(ActiveNote::from_note_on_event(e))
                }
                Some(CoreEventSpace::NoteOff(e)) => self.active_notes.retain(|note| *note != e.0),
                _ => {}
            }
        }
    }

    pub fn recover_all_current_notes(&self, buffer: &mut EventBuffer) {
        println!(
            "Recovering {} active notes: {:?}",
            self.active_notes.len(),
            &self.active_notes
        );

        for note in &self.active_notes {
            buffer.push(&note.to_note_event())
        }
    }

    pub fn reset(&mut self) {
        self.active_notes.clear()
    }
}
