// Forked and modified from: https://github.com/robbert-vdh/nih-plug/tree/master/plugins/examples/gain
use nih_plug::prelude::*;
use nih_plug_webview::*;
use serde::Deserialize;
use serde_json::json;
use std::sync::atomic::{Ordering, AtomicBool};
use std::sync::Arc;

struct Gain {
    params: Arc<GainParams>,
}

#[derive(Deserialize)]
#[serde(tag = "type")]
enum Action {
    Init,
    SetSize { width: u32, height: u32 },
    SetGain { value: f32 },
}

#[derive(Params)]
struct GainParams {
    #[id = "gain"]
    pub gain: FloatParam,
    gain_value_changed: Arc<AtomicBool>
}

impl Default for Gain {
    fn default() -> Self {
        Self {
            params: Arc::new(GainParams::default()),
        }
    }
}

impl Default for GainParams {
    fn default() -> Self {
        let gain_value_changed = Arc::new(AtomicBool::new(false));

        let v = gain_value_changed.clone();
        let param_callback = Arc::new(move |_: f32| {
            v.store(true, Ordering::Relaxed);
        });

        Self {
            gain: FloatParam::new(
                "Gain",
                util::db_to_gain(0.0),
                FloatRange::Skewed {
                    min: util::db_to_gain(-30.0),
                    max: util::db_to_gain(30.0),
                    factor: FloatRange::gain_skew_factor(-30.0, 30.0),
                },
            )
            .with_smoother(SmoothingStyle::Logarithmic(50.0))
            .with_unit(" dB")
            .with_value_to_string(formatters::v2s_f32_gain_to_db(2))
            .with_string_to_value(formatters::s2v_f32_gain_to_db())
            .with_callback(param_callback.clone()),
            gain_value_changed
        }
    }
}

impl Plugin for Gain {
    type BackgroundTask = ();

    const NAME: &'static str = "Gain";
    const VENDOR: &'static str = "Moist Plugins GmbH";
    const URL: &'static str = "https://youtu.be/dQw4w9WgXcQ";
    const EMAIL: &'static str = "info@example.com";

    const VERSION: &'static str = "0.0.1";

    const DEFAULT_INPUT_CHANNELS: u32 = 2;
    const DEFAULT_OUTPUT_CHANNELS: u32 = 2;

    const DEFAULT_AUX_INPUTS: Option<AuxiliaryIOConfig> = None;
    const DEFAULT_AUX_OUTPUTS: Option<AuxiliaryIOConfig> = None;

    const MIDI_INPUT: MidiConfig = MidiConfig::None;
    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn accepts_bus_config(&self, config: &BusConfig) -> bool {
        config.num_input_channels == config.num_output_channels && config.num_input_channels > 0
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        for channel_samples in buffer.iter_samples() {
            let gain = self.params.gain.smoothed.next();

            for sample in channel_samples {
                *sample *= gain;
            }
        }

        ProcessStatus::Normal
    }

    fn editor(&self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        let params = self.params.clone();
        let gain_value_changed = self.params.gain_value_changed.clone();
        let editor = WebViewEditorBuilder::new()
            .with_source(HTMLSource::String(include_str!("gui.html")))
            .with_size(200, 200)
            .with_background_color((150, 150, 150, 255))
            .with_developer_mode(true)
            .with_event_loop(move |ctx, setter| {
                for msg in ctx.consume_json() {
                    match msg {
                        WebviewMessage::JSON(msg) => {
                            if let Ok(action) = serde_json::from_value(msg) {
                                match action {
                                    Action::SetGain { value } => {
                                        setter.begin_set_parameter(&params.gain);
                                        setter.set_parameter_normalized(&params.gain, value);
                                        setter.end_set_parameter(&params.gain);
                                    }
                                    Action::SetSize { width, height } => {
                                        ctx.resize(width, height);
                                    }
                                    Action::Init => {
                                        let _ = ctx.send_json(json!({
                                            "type": "set_size",
                                            "width": ctx.width.load(Ordering::Relaxed),
                                            "height": ctx.height.load(Ordering::Relaxed)
                                        }));
                                    }
                                }
                            } else {
                                panic!("Invalid action received from web UI.")
                            }
                        }
                        WebviewMessage::FileDropped(path) => println!("File dropped: {:?}", path)
                    }
                }

                if gain_value_changed.swap(false, Ordering::Relaxed) {
                    let _ = ctx.send_json(json!({
                        "type": "param_change",
                        "param": "gain",
                        "value": params.gain.unmodulated_normalized_value(),
                        "text": params.gain.to_string()
                    }));
                }
            })
            .build();

        if let Ok(editor) = editor {
            Some(Box::new(editor))
        } else {
            panic!("Failed to construct editor.")
        }
    }

    fn deactivate(&mut self) {}
}

impl ClapPlugin for Gain {
    const CLAP_ID: &'static str = "com.moist-plugins-gmbh.gain";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("A smoothed gain parameter example plugin");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::AudioEffect,
        ClapFeature::Stereo,
        ClapFeature::Mono,
        ClapFeature::Utility,
    ];
}

impl Vst3Plugin for Gain {
    const VST3_CLASS_ID: [u8; 16] = *b"GainMoistestPlug";
    const VST3_CATEGORIES: &'static str = "Fx|Dynamics";
}

nih_export_clap!(Gain);
nih_export_vst3!(Gain);
