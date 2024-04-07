//! Contains all types and implementations related to parameter management.

use crate::{PolySynthAudioProcessor, PolySynthPluginMainThread};
use clack_extensions::params::*;
use clack_extensions::state::PluginStateImpl;
use clack_plugin::events::spaces::CoreEventSpace;
use clack_plugin::prelude::*;
use clack_plugin::stream::{InputStream, OutputStream};
use std::ffi::CStr;
use std::fmt::Write as _;
use std::io::{Read, Write as _};
use std::sync::atomic::{AtomicU32, Ordering};

/// The default value of the volume parameter.
const DEFAULT_VOLUME: f32 = 0.2;

/// A struct that manages the parameters for our plugin.
///
/// For now, it only manages a single, `volume` parameter.
///
/// This struct will be used both by the [`PolySynthPluginMainThread`] (which the host will use
/// to query the value of our parameters), and by the [`PolySynthAudioProcessor`], which will
/// actually generate the audio samples.
pub struct PolySynthParams {
    /// The current value of the volume parameter.
    volume: AtomicF32,
}

impl PolySynthParams {
    /// Initializes the shared parameter value.
    pub fn new() -> Self {
        Self {
            volume: AtomicF32::new(DEFAULT_VOLUME),
        }
    }

    /// Returns the current volume.
    #[inline]
    pub fn get_volume(&self) -> f32 {
        self.volume.load(Ordering::SeqCst)
    }

    /// Sets a new value for the value parameter.
    /// The value is clamped, as it should only be in the `0..=1` range.
    #[inline]
    pub fn set_volume(&self, new_volume: f32) {
        let new_volume = new_volume.clamp(0., 1.);
        self.volume.store(new_volume, Ordering::SeqCst)
    }

    /// Handles incoming events.
    ///
    /// If the given event is a matching parameter change event, the volume parameter will be
    /// updated accordingly.
    pub fn handle_event(&self, event: &UnknownEvent) {
        if let Some(CoreEventSpace::ParamValue(event)) = event.as_core_event() {
            if event.param_id() == 1 {
                self.set_volume(event.value() as f32)
            }
        }
    }
}

pub struct PolySynthParamModulations {
    volume_mod: f32,
}
impl PolySynthParamModulations {
    pub fn new() -> Self {
        Self { volume_mod: 0.0 }
    }

    #[inline]
    pub fn volume(&self) -> f32 {
        self.volume_mod
    }

    pub fn handle_event(&mut self, event: &UnknownEvent) {
        if let Some(CoreEventSpace::ParamMod(event)) = event.as_core_event() {
            if event.param_id() == 1 {
                self.volume_mod = event.amount() as f32
            }
        }
    }
}

/// Implementation of the State extension.
///
/// Our state "serialization" is extremely simple and basic: we only have the value of the
/// volume parameter to store, so we just store its bytes (in little-endian) and call it a day.
impl<'a> PluginStateImpl for PolySynthPluginMainThread<'a> {
    fn save(&mut self, output: &mut OutputStream) -> Result<(), PluginError> {
        let volume_param = self.shared.params.get_volume();

        output.write_all(&volume_param.to_le_bytes())?;
        Ok(())
    }

    fn load(&mut self, input: &mut InputStream) -> Result<(), PluginError> {
        let mut buf = [0; 4];
        input.read_exact(&mut buf)?;
        let volume_value = f32::from_le_bytes(buf);
        self.shared.params.set_volume(volume_value);
        Ok(())
    }
}

impl<'a> PluginMainThreadParams for PolySynthPluginMainThread<'a> {
    fn count(&mut self) -> u32 {
        1
    }

    fn get_info(&mut self, param_index: u32, info: &mut ParamInfoWriter) {
        if param_index == 0 {
            info.set(&ParamInfo {
                id: 1.into(),
                flags: ParamInfoFlags::IS_AUTOMATABLE | ParamInfoFlags::IS_MODULATABLE,
                cookie: Default::default(),
                name: b"Volume",
                module: b"",
                min_value: 0.0,
                max_value: 1.0,
                default_value: DEFAULT_VOLUME as f64,
            })
        } else if param_index == 1 {
            info.set(&ParamInfo {
                id: 2.into(),
                flags: ParamInfoFlags::IS_AUTOMATABLE | ParamInfoFlags::IS_STEPPED,
                cookie: Default::default(),
                name: b"Foo param!",
                module: b"",
                min_value: 0.0,
                max_value: 1.0,
                default_value: DEFAULT_VOLUME as f64,
            })
        }
    }

    fn get_value(&mut self, param_id: u32) -> Option<f64> {
        match param_id {
            1 => Some(self.shared.params.get_volume() as f64),
            2 => Some(0.0),
            _ => None,
        }
    }

    fn value_to_text(
        &mut self,
        param_id: u32,
        value: f64,
        writer: &mut ParamDisplayWriter,
    ) -> std::fmt::Result {
        match param_id {
            1 => write!(writer, "{0:.2} %", value * 100.0),
            2 => write!(writer, "{} foo!", if value < 0.5 { "No" } else { "Yes" }),
            _ => Err(std::fmt::Error),
        }
    }

    fn text_to_value(&mut self, param_id: u32, text: &CStr) -> Option<f64> {
        let text = text.to_str().ok()?;

        if param_id == 1 {
            let text = text.strip_suffix('%').unwrap_or(text).trim();
            let percentage: f64 = text.parse().ok()?;

            Some(percentage / 100.0)
        } else {
            None
        }
    }

    fn flush(
        &mut self,
        input_parameter_changes: &InputEvents,
        _output_parameter_changes: &mut OutputEvents,
    ) {
        for event in input_parameter_changes {
            self.shared.params.handle_event(event)
        }
    }
}

impl<'a> PluginAudioProcessorParams for PolySynthAudioProcessor<'a> {
    fn flush(
        &mut self,
        input_parameter_changes: &InputEvents,
        _output_parameter_changes: &mut OutputEvents,
    ) {
        for event in input_parameter_changes {
            self.shared.params.handle_event(event)
        }
    }
}

/// A small helper to atomically load and store an `f32` value.
struct AtomicF32(AtomicU32);

impl AtomicF32 {
    /// Creates a new atomic `f32`.
    #[inline]
    fn new(value: f32) -> Self {
        Self(AtomicU32::new(f32_to_u32_bytes(value)))
    }

    /// Stores the given `value` using the given `order`ing.
    #[inline]
    fn store(&self, value: f32, order: Ordering) {
        self.0.store(f32_to_u32_bytes(value), order)
    }

    /// Loads the contained `value` using the given `order`ing.
    #[inline]
    fn load(&self, order: Ordering) -> f32 {
        f32_from_u32_bytes(self.0.load(order))
    }
}

/// Packs a `f32` into the bytes of an `u32`.
///
/// The resulting value is meaningless and should not be used directly,
/// except for unpacking with [`f32_from_u32_bytes`].
///
/// This is an internal helper used by [`AtomicF32`].
#[inline]
fn f32_to_u32_bytes(value: f32) -> u32 {
    u32::from_ne_bytes(value.to_ne_bytes())
}

/// The counterpart to [`f32_to_u32_bytes`].
#[inline]
fn f32_from_u32_bytes(bytes: u32) -> f32 {
    f32::from_ne_bytes(bytes.to_ne_bytes())
}
