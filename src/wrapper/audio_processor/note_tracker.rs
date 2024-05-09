use clack_host::events::event_types::NoteOnEvent;
use clack_host::events::spaces::CoreEventSpace;
use clack_host::prelude::{EventBuffer, InputEvents, Pckn};

#[derive(Debug)]
struct ActiveNote {
    port_index: u16,
    channel: u16,
    key: u16,
    note_id: u32,
    velocity: f64, // TODO: all the other note stuff
}

impl ActiveNote {
    fn to_note_event(&self) -> NoteOnEvent {
        NoteOnEvent::new(
            0,
            Pckn::new(self.port_index, self.channel, self.key, self.note_id),
            self.velocity,
        )
    }

    fn from_note_on_event(event: &NoteOnEvent) -> Option<Self> {
        Some(Self {
            // Some hosts won't populate note_id
            note_id: event.note_id().into_specific().unwrap_or(0),
            port_index: event.port_index().into_specific()?,
            channel: event.channel().into_specific()?,
            key: event.key().into_specific()?,
            velocity: event.velocity(),
        })
    }
}

impl PartialEq<Pckn> for ActiveNote {
    fn eq(&self, other: &Pckn) -> bool {
        other.note_id.matches(self.note_id)
            && other.port_index.matches(self.port_index)
            && other.channel.matches(self.channel)
            && other.key.matches(self.key)
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
                    if let Some(active_note) = ActiveNote::from_note_on_event(e) {
                        self.active_notes.push(active_note)
                    }
                }
                Some(CoreEventSpace::NoteOff(e)) => {
                    self.active_notes.retain(|note| *note != e.pckn())
                }
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
