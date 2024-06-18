pub mod dto {
    use std::fmt;

    use serde::{ser::SerializeStruct, Deserialize, Serialize};
    use serde_json::Value;
    use serde_repr::{Deserialize_repr, Serialize_repr};

    const OLED_COLOR_BLACK: u16 = 0x0000;
    const OLED_COLOR_BLUE: u16 = 0x001F;
    const OLED_COLOR_RED: u16 = 0xF800;
    const OLED_COLOR_GREEN: u16 = 0x07E0;
    const OLED_COLOR_CYAN: u16 = 0x07FF;
    const OLED_COLOR_MAGENTA: u16 = 0xF81F;
    const OLED_COLOR_YELLOW: u16 = 0xFFE0;
    const OLED_COLOR_WARM: u16 = 0xFC00;
    const OLED_COLOR_WHITE: u16 = 0xFFFF;

    #[derive(Serialize)]
    pub struct GaugeTheme {
        ok_color: u16,
        low_color: u16,
        high_color: u16,
        alert_color: u16,
    }

    impl Default for GaugeTheme {
        fn default() -> GaugeTheme {
            GaugeTheme {
                ok_color: OLED_COLOR_WARM,
                low_color: OLED_COLOR_BLUE,
                high_color: OLED_COLOR_RED,
                alert_color: OLED_COLOR_RED,
            }
        }
    }

    #[derive(Serialize)]
    pub struct GaugeConfig {
        pub name: String,
        pub units: String,
        pub format: String,
        pub min: f32,
        pub max: f32,
        pub low_value: f32,
        pub high_value: f32,
    }

    #[derive(Serialize)]
    pub struct GaugeData {
        pub current_value: f32,
    }

    impl GaugeData {
        const OFFLINE_VALUE: f32 = f32::MAX;
    }

    type DisplayConfigurationGauges = Vec<GaugeConfig>;

    #[derive(Serialize)]
    pub struct DisplayConfiguration {
        pub gauges: DisplayConfigurationGauges,
    }

    #[derive(Serialize)]
    pub struct Configuration {
        pub theme: GaugeTheme,
        pub display1: DisplayConfiguration,
        pub display2: DisplayConfiguration,
        pub display3: DisplayConfiguration,
    }

    type DisplayDataGauges = Vec<GaugeData>;

    #[derive(Serialize)]
    pub struct DisplayData {
        pub gauges: DisplayDataGauges,
    }

    #[derive(Serialize)]
    pub struct Data {
        pub display1: DisplayData,
        pub display2: DisplayData,
        pub display3: DisplayData,
    }

    pub enum OutMessage {
        Configuration { message: Configuration },
        Data { message: Data },
    }

    impl serde::Serialize for OutMessage {
        fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
            // 3 is the number of fields in the struct.
            let mut state = s.serialize_struct("OutMessage", 2)?;
            match self {
                Self::Configuration { message } => {
                    state.serialize_field("type", &1);
                    state.serialize_field("message", &message);
                }
                Self::Data { message } => {
                    state.serialize_field("type", &2);
                    state.serialize_field("message", &message);
                }
            }

            return state.end();
        }
    }

    pub enum InMessage {
        NeedGaugeConfig {},
        NeedGaugeData {},
    }

    impl<'de> serde::Deserialize<'de> for InMessage {
        fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
            let value = Value::deserialize(d)?;

            Ok(match value.get("type").and_then(Value::as_u64).unwrap() {
                1 => InMessage::NeedGaugeConfig {},
                2 => InMessage::NeedGaugeData {},
                type_ => panic!("unsupported type {:?}", type_),
            })
        }
    }

    impl fmt::Display for InMessage {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            match self {
                Self::NeedGaugeConfig {} => {
                    return write!(f, "NeedGaugeConfig");
                }
                Self::NeedGaugeData {} => {
                    return write!(f, "NeedGaugeData");
                }
            }
        }
    }
}
